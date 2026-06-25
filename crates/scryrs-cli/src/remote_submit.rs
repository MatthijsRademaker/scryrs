//! Remote ingest submission layer.
//!
//! Provides a blocking HTTP transport for `POST /v1/trace-events/batch` backed by
//! `ureq`, plus a testable `RemoteSubmitter` trait. Also owns deterministic
//! `producer_event_id` derivation (SHA-256 hex of canonical `TraceEvent` JSON,
//! `:`, and 1-based physical line number).

use sha2::{Digest, Sha256};
use std::time::Duration;

use scryrs_types::{BatchIngestResponse, EnvelopeEvent, ServerIngestEnvelope, TraceEvent};

/// Build a `ServerIngestEnvelope` version `"1.0.0"` from accepted events and
/// resolved remote identity.
pub(crate) fn build_envelope(
    accepted: &[(usize, TraceEvent)],
    repository_id: &str,
    workspace_id: &str,
    agent_id: &str,
) -> ServerIngestEnvelope {
    let events: Vec<EnvelopeEvent> = accepted
        .iter()
        .map(|(line, event)| {
            let producer_event_id = derive_producer_event_id(event, *line);
            EnvelopeEvent {
                producer_event_id,
                client_timestamp: event.timestamp.clone(),
                event: event.clone(),
            }
        })
        .collect();

    ServerIngestEnvelope {
        envelope_version: "1.0.0".into(),
        repository_id: repository_id.into(),
        workspace_id: workspace_id.into(),
        agent_id: agent_id.into(),
        events,
    }
}

/// Derive a deterministic `producer_event_id` from the canonical serialized
/// `TraceEvent` plus 1-based physical line number.
///
/// Format: `SHA256(canonical_json):<line_number>`
pub(crate) fn derive_producer_event_id(event: &TraceEvent, line: usize) -> String {
    // Canonical JSON: serialize with sorted keys for determinism.
    // serde_json::to_string sorts keys by default for structs.
    let canonical =
        serde_json::to_string(event).unwrap_or_else(|_| format!("<serialize-error-line-{line}>"));
    let hash = hex::encode(Sha256::digest(canonical.as_bytes()));
    format!("{hash}:{line}")
}

/// Transport abstraction for remote submission, enabling deterministic tests
/// without real network calls.
pub(crate) trait RemoteSubmitter {
    /// Submit a `ServerIngestEnvelope` to the configured ingest URL and return
    /// the server response or an error.
    fn submit(
        &self,
        ingest_url: &str,
        envelope: &ServerIngestEnvelope,
        timeout_ms: u64,
    ) -> Result<BatchIngestResponse, SubmitError>;
}

/// Concrete `ureq`-backed submitter for production use.
pub(crate) struct UreqSubmitter;

/// Errors that can occur during remote submission.
#[derive(Debug)]
pub(crate) enum SubmitError {
    /// The server timed out.
    Timeout,
    /// Connection refused or network unreachable.
    Connection(String),
    /// Non-2xx HTTP status with response body excerpt.
    HttpStatus { status: u16, body: String },
    /// The response body could not be parsed as `BatchIngestResponse`.
    MalformedResponse(String),
    /// Serialization of the request body failed.
    Serialization(String),
}

impl std::fmt::Display for SubmitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubmitError::Timeout => write!(f, "remote ingest timed out"),
            SubmitError::Connection(e) => write!(f, "cannot reach ingest server: {e}"),
            SubmitError::HttpStatus { status, body } => {
                write!(
                    f,
                    "ingest server returned HTTP {status}: {}",
                    body.lines().next().unwrap_or("(empty body)")
                )
            }
            SubmitError::MalformedResponse(e) => {
                write!(f, "malformed ingest server response: {e}")
            }
            SubmitError::Serialization(e) => {
                write!(f, "cannot serialize request body: {e}")
            }
        }
    }
}

impl RemoteSubmitter for UreqSubmitter {
    fn submit(
        &self,
        ingest_url: &str,
        envelope: &ServerIngestEnvelope,
        timeout_ms: u64,
    ) -> Result<BatchIngestResponse, SubmitError> {
        // Build the full URL with path.
        let url = if ingest_url.ends_with('/') {
            format!("{ingest_url}v1/trace-events/batch")
        } else {
            format!("{ingest_url}/v1/trace-events/batch")
        };

        let body =
            serde_json::to_vec(envelope).map_err(|e| SubmitError::Serialization(e.to_string()))?;

        let response = ureq::post(&url)
            .set("Content-Type", "application/json")
            .set("User-Agent", "scryrs-cli/0.1.0")
            .timeout(Duration::from_millis(timeout_ms))
            .send_bytes(&body);

        match response {
            Ok(resp) => {
                let status = resp.status();
                let body_text = resp
                    .into_string()
                    .unwrap_or_else(|e| format!("<read error: {e}>"));

                if !(200..300).contains(&status) {
                    return Err(SubmitError::HttpStatus {
                        status,
                        body: body_text,
                    });
                }

                serde_json::from_str::<BatchIngestResponse>(&body_text)
                    .map_err(|e| SubmitError::MalformedResponse(format!("{e}: {body_text}")))
            }
            Err(ureq::Error::Status(code, resp)) => {
                let body_text = resp
                    .into_string()
                    .unwrap_or_else(|e| format!("<read error: {e}>"));
                Err(SubmitError::HttpStatus {
                    status: code,
                    body: body_text,
                })
            }
            Err(ureq::Error::Transport(e)) => {
                let msg = e.to_string();
                if msg.contains("timed out") || msg.contains("Timeout") {
                    Err(SubmitError::Timeout)
                } else {
                    Err(SubmitError::Connection(msg))
                }
            }
        }
    }
}

