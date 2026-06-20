//! JSONL ingestion: read line-by-line, validate as TraceEvent, accumulate rejections.

use std::io::{self, BufRead};

use scryrs_types::TraceEvent;

/// A single rejected input line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rejection {
    /// 1-based physical line number in the input.
    pub line: usize,
    /// Failing field/path when available from the deserializer.
    pub field: Option<String>,
    /// Human-readable reason for the rejection.
    pub reason: String,
}

/// Result of ingesting a JSONL input stream.
pub struct IngestionOutcome {
    /// Successfully parsed and validated TraceEvents.
    pub accepted: Vec<TraceEvent>,
    /// Rejected non-empty lines with diagnostics.
    pub rejected: Vec<Rejection>,
}

/// Read JSONL lines from `reader`, skip blank/whitespace-only lines,
/// deserialize each non-empty line as a [`TraceEvent`], and return
/// accepted events and structured rejections.
///
/// Ingestion continues after per-line validation failures — a malformed
/// line does not abort the stream.
pub fn ingest_jsonl(reader: impl BufRead) -> io::Result<IngestionOutcome> {
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();

    for (zero_based, line_result) in reader.lines().enumerate() {
        let line = line_result?;
        if line.trim().is_empty() {
            continue;
        }

        let line_1based = zero_based + 1;
        let mut json_de = serde_json::Deserializer::from_str(&line);
        match serde_path_to_error::deserialize(&mut json_de) {
            Ok(event) => accepted.push(event),
            Err(e) => {
                let field_path = e.path().to_string();
                // A path of "." or empty means the error occurred at the root
                // (e.g., malformed JSON syntax) — no specific field to report.
                let field = if field_path.is_empty() || field_path == "." {
                    None
                } else {
                    Some(field_path)
                };
                rejected.push(Rejection {
                    line: line_1based,
                    field,
                    reason: e.into_inner().to_string(),
                });
            }
        }
    }

    Ok(IngestionOutcome { accepted, rejected })
}

#[cfg(test)]
mod tests {
    use std::io;

    use scryrs_types::{
        DocRetrievedPayload, Outcome, SCHEMA_VERSION, TraceEventPayload, TraceEventType,
    };

    use super::*;

    fn make_valid_event_json(session_id: &str, doc_ref: &str) -> String {
        format!(
            concat!(
                r#"{{"schema_version":"{sv}","timestamp":"2026-06-20T00:00:00Z","session_id":"{sid}","event_type":"DocRetrieved","tool_name":"read","payload":{{"type":"DocRetrieved","doc_ref":"{dr}"}},"outcome":{{"result":"Success"}}}}"#
            ),
            sv = SCHEMA_VERSION,
            sid = session_id,
            dr = doc_ref,
        )
    }

    fn ingest(input: &str) -> IngestionOutcome {
        ingest_jsonl(io::BufReader::new(input.as_bytes()))
            .unwrap_or_else(|e| panic!("ingestion should succeed: {e}"))
    }

    // --- All-valid input ---

    #[test]
    fn all_valid_lines_accepted() {
        let input = format!(
            "{}\n{}\n",
            make_valid_event_json("s1", "doc/a.md"),
            make_valid_event_json("s2", "doc/b.md"),
        );
        let outcome = ingest(&input);

        assert_eq!(outcome.accepted.len(), 2);
        assert!(outcome.rejected.is_empty());

        assert_eq!(outcome.accepted[0].session_id, "s1");
        assert_eq!(outcome.accepted[1].session_id, "s2");
    }

    // --- Blank-line skipping ---

    #[test]
    fn blank_lines_are_skipped() {
        let input = format!(
            "\n{}\n\n{}\n  \n",
            make_valid_event_json("s1", "doc/a.md"),
            make_valid_event_json("s2", "doc/b.md"),
        );
        let outcome = ingest(&input);

        assert_eq!(outcome.accepted.len(), 2);
        // Blank lines do not contribute to accepted or rejected counts
        assert!(outcome.rejected.is_empty());
    }

    #[test]
    fn whitespace_only_lines_are_skipped() {
        let input = format!("   \t  \n{}\n", make_valid_event_json("s1", "doc/x.md"),);
        let outcome = ingest(&input);

        assert_eq!(outcome.accepted.len(), 1);
        assert!(outcome.rejected.is_empty());
    }

    // --- Partially-invalid input ---

    #[test]
    fn malformed_line_rejected_ingestion_continues() {
        let input = format!(
            "{}\nnot valid json\n{}\n",
            make_valid_event_json("s1", "doc/a.md"),
            make_valid_event_json("s2", "doc/b.md"),
        );
        let outcome = ingest(&input);

        assert_eq!(outcome.accepted.len(), 2);
        assert_eq!(outcome.rejected.len(), 1);
        assert_eq!(outcome.rejected[0].line, 2);
        assert!(!outcome.rejected[0].reason.is_empty());
    }

    #[test]
    fn schema_invalid_line_rejected() {
        // JSON is valid but missing required fields for TraceEvent
        let input = format!(
            "{}\n{{\"not\":\"an_event\"}}\n{}\n",
            make_valid_event_json("s1", "doc/a.md"),
            make_valid_event_json("s2", "doc/b.md"),
        );
        let outcome = ingest(&input);

        assert_eq!(outcome.accepted.len(), 2);
        assert_eq!(outcome.rejected.len(), 1);
        assert_eq!(outcome.rejected[0].line, 2);
        assert!(!outcome.rejected[0].reason.is_empty());
    }

    #[test]
    fn multiple_malformed_lines_all_rejected() {
        let outcome = ingest("bad1\nbad2\nbad3\n");

        assert!(outcome.accepted.is_empty());
        assert_eq!(outcome.rejected.len(), 3);
        assert_eq!(outcome.rejected[0].line, 1);
        assert_eq!(outcome.rejected[1].line, 2);
        assert_eq!(outcome.rejected[2].line, 3);
    }

    // --- Empty input ---

    #[test]
    fn empty_input_returns_empty() {
        let outcome = ingest("");

        assert!(outcome.accepted.is_empty());
        assert!(outcome.rejected.is_empty());
    }

    // --- Only blank lines ---

    #[test]
    fn only_blank_lines_returns_empty() {
        let outcome = ingest("\n\n  \n\t\n");

        assert!(outcome.accepted.is_empty());
        assert!(outcome.rejected.is_empty());
    }

    // --- Event content fidelity ---

    #[test]
    fn ingested_events_match_input_semantics() {
        let outcome = ingest(&make_valid_event_json("test-session", "doc/api.md"));

        assert_eq!(outcome.accepted.len(), 1);
        let event = &outcome.accepted[0];
        assert_eq!(event.session_id, "test-session");
        assert_eq!(event.event_type, TraceEventType::DocRetrieved);
        assert!(matches!(
            event.payload,
            TraceEventPayload::DocRetrieved(DocRetrievedPayload { ref doc_ref }) if doc_ref == "doc/api.md"
        ));
        assert_eq!(event.outcome, Outcome::Success);
    }

    // --- Field path extraction via serde_path_to_error ---

    #[test]
    fn missing_required_field_populates_field() {
        // Valid JSON but missing the required "timestamp" field.
        // serde_path_to_error reports missing struct fields at the root
        // level — the path is "." which we map to None.
        let input = format!(
            r#"{{"schema_version":"{}","session_id":"s1","event_type":"DocRetrieved","tool_name":"read","payload":{{"type":"DocRetrieved","doc_ref":"doc/a.md"}},"outcome":{{"result":"Success"}}}}"#,
            SCHEMA_VERSION,
        );
        let outcome = ingest(&input);

        assert_eq!(outcome.accepted.len(), 0);
        assert_eq!(outcome.rejected.len(), 1);
        assert_eq!(outcome.rejected[0].line, 1);
        // Missing required fields on the root struct report path ".",
        // which we normalize to None. The reason string still describes
        // the missing field.
        assert_eq!(outcome.rejected[0].field, None);
        assert!(outcome.rejected[0].reason.contains("timestamp"));
    }

    #[test]
    fn wrong_type_field_populates_field() {
        // Valid JSON but "outcome" has wrong type (string instead of object).
        let input = format!(
            r#"{{"schema_version":"{}","timestamp":"2026-06-20T00:00:00Z","session_id":"s1","event_type":"DocRetrieved","tool_name":"read","payload":{{"type":"DocRetrieved","doc_ref":"doc/a.md"}},"outcome":"wrong_type"}}"#,
            SCHEMA_VERSION,
        );
        let outcome = ingest(&input);

        assert_eq!(outcome.accepted.len(), 0);
        assert_eq!(outcome.rejected.len(), 1);
        assert_eq!(outcome.rejected[0].line, 1);
        assert!(
            outcome.rejected[0].field.is_some(),
            "field should be populated for type error"
        );
        let field = match outcome.rejected[0].field.as_deref() {
            Some(f) => f,
            None => panic!("field should be populated for type error"),
        };
        assert!(
            field.contains("outcome"),
            "field path should contain 'outcome', got: {field}"
        );
    }

    #[test]
    fn malformed_json_field_is_none() {
        // Not valid JSON at all — field path cannot be determined.
        let input = "not valid json\n";
        let outcome = ingest(input);

        assert_eq!(outcome.rejected.len(), 1);
        assert_eq!(
            outcome.rejected[0].field, None,
            "field should be None for malformed (non-JSON) input"
        );
    }

    #[test]
    fn nested_payload_field_populates_path() {
        // Valid JSON but payload missing required "doc_ref" for DocRetrieved.
        // serde_path_to_error reports the path up to the point where the
        // tagged enum variant fails — "payload" rather than "payload.doc_ref".
        let input = format!(
            r#"{{"schema_version":"{}","timestamp":"2026-06-20T00:00:00Z","session_id":"s1","event_type":"DocRetrieved","tool_name":"read","payload":{{"type":"DocRetrieved"}},"outcome":{{"result":"Success"}}}}"#,
            SCHEMA_VERSION,
        );
        let outcome = ingest(&input);

        assert_eq!(outcome.accepted.len(), 0);
        assert_eq!(outcome.rejected.len(), 1);
        assert_eq!(outcome.rejected[0].line, 1);
        let field = match outcome.rejected[0].field.as_deref() {
            Some(f) => f,
            None => panic!("field should be populated for nested payload error"),
        };
        // serde_path_to_error reports the path to the failing enum variant
        // key, which is "payload" — not the inner "payload.doc_ref".
        assert_eq!(field, "payload");
    }
}