/// Re-export hex for use in producer_event_id derivation.
mod hex {
    pub(crate) fn encode(bytes: impl AsRef<[u8]>) -> String {
        let bytes = bytes.as_ref();
        let mut s = String::with_capacity(bytes.len() * 2);
        for b in bytes {
            use std::fmt::Write;
            let _ = write!(s, "{b:02x}");
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scryrs_types::{DocRetrievedPayload, Outcome, TraceEventPayload, TraceEventType};

    fn make_test_event(session_id: &str, doc_ref: &str) -> TraceEvent {
        TraceEvent {
            schema_version: scryrs_types::SCHEMA_VERSION.into(),
            timestamp: "2026-06-20T12:00:00Z".into(),
            session_id: session_id.into(),
            event_type: TraceEventType::DocRetrieved,
            tool_name: Some("read".into()),
            payload: TraceEventPayload::DocRetrieved(DocRetrievedPayload {
                doc_ref: doc_ref.into(),
            }),
            outcome: Outcome::Success,
        }
    }

    #[test]
    fn producer_event_id_is_deterministic() {
        let event = make_test_event("s1", "doc/a.md");
        let id1 = derive_producer_event_id(&event, 3);
        let id2 = derive_producer_event_id(&event, 3);
        assert_eq!(id1, id2, "same event + line => same id");
    }

    #[test]
    fn producer_event_id_differs_by_line() {
        let event = make_test_event("s1", "doc/a.md");
        let id3 = derive_producer_event_id(&event, 3);
        let id5 = derive_producer_event_id(&event, 5);
        assert_ne!(id3, id5, "different line => different id");
    }

    #[test]
    fn producer_event_id_differs_by_content() {
        let e1 = make_test_event("s1", "doc/a.md");
        let e2 = make_test_event("s2", "doc/a.md");
        let id1 = derive_producer_event_id(&e1, 1);
        let id2 = derive_producer_event_id(&e2, 1);
        assert_ne!(id1, id2, "different session_id => different id");
    }

    #[test]
    fn producer_event_id_format_is_sha256_colon_line() {
        let event = make_test_event("s1", "doc/a.md");
        let id = derive_producer_event_id(&event, 42);
        // Must be 64 hex chars + ":" + line number
        let colon_pos = id
            .rfind(':')
            .unwrap_or_else(|| panic!("missing colon in {id}"));
        assert_eq!(&id[colon_pos + 1..], "42", "line number suffix");
        assert_eq!(
            id[..colon_pos].len(),
            64,
            "sha256 hex must be 64 chars, got {}",
            id[..colon_pos].len()
        );
        // All chars before colon must be lowercase hex.
        assert!(
            id[..colon_pos].chars().all(|c| c.is_ascii_hexdigit()),
            "digest must be hex, got: {}",
            &id[..colon_pos]
        );
    }

    #[test]
    fn build_envelope_includes_all_accepted_events() {
        let e1 = make_test_event("s1", "doc/a.md");
        let e2 = make_test_event("s2", "doc/b.md");
        let accepted = vec![(1, e1.clone()), (3, e2.clone())];

        let envelope = build_envelope(&accepted, "repo-1", "ws-1", "agent-pi");

        assert_eq!(envelope.envelope_version, "1.0.0");
        assert_eq!(envelope.repository_id, "repo-1");
        assert_eq!(envelope.workspace_id, "ws-1");
        assert_eq!(envelope.agent_id, "agent-pi");
        assert_eq!(envelope.events.len(), 2);

        // Verify producer_event_ids are deterministic.
        assert_eq!(
            envelope.events[0].producer_event_id,
            derive_producer_event_id(&e1, 1)
        );
        assert_eq!(
            envelope.events[1].producer_event_id,
            derive_producer_event_id(&e2, 3)
        );

        // Verify timestamps are from the event.
        assert_eq!(envelope.events[0].client_timestamp, "2026-06-20T12:00:00Z");
    }
}
