//! Shared contracts for scryrs workspace crates.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Version for machine-facing contracts emitted by this scaffold.
pub const SCHEMA_VERSION: &str = "0.1.0";

/// Version for the hotspot report output contract, independent of
/// `SCHEMA_VERSION` which governs trace event wire format.
pub const HOTSPOT_SCHEMA_VERSION: &str = "1.0.0";

/// Version for the live hotspot query response, independent of
/// `SCHEMA_VERSION` (trace event wire format) and
/// `HOTSPOT_SCHEMA_VERSION` (local report output).
pub const LIVE_HOTSPOT_SCHEMA_VERSION: &str = "1.0.0";

/// Suite component metadata used by feature-gated crates and CLI output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeatureDescriptor {
    pub id: &'static str,
    pub title: &'static str,
    pub summary: &'static str,
}

/// Versioned trace event envelope used by all trace producers and consumers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceEvent {
    pub schema_version: String,
    pub timestamp: String,
    pub session_id: String,
    pub event_type: TraceEventType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    pub payload: TraceEventPayload,
    pub outcome: Outcome,
}

impl TraceEvent {
    /// Return a hotspot subject for subject-bearing events, or `None` for
    /// lifecycle events that have no hotspot subject.
    #[must_use]
    pub fn subject(&self) -> Option<&str> {
        match &self.payload {
            TraceEventPayload::SessionStart(_) | TraceEventPayload::SessionEnd(_) => None,
            TraceEventPayload::FileOpened(p) => Some(p.path.as_str()),
            TraceEventPayload::SearchRun(p) => Some(p.query.as_str()),
            TraceEventPayload::SymbolInspected(p) => Some(p.name.as_str()),
            TraceEventPayload::CommandExecuted(p) => Some(p.command.as_str()),
            TraceEventPayload::DocRetrieved(p) => Some(p.doc_ref.as_str()),
            TraceEventPayload::EditMade(p) => Some(p.target.as_str()),
            TraceEventPayload::FailedLookup(p) => Some(p.subject.as_str()),
        }
    }

    /// Return a short category tag for subject-bearing events, or `None`
    /// for lifecycle events. This is the `subject_kind` column used for
    /// indexed subject lookup in the datastore.
    #[must_use]
    pub fn subject_kind(&self) -> Option<&'static str> {
        match &self.payload {
            TraceEventPayload::SessionStart(_) | TraceEventPayload::SessionEnd(_) => None,
            TraceEventPayload::FileOpened(_) | TraceEventPayload::EditMade(_) => Some("file"),
            TraceEventPayload::SearchRun(_) => Some("search"),
            TraceEventPayload::SymbolInspected(_) | TraceEventPayload::FailedLookup(_) => {
                Some("symbol")
            }
            TraceEventPayload::CommandExecuted(_) => Some("command"),
            TraceEventPayload::DocRetrieved(_) => Some("document"),
        }
    }

    /// Extract the failure reason string for `Outcome::Failure` events,
    /// or `None` for success outcomes.
    #[must_use]
    pub fn failure_reason(&self) -> Option<&str> {
        match &self.outcome {
            Outcome::Success => None,
            Outcome::Failure { reason } => reason.as_deref(),
        }
    }

    /// Validate semantic invariants for an event that has passed
    /// structural deserialization. Returns `Ok(())` when both
    /// `schema_version` equals `SCHEMA_VERSION` and `event_type`
    /// matches the concrete `payload.type` tag.
    ///
    /// The caller should treat an `Err(reason)` as a rejection.
    #[must_use = "callers must check semantic invariants; discarded Result hides invalid events"]
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(format!(
                "schema_version mismatch: got '{}', expected '{}'",
                self.schema_version, SCHEMA_VERSION,
            ));
        }
        let expected_type = self.event_type.payload_type_str();
        let actual_type = self.payload.payload_type_str();
        if expected_type != actual_type {
            return Err(format!(
                "event_type/payload.type mismatch: event_type='{}', payload.type='{}'",
                expected_type, actual_type,
            ));
        }
        Ok(())
    }
}

/// Kind of trace event, mirroring the payload variant in use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraceEventType {
    SessionStart,
    SessionEnd,
    FileOpened,
    SearchRun,
    SymbolInspected,
    CommandExecuted,
    DocRetrieved,
    EditMade,
    FailedLookup,
}

impl TraceEventType {
    /// Return the expected `payload.type` string tag for this event type.
    #[must_use]
    pub fn payload_type_str(self) -> &'static str {
        match self {
            TraceEventType::SessionStart => "SessionStart",
            TraceEventType::SessionEnd => "SessionEnd",
            TraceEventType::FileOpened => "FileOpened",
            TraceEventType::SearchRun => "SearchRun",
            TraceEventType::SymbolInspected => "SymbolInspected",
            TraceEventType::CommandExecuted => "CommandExecuted",
            TraceEventType::DocRetrieved => "DocRetrieved",
            TraceEventType::EditMade => "EditMade",
            TraceEventType::FailedLookup => "FailedLookup",
        }
    }
}

/// Success or failure outcome carried on every trace event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "result")]
pub enum Outcome {
    Success,
    Failure {
        #[serde(skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
}

/// Payload families, one per activity kind. Self-describing on the wire via
/// the `type` tag so consumers can identify the concrete shape from JSON alone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TraceEventPayload {
    SessionStart(SessionStartPayload),
    SessionEnd(SessionEndPayload),
    FileOpened(FileOpenedPayload),
    SearchRun(SearchRunPayload),
    SymbolInspected(SymbolInspectedPayload),
    CommandExecuted(CommandExecutedPayload),
    DocRetrieved(DocRetrievedPayload),
    EditMade(EditMadePayload),
    FailedLookup(FailedLookupPayload),
}

impl TraceEventPayload {
    /// Return the `type` tag string for this payload variant.
    #[must_use]
    pub fn payload_type_str(&self) -> &'static str {
        match self {
            TraceEventPayload::SessionStart(_) => "SessionStart",
            TraceEventPayload::SessionEnd(_) => "SessionEnd",
            TraceEventPayload::FileOpened(_) => "FileOpened",
            TraceEventPayload::SearchRun(_) => "SearchRun",
            TraceEventPayload::SymbolInspected(_) => "SymbolInspected",
            TraceEventPayload::CommandExecuted(_) => "CommandExecuted",
            TraceEventPayload::DocRetrieved(_) => "DocRetrieved",
            TraceEventPayload::EditMade(_) => "EditMade",
            TraceEventPayload::FailedLookup(_) => "FailedLookup",
        }
    }
}

// --- Per-family payload types ---

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionStartPayload;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionEndPayload;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileOpenedPayload {
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchRunPayload {
    pub query: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolInspectedPayload {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandExecutedPayload {
    pub command: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocRetrievedPayload {
    pub doc_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditMadePayload {
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailedLookupPayload {
    pub subject: String,
}

// --- Adjacent types (unchanged from scaffold) ---

/// Ranked hotspot entry carrying full evidence from deterministic analysis.
#[allow(non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotspotEntry {
    pub rank: u32,
    pub subjectKind: String,
    pub subject: String,
    pub score: u32,
    pub counts: HotspotCounts,
    pub sessionCount: u32,
    pub firstSeen: String,
    pub lastSeen: String,
    pub evidence: HotspotEvidence,
}

/// Per-event-type and per-outcome breakdown counts for a hotspot entry.
#[allow(non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotspotCounts {
    pub eventType: HashMap<String, u32>,
    pub outcome: HashMap<String, u32>,
}

/// Ordered SQLite row ID references for all contributing events.
#[allow(non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotspotEvidence {
    pub rowIds: Vec<u64>,
}

/// Top-level hotspot report envelope emitted to stdout and `.scryrs/hotspots.json`.
#[allow(non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HotspotsReport {
    pub schemaVersion: String,
    pub command: String,
    pub repositoryPath: String,
    pub storePath: String,
    pub runMetadata: RunMetadata,
    pub generatedAt: String,
    pub entries: Vec<HotspotEntry>,
}

/// Deterministic metadata derived from the SQLite store state.
#[allow(non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RunMetadata {
    pub storeSchemaVersion: i64,
    pub analyzedEventCount: u64,
    pub analyzedSubjectCount: u64,
    pub firstEventId: u64,
    pub lastEventId: u64,
}

// --- Live hotspot accumulator and signal types ---

/// Deterministic signal persisted when a cumulative hotspot score crosses
/// the configured threshold. Each signal is append-only and stored
/// separately from accumulator rows.
#[allow(non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotspotSignal {
    /// Repository that produced the event.
    pub repositoryId: String,
    /// Subject kind tag ("file", "search", "symbol", "command", "document").
    pub subjectKind: String,
    /// Concrete subject string.
    pub subject: String,
    /// Cumulative score at the time of the crossing.
    pub score: u32,
    /// Score delta contributed by the triggering event.
    pub delta: u32,
    /// Window model tag — always `"cumulative"` for this foundation.
    pub window: String,
    /// Configured threshold that was crossed.
    pub threshold: u32,
    /// Ordered server_trace_events row IDs contributing to this signal.
    pub evidenceRowIds: Vec<u64>,
    /// RFC 3339 timestamp when the signal was created.
    pub createdAt: String,
}

// --- Live hotspot server contract types (Phase 4) ---

/// Versioned batch wrapper for trace events submitted to the live hotspot server.
/// Carries submission-context identity fields and an array of per-event items.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerIngestEnvelope {
    pub envelope_version: String,
    pub repository_id: String,
    pub workspace_id: String,
    pub agent_id: String,
    pub events: Vec<EnvelopeEvent>,
}

/// Per-event item within a `ServerIngestEnvelope`, pairing producer-scoped
/// identity and timing metadata with the inner `TraceEvent`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvelopeEvent {
    pub producer_event_id: String,
    pub client_timestamp: String,
    pub event: TraceEvent,
}

/// Acknowledgment status for a single event within a batch ingest response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventAckStatus {
    /// First submission of this event — a new record was created.
    Accepted,
    /// Duplicate submission — already processed, no new record created.
    Idempotent,
    /// Event failed server-side validation — see `EventAck.error_reason`.
    Rejected,
}

/// Per-event acknowledgment returned in `BatchIngestResponse`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventAck {
    /// Zeroth-indexed position of this item in the request `events` array.
    pub index: usize,
    /// Producer-scoped event identifier; `None` only when the request item
    /// could not supply one (malformed per-item decode).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub producer_event_id: Option<String>,
    pub status: EventAckStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_event_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_reason: Option<String>,
    pub received_at: String,
}

/// JSON acknowledgment returned by `POST /v1/trace-events/batch`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchIngestResponse {
    /// Count of events accepted (first-writer-wins) in this batch.
    pub accepted_count: u64,
    /// Count of duplicate (idempotent) events in this batch.
    pub duplicate_count: u64,
    /// Count of rejected events in this batch.
    pub rejected_count: u64,
    /// Count of accepted events in this batch (excluding idempotent).
    pub received_count: u64,
    pub events: Vec<EventAck>,
    pub received_at: String,
}

/// Live hotspot query response envelope for `GET /v1/repositories/{id}/hotspots`.
/// Separate from the local-only `HotspotsReport` — carries no filesystem-path fields.
#[allow(non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveHotspotsResponse {
    pub schemaVersion: String,
    pub repositoryId: String,
    pub cursor: String,
    pub generatedAt: String,
    pub entries: Vec<HotspotEntry>,
}

/// Knowledge graph node placeholder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphNode {
    pub id: String,
    pub title: String,
}

/// Reviewable knowledge proposal placeholder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeProposal {
    pub title: String,
    pub rationale: String,
}

/// Runtime routing hint placeholder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteHint {
    pub target: String,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn serialize_json<T: serde::Serialize>(value: &T) -> String {
        match serde_json::to_string(value) {
            Ok(json) => json,
            Err(error) => panic!("serialize: {error}"),
        }
    }

    fn deserialize_json<T: serde::de::DeserializeOwned>(json: &str) -> T {
        match serde_json::from_str(json) {
            Ok(value) => value,
            Err(error) => panic!("deserialize: {error}"),
        }
    }

    #[test]
    fn schema_version_starts_at_initial_scaffold_version() {
        assert_eq!(SCHEMA_VERSION, "0.1.0");
    }

    // --- Subject extraction ---

    #[test]
    fn lifecycle_events_return_no_subject() {
        let start = TraceEvent {
            schema_version: SCHEMA_VERSION.into(),
            timestamp: "2026-06-20T00:00:00Z".into(),
            session_id: "s1".into(),
            event_type: TraceEventType::SessionStart,
            tool_name: None,
            payload: TraceEventPayload::SessionStart(SessionStartPayload),
            outcome: Outcome::Success,
        };
        let end = TraceEvent {
            schema_version: SCHEMA_VERSION.into(),
            timestamp: "2026-06-20T00:00:00Z".into(),
            session_id: "s1".into(),
            event_type: TraceEventType::SessionEnd,
            tool_name: None,
            payload: TraceEventPayload::SessionEnd(SessionEndPayload),
            outcome: Outcome::Success,
        };
        assert!(start.subject().is_none());
        assert!(end.subject().is_none());
    }

    #[test]
    fn subject_bearing_events_return_the_correct_subject() {
        let events: Vec<TraceEvent> = vec![
            TraceEvent {
                schema_version: SCHEMA_VERSION.into(),
                timestamp: "2026-06-20T00:00:00Z".into(),
                session_id: "s1".into(),
                event_type: TraceEventType::FileOpened,
                tool_name: Some("read".into()),
                payload: TraceEventPayload::FileOpened(FileOpenedPayload {
                    path: "src/a.rs".into(),
                }),
                outcome: Outcome::Success,
            },
            TraceEvent {
                schema_version: SCHEMA_VERSION.into(),
                timestamp: "2026-06-20T00:00:00Z".into(),
                session_id: "s1".into(),
                event_type: TraceEventType::SearchRun,
                tool_name: Some("search".into()),
                payload: TraceEventPayload::SearchRun(SearchRunPayload {
                    query: "routing".into(),
                }),
                outcome: Outcome::Success,
            },
            TraceEvent {
                schema_version: SCHEMA_VERSION.into(),
                timestamp: "2026-06-20T00:00:00Z".into(),
                session_id: "s1".into(),
                event_type: TraceEventType::SymbolInspected,
                tool_name: Some("inspect".into()),
                payload: TraceEventPayload::SymbolInspected(SymbolInspectedPayload {
                    name: "MyStruct".into(),
                }),
                outcome: Outcome::Success,
            },
            TraceEvent {
                schema_version: SCHEMA_VERSION.into(),
                timestamp: "2026-06-20T00:00:00Z".into(),
                session_id: "s1".into(),
                event_type: TraceEventType::CommandExecuted,
                tool_name: Some("bash".into()),
                payload: TraceEventPayload::CommandExecuted(CommandExecutedPayload {
                    command: "cargo build".into(),
                }),
                outcome: Outcome::Success,
            },
            TraceEvent {
                schema_version: SCHEMA_VERSION.into(),
                timestamp: "2026-06-20T00:00:00Z".into(),
                session_id: "s1".into(),
                event_type: TraceEventType::DocRetrieved,
                tool_name: Some("read".into()),
                payload: TraceEventPayload::DocRetrieved(DocRetrievedPayload {
                    doc_ref: "api/foo.md".into(),
                }),
                outcome: Outcome::Success,
            },
            TraceEvent {
                schema_version: SCHEMA_VERSION.into(),
                timestamp: "2026-06-20T00:00:00Z".into(),
                session_id: "s1".into(),
                event_type: TraceEventType::EditMade,
                tool_name: Some("edit".into()),
                payload: TraceEventPayload::EditMade(EditMadePayload {
                    target: "src/b.rs".into(),
                }),
                outcome: Outcome::Success,
            },
            TraceEvent {
                schema_version: SCHEMA_VERSION.into(),
                timestamp: "2026-06-20T00:00:00Z".into(),
                session_id: "s1".into(),
                event_type: TraceEventType::FailedLookup,
                tool_name: Some("search".into()),
                payload: TraceEventPayload::FailedLookup(FailedLookupPayload {
                    subject: "missing_symbol".into(),
                }),
                outcome: Outcome::Failure {
                    reason: Some("not found".into()),
                },
            },
        ];

        let subjects: Vec<&str> = events.iter().filter_map(|e| e.subject()).collect();
        assert_eq!(
            subjects,
            vec![
                "src/a.rs",
                "routing",
                "MyStruct",
                "cargo build",
                "api/foo.md",
                "src/b.rs",
                "missing_symbol",
            ]
        );
    }

    // --- Serde round-trip tests (Task 3.1, 3.2, 3.3) ---

    fn round_trip(event: &TraceEvent) {
        let json = match serde_json::to_string_pretty(event) {
            Ok(v) => v,
            Err(e) => panic!("serialization failed: {e}"),
        };
        // Every event must carry the schema version in its JSON.
        assert!(
            json.contains(SCHEMA_VERSION),
            "serialized JSON must contain schema version '{}'",
            SCHEMA_VERSION
        );
        let reconstructed: TraceEvent = match serde_json::from_str(&json) {
            Ok(v) => v,
            Err(e) => panic!("deserialization failed: {e}"),
        };
        assert_eq!(
            &reconstructed, event,
            "round-tripped event must equal original"
        );
    }

    fn make_event(
        event_type: TraceEventType,
        tool_name: Option<&str>,
        payload: TraceEventPayload,
        outcome: Outcome,
    ) -> TraceEvent {
        TraceEvent {
            schema_version: SCHEMA_VERSION.into(),
            timestamp: "2026-06-20T12:00:00Z".into(),
            session_id: "test-session-1".into(),
            event_type,
            tool_name: tool_name.map(Into::into),
            payload,
            outcome,
        }
    }

    #[test]
    fn session_start_round_trips() {
        round_trip(&make_event(
            TraceEventType::SessionStart,
            None,
            TraceEventPayload::SessionStart(SessionStartPayload),
            Outcome::Success,
        ));
    }

    #[test]
    fn session_end_round_trips() {
        round_trip(&make_event(
            TraceEventType::SessionEnd,
            None,
            TraceEventPayload::SessionEnd(SessionEndPayload),
            Outcome::Success,
        ));
    }

    #[test]
    fn file_opened_round_trips() {
        round_trip(&make_event(
            TraceEventType::FileOpened,
            Some("read"),
            TraceEventPayload::FileOpened(FileOpenedPayload {
                path: "src/main.rs".into(),
            }),
            Outcome::Success,
        ));
    }

    #[test]
    fn search_run_round_trips() {
        round_trip(&make_event(
            TraceEventType::SearchRun,
            Some("search"),
            TraceEventPayload::SearchRun(SearchRunPayload {
                query: "error handling".into(),
            }),
            Outcome::Success,
        ));
    }

    #[test]
    fn symbol_inspected_round_trips() {
        round_trip(&make_event(
            TraceEventType::SymbolInspected,
            Some("inspect"),
            TraceEventPayload::SymbolInspected(SymbolInspectedPayload {
                name: "Dispatcher".into(),
            }),
            Outcome::Success,
        ));
    }

    #[test]
    fn command_executed_round_trips() {
        round_trip(&make_event(
            TraceEventType::CommandExecuted,
            Some("bash"),
            TraceEventPayload::CommandExecuted(CommandExecutedPayload {
                command: "cargo test".into(),
            }),
            Outcome::Success,
        ));
    }

    #[test]
    fn doc_retrieved_round_trips() {
        round_trip(&make_event(
            TraceEventType::DocRetrieved,
            Some("read"),
            TraceEventPayload::DocRetrieved(DocRetrievedPayload {
                doc_ref: "docs/api.md".into(),
            }),
            Outcome::Success,
        ));
    }

    #[test]
    fn edit_made_round_trips() {
        round_trip(&make_event(
            TraceEventType::EditMade,
            Some("edit"),
            TraceEventPayload::EditMade(EditMadePayload {
                target: "src/lib.rs".into(),
            }),
            Outcome::Success,
        ));
    }

    #[test]
    fn failed_lookup_with_failure_outcome_round_trips() {
        round_trip(&make_event(
            TraceEventType::FailedLookup,
            Some("search"),
            TraceEventPayload::FailedLookup(FailedLookupPayload {
                subject: "nonexistent_fn".into(),
            }),
            Outcome::Failure {
                reason: Some("symbol not found".into()),
            },
        ));
    }

    #[test]
    fn schema_version_present_in_every_serialized_event() {
        // Explicitly checks every event type carries schema_version in JSON.
        let all_events = vec![
            (
                TraceEventType::SessionStart,
                None,
                TraceEventPayload::SessionStart(SessionStartPayload),
                Outcome::Success,
            ),
            (
                TraceEventType::SessionEnd,
                None,
                TraceEventPayload::SessionEnd(SessionEndPayload),
                Outcome::Success,
            ),
            (
                TraceEventType::FileOpened,
                Some("read"),
                TraceEventPayload::FileOpened(FileOpenedPayload {
                    path: "a.rs".into(),
                }),
                Outcome::Success,
            ),
            (
                TraceEventType::SearchRun,
                Some("search"),
                TraceEventPayload::SearchRun(SearchRunPayload { query: "q".into() }),
                Outcome::Success,
            ),
            (
                TraceEventType::SymbolInspected,
                Some("inspect"),
                TraceEventPayload::SymbolInspected(SymbolInspectedPayload { name: "N".into() }),
                Outcome::Success,
            ),
            (
                TraceEventType::CommandExecuted,
                Some("bash"),
                TraceEventPayload::CommandExecuted(CommandExecutedPayload {
                    command: "c".into(),
                }),
                Outcome::Success,
            ),
            (
                TraceEventType::DocRetrieved,
                Some("read"),
                TraceEventPayload::DocRetrieved(DocRetrievedPayload {
                    doc_ref: "d".into(),
                }),
                Outcome::Success,
            ),
            (
                TraceEventType::EditMade,
                Some("edit"),
                TraceEventPayload::EditMade(EditMadePayload { target: "t".into() }),
                Outcome::Success,
            ),
            (
                TraceEventType::FailedLookup,
                Some("search"),
                TraceEventPayload::FailedLookup(FailedLookupPayload {
                    subject: "s".into(),
                }),
                Outcome::Failure {
                    reason: Some("r".into()),
                },
            ),
        ];

        for (event_type, tool_name, payload, outcome) in all_events {
            let event = make_event(event_type, tool_name, payload, outcome);
            let json = match serde_json::to_string(&event) {
                Ok(v) => v,
                Err(e) => panic!("serialize: {e}"),
            };
            assert!(
                json.contains(SCHEMA_VERSION),
                "event type {event_type:?} must carry schema_version in JSON"
            );
        }
    }

    #[test]
    fn payloads_are_self_describing_via_type_tag() {
        // Every serialized payload must include a "type" field that identifies
        // the concrete payload family from JSON alone.
        let event = make_event(
            TraceEventType::FileOpened,
            Some("read"),
            TraceEventPayload::FileOpened(FileOpenedPayload {
                path: "x.rs".into(),
            }),
            Outcome::Success,
        );
        let json = match serde_json::to_string(&event) {
            Ok(v) => v,
            Err(e) => panic!("serialize: {e}"),
        };
        assert!(
            json.contains("\"type\":\"FileOpened\""),
            "payload must include self-describing type tag"
        );
    }

    // --- TraceEvent::validate() semantic invariant tests ---

    #[test]
    fn validate_accepts_semantically_correct_event() {
        let event = make_event(
            TraceEventType::FileOpened,
            Some("read"),
            TraceEventPayload::FileOpened(FileOpenedPayload {
                path: "x.rs".into(),
            }),
            Outcome::Success,
        );
        assert!(event.validate().is_ok());
    }

    #[test]
    fn validate_rejects_wrong_schema_version() {
        let mut event = make_event(
            TraceEventType::DocRetrieved,
            Some("read"),
            TraceEventPayload::DocRetrieved(DocRetrievedPayload {
                doc_ref: "d.md".into(),
            }),
            Outcome::Success,
        );
        event.schema_version = "0.9.9".into();
        let validate_result = event.validate();
        assert!(
            validate_result.is_err(),
            "version mismatch should be rejected"
        );
        let err = match validate_result {
            Err(e) => e,
            Ok(_) => String::new(),
        };
        assert!(err.contains("schema_version mismatch"));
        assert!(err.contains("0.9.9"));
        assert!(err.contains(SCHEMA_VERSION));
    }

    #[test]
    fn validate_rejects_event_type_payload_mismatch() {
        let mut event = make_event(
            TraceEventType::FileOpened,
            Some("read"),
            // payload type is FileOpened, but we'll swap event_type
            TraceEventPayload::FileOpened(FileOpenedPayload {
                path: "x.rs".into(),
            }),
            Outcome::Success,
        );
        event.event_type = TraceEventType::DocRetrieved;
        let validate_result = event.validate();
        assert!(validate_result.is_err(), "type mismatch should be rejected");
        let err = match validate_result {
            Err(e) => e,
            Ok(_) => String::new(),
        };
        assert!(err.contains("event_type/payload.type mismatch"));
        assert!(err.contains("DocRetrieved"));
        assert!(err.contains("FileOpened"));
    }

    #[test]
    fn validate_all_lifecycle_events_are_accepted() {
        for (event_type, payload) in [
            (
                TraceEventType::SessionStart,
                TraceEventPayload::SessionStart(SessionStartPayload),
            ),
            (
                TraceEventType::SessionEnd,
                TraceEventPayload::SessionEnd(SessionEndPayload),
            ),
        ] {
            let event = make_event(event_type, None, payload, Outcome::Success);
            assert!(
                event.validate().is_ok(),
                "validate failed for {event_type:?}"
            );
        }
    }

    #[test]
    fn no_harness_specific_fields_in_json() {
        // Verify the wire format does not contain harness-specific identifiers.
        let event = make_event(
            TraceEventType::CommandExecuted,
            Some("bash"),
            TraceEventPayload::CommandExecuted(CommandExecutedPayload {
                command: "cargo build".into(),
            }),
            Outcome::Success,
        );
        let json = match serde_json::to_string(&event) {
            Ok(v) => v,
            Err(e) => panic!("serialize: {e}"),
        };
        // Harness-specific terms that must not appear.
        for forbidden in &["harness", "stdout", "stderr", "diff", "body", "content"] {
            assert!(
                !json.contains(forbidden),
                "JSON must not contain harness-specific field: '{forbidden}'"
            );
        }
    }

    // --- Hotspot types (Hotspot Foundation 02) ---

    #[test]
    fn hotspot_schema_version_is_independent() {
        assert_eq!(HOTSPOT_SCHEMA_VERSION, "1.0.0");
        assert_ne!(HOTSPOT_SCHEMA_VERSION, SCHEMA_VERSION);
    }

    #[test]
    fn hotspot_entry_serialization_round_trip() {
        let mut event_type_counts = HashMap::new();
        event_type_counts.insert("FileOpened".to_string(), 3u32);
        event_type_counts.insert("EditMade".to_string(), 2u32);

        let mut outcome_counts = HashMap::new();
        outcome_counts.insert("success".to_string(), 4u32);
        outcome_counts.insert("failure".to_string(), 1u32);

        let entry = HotspotEntry {
            rank: 1,
            subjectKind: "file".to_string(),
            subject: "src/main.rs".to_string(),
            score: 11,
            counts: HotspotCounts {
                eventType: event_type_counts,
                outcome: outcome_counts,
            },
            sessionCount: 2,
            firstSeen: "2026-06-21T09:00:00Z".to_string(),
            lastSeen: "2026-06-21T12:00:00Z".to_string(),
            evidence: HotspotEvidence {
                rowIds: vec![3, 7, 12, 45, 67],
            },
        };

        let json = match serde_json::to_string(&entry) {
            Ok(v) => v,
            Err(e) => panic!("serialize HotspotEntry: {e}"),
        };
        let parsed: serde_json::Value = match serde_json::from_str(&json) {
            Ok(v) => v,
            Err(e) => panic!("deserialize HotspotEntry JSON: {e}"),
        };

        assert_eq!(parsed["rank"], 1);
        assert_eq!(parsed["subjectKind"], "file");
        assert_eq!(parsed["subject"], "src/main.rs");
        assert_eq!(parsed["score"], 11);
        assert_eq!(parsed["sessionCount"], 2);
        assert_eq!(parsed["firstSeen"], "2026-06-21T09:00:00Z");
        assert_eq!(parsed["lastSeen"], "2026-06-21T12:00:00Z");
        assert_eq!(parsed["counts"]["eventType"]["FileOpened"], 3);
        assert_eq!(parsed["counts"]["eventType"]["EditMade"], 2);
        assert_eq!(parsed["counts"]["outcome"]["success"], 4);
        assert_eq!(parsed["counts"]["outcome"]["failure"], 1);
        assert_eq!(
            parsed["evidence"]["rowIds"]
                .as_array()
                .unwrap_or_else(|| panic!("rowIds not an array"))
                .len(),
            5
        );
    }

    #[test]
    fn hotspots_report_envelope_serialization_round_trip() {
        let mut event_type_counts = HashMap::new();
        event_type_counts.insert("SearchRun".to_string(), 1u32);
        let mut outcome_counts = HashMap::new();
        outcome_counts.insert("success".to_string(), 1u32);

        let entry = HotspotEntry {
            rank: 1,
            subjectKind: "search".to_string(),
            subject: "routing".to_string(),
            score: 2,
            counts: HotspotCounts {
                eventType: event_type_counts,
                outcome: outcome_counts,
            },
            sessionCount: 1,
            firstSeen: "2026-06-21T10:00:00Z".to_string(),
            lastSeen: "2026-06-21T10:00:00Z".to_string(),
            evidence: HotspotEvidence { rowIds: vec![5] },
        };

        let report = HotspotsReport {
            schemaVersion: HOTSPOT_SCHEMA_VERSION.into(),
            command: "hotspots".into(),
            repositoryPath: "/abs/path".into(),
            storePath: "/abs/path/.scryrs/scryrs.db".into(),
            runMetadata: RunMetadata {
                storeSchemaVersion: 1,
                analyzedEventCount: 1,
                analyzedSubjectCount: 1,
                firstEventId: 1,
                lastEventId: 1,
            },
            generatedAt: "2026-06-21T12:00:00Z".into(),
            entries: vec![entry],
        };

        let json = match serde_json::to_string(&report) {
            Ok(v) => v,
            Err(e) => panic!("serialize HotspotsReport: {e}"),
        };
        let parsed: serde_json::Value = match serde_json::from_str(&json) {
            Ok(v) => v,
            Err(e) => panic!("deserialize HotspotsReport JSON: {e}"),
        };

        assert_eq!(parsed["schemaVersion"], "1.0.0");
        assert_eq!(parsed["command"], "hotspots");
        assert_eq!(parsed["repositoryPath"], "/abs/path");
        assert_eq!(parsed["storePath"], "/abs/path/.scryrs/scryrs.db");
        assert_eq!(parsed["runMetadata"]["storeSchemaVersion"], 1);
        assert_eq!(parsed["runMetadata"]["analyzedEventCount"], 1);
        assert_eq!(parsed["runMetadata"]["analyzedSubjectCount"], 1);
        assert_eq!(parsed["generatedAt"], "2026-06-21T12:00:00Z");
        assert_eq!(
            parsed["entries"]
                .as_array()
                .unwrap_or_else(|| panic!("entries not an array"))
                .len(),
            1
        );
    }

    #[test]
    fn empty_entries_hotspots_report_serializes_correctly() {
        let report = HotspotsReport {
            schemaVersion: HOTSPOT_SCHEMA_VERSION.into(),
            command: "hotspots".into(),
            repositoryPath: "/abs/path".into(),
            storePath: "/abs/path/.scryrs/scryrs.db".into(),
            runMetadata: RunMetadata {
                storeSchemaVersion: 1,
                analyzedEventCount: 0,
                analyzedSubjectCount: 0,
                firstEventId: 0,
                lastEventId: 0,
            },
            generatedAt: "2026-06-21T12:00:00Z".into(),
            entries: vec![],
        };

        let json = match serde_json::to_string(&report) {
            Ok(v) => v,
            Err(e) => panic!("serialize empty report: {e}"),
        };
        let parsed: serde_json::Value = match serde_json::from_str(&json) {
            Ok(v) => v,
            Err(e) => panic!("deserialize empty report: {e}"),
        };
        assert_eq!(
            parsed["entries"]
                .as_array()
                .unwrap_or_else(|| panic!("entries not an array"))
                .len(),
            0
        );
        assert_eq!(parsed["runMetadata"]["analyzedEventCount"], 0);
    }

    #[test]
    fn hotspot_entry_round_trip_via_value() {
        // Full serialization round-trip through serde_json::Value.
        let original = HotspotEntry {
            rank: 2,
            subjectKind: "command".to_string(),
            subject: "cargo build".to_string(),
            score: 1,
            counts: HotspotCounts {
                eventType: {
                    let mut m = HashMap::new();
                    m.insert("CommandExecuted".to_string(), 1u32);
                    m
                },
                outcome: {
                    let mut m = HashMap::new();
                    m.insert("success".to_string(), 1u32);
                    m
                },
            },
            sessionCount: 1,
            firstSeen: "2026-06-21T09:00:00Z".to_string(),
            lastSeen: "2026-06-21T09:00:00Z".to_string(),
            evidence: HotspotEvidence { rowIds: vec![42] },
        };

        let json = match serde_json::to_string(&original) {
            Ok(v) => v,
            Err(e) => panic!("serialize: {e}"),
        };
        let deserialized: HotspotEntry = match serde_json::from_str(&json) {
            Ok(v) => v,
            Err(e) => panic!("deserialize: {e}"),
        };
        assert_eq!(deserialized, original);
        assert!(json.contains("\"rank\":2"));
        assert!(json.contains("\"score\":1"));
        assert!(json.contains("\"rowIds\":[42]"));
    }

    // --- Live hotspot server contract types ---

    fn make_sample_trace_event() -> TraceEvent {
        TraceEvent {
            schema_version: SCHEMA_VERSION.into(),
            timestamp: "2026-06-24T10:00:00Z".into(),
            session_id: "sess-1".into(),
            event_type: TraceEventType::FileOpened,
            tool_name: Some("read".into()),
            payload: TraceEventPayload::FileOpened(FileOpenedPayload {
                path: "src/main.rs".into(),
            }),
            outcome: Outcome::Success,
        }
    }

    #[test]
    fn server_ingest_envelope_round_trips() {
        let inner_event = make_sample_trace_event();
        let envelope = ServerIngestEnvelope {
            envelope_version: "1.0.0".into(),
            repository_id: "github.com/scryrs-project/scryrs".into(),
            workspace_id: "ws-abc123".into(),
            agent_id: "pi".into(),
            events: vec![EnvelopeEvent {
                producer_event_id: "evt-001".into(),
                client_timestamp: "2026-06-24T10:00:05Z".into(),
                event: inner_event.clone(),
            }],
        };
        let json = serialize_json(&envelope);
        let reconstructed: ServerIngestEnvelope = deserialize_json(&json);
        assert_eq!(reconstructed, envelope);
        assert_eq!(reconstructed.events[0].event, inner_event);
    }

    #[test]
    fn envelope_event_round_trips_independent_of_inner_trace_event() {
        let inner = make_sample_trace_event();
        let env_event = EnvelopeEvent {
            producer_event_id: "evt-002".into(),
            client_timestamp: "2026-06-24T10:01:00Z".into(),
            event: inner.clone(),
        };
        let json = serialize_json(&env_event);
        let reconstructed: EnvelopeEvent = deserialize_json(&json);
        assert_eq!(reconstructed, env_event);
        // Inner TraceEvent round-trips unchanged.
        assert_eq!(reconstructed.event, inner);
    }

    #[test]
    fn batch_ingest_response_round_trips() {
        let response = BatchIngestResponse {
            accepted_count: 2,
            duplicate_count: 1,
            rejected_count: 0,
            received_count: 2,
            events: vec![
                EventAck {
                    index: 0,
                    producer_event_id: Some("evt-001".into()),
                    status: EventAckStatus::Accepted,
                    server_event_id: Some("srv-42".into()),
                    error_reason: None,
                    received_at: "2026-06-24T10:00:07Z".into(),
                },
                EventAck {
                    index: 1,
                    producer_event_id: Some("evt-001".into()),
                    status: EventAckStatus::Idempotent,
                    server_event_id: None,
                    error_reason: None,
                    received_at: "2026-06-24T10:00:07Z".into(),
                },
                EventAck {
                    index: 2,
                    producer_event_id: Some("evt-003".into()),
                    status: EventAckStatus::Accepted,
                    server_event_id: Some("srv-43".into()),
                    error_reason: None,
                    received_at: "2026-06-24T10:00:07Z".into(),
                },
            ],
            received_at: "2026-06-24T10:00:07Z".into(),
        };
        let json = serialize_json(&response);
        let reconstructed: BatchIngestResponse = deserialize_json(&json);
        assert_eq!(reconstructed, response);
    }

    #[test]
    fn event_ack_status_serializes_as_snake_case() {
        let ack = EventAck {
            index: 0,
            producer_event_id: Some("evt-001".into()),
            status: EventAckStatus::Accepted,
            error_reason: None,
            server_event_id: None,
            received_at: "2026-06-24T10:00:07Z".into(),
        };
        let json = serialize_json(&ack);
        assert!(json.contains("\"status\":\"accepted\""));

        let ack = EventAck {
            index: 1,
            producer_event_id: Some("evt-001".into()),
            status: EventAckStatus::Idempotent,
            error_reason: None,
            server_event_id: None,
            received_at: "2026-06-24T10:00:07Z".into(),
        };
        let json = serialize_json(&ack);
        assert!(json.contains("\"status\":\"idempotent\""));

        let ack = EventAck {
            index: 2,
            producer_event_id: Some("evt-001".into()),
            status: EventAckStatus::Rejected,
            error_reason: Some("invalid TraceEvent: missing session_id".into()),
            server_event_id: None,
            received_at: "2026-06-24T10:00:07Z".into(),
        };
        let json = serialize_json(&ack);
        assert!(json.contains("\"status\":\"rejected\""));
    }

    #[test]
    fn event_ack_server_event_id_is_optional() {
        let ack_without = EventAck {
            index: 0,
            producer_event_id: Some("evt-001".into()),
            status: EventAckStatus::Accepted,
            error_reason: None,
            server_event_id: None,
            received_at: "2026-06-24T10:00:07Z".into(),
        };
        let json = serialize_json(&ack_without);
        assert!(!json.contains("server_event_id"));

        let ack_with = EventAck {
            index: 0,
            producer_event_id: Some("evt-001".into()),
            status: EventAckStatus::Accepted,
            error_reason: None,
            server_event_id: Some("srv-42".into()),
            received_at: "2026-06-24T10:00:07Z".into(),
        };
        let json = serialize_json(&ack_with);
        assert!(json.contains("server_event_id"));
    }

    #[test]
    fn live_hotspots_response_round_trips() {
        let mut event_type_counts = HashMap::new();
        event_type_counts.insert("FileOpened".to_string(), 3u32);
        let mut outcome_counts = HashMap::new();
        outcome_counts.insert("success".to_string(), 3u32);

        let entry = HotspotEntry {
            rank: 1,
            subjectKind: "file".to_string(),
            subject: "src/main.rs".to_string(),
            score: 15,
            counts: HotspotCounts {
                eventType: event_type_counts,
                outcome: outcome_counts,
            },
            sessionCount: 2,
            firstSeen: "2026-06-24T09:00:00Z".to_string(),
            lastSeen: "2026-06-24T12:00:00Z".to_string(),
            evidence: HotspotEvidence {
                rowIds: vec![10, 20, 30],
            },
        };

        let response = LiveHotspotsResponse {
            schemaVersion: LIVE_HOTSPOT_SCHEMA_VERSION.into(),
            repositoryId: "github.com/scryrs-project/scryrs".into(),
            cursor: "cursor-42".into(),
            generatedAt: "2026-06-24T12:00:00Z".into(),
            entries: vec![entry.clone()],
        };

        let json = serialize_json(&response);
        let reconstructed: LiveHotspotsResponse = deserialize_json(&json);
        assert_eq!(reconstructed, response);
    }

    #[test]
    fn live_hotspots_response_no_filesystem_fields() {
        let response = LiveHotspotsResponse {
            schemaVersion: LIVE_HOTSPOT_SCHEMA_VERSION.into(),
            repositoryId: "github.com/scryrs-project/scryrs".into(),
            cursor: "cursor-1".into(),
            generatedAt: "2026-06-24T12:00:00Z".into(),
            entries: vec![],
        };
        let json = serialize_json(&response);
        // Must not contain local-filesystem fields.
        assert!(!json.contains("repositoryPath"));
        assert!(!json.contains("storePath"));
    }

    #[test]
    fn live_hotspot_schema_version_is_independent() {
        assert_eq!(LIVE_HOTSPOT_SCHEMA_VERSION, "1.0.0");
        assert_ne!(LIVE_HOTSPOT_SCHEMA_VERSION, SCHEMA_VERSION);
        // Live version is the same value as HOTSPOT_SCHEMA_VERSION but semantically independent.
        assert_eq!(LIVE_HOTSPOT_SCHEMA_VERSION, HOTSPOT_SCHEMA_VERSION);
    }

    #[test]
    fn dedup_key_is_4_tuple_of_identity_fields() {
        // Verify the four identity fields that compose the deduplication key exist.
        let inner = make_sample_trace_event();
        let env1 = ServerIngestEnvelope {
            envelope_version: "1.0.0".into(),
            repository_id: "repo-a".into(),
            workspace_id: "ws-1".into(),
            agent_id: "pi".into(),
            events: vec![EnvelopeEvent {
                producer_event_id: "evt-001".into(),
                client_timestamp: "2026-06-24T10:00:00Z".into(),
                event: inner.clone(),
            }],
        };

        // Same 4-tuple — should be recognized as same key.
        let env2_same_key = ServerIngestEnvelope {
            envelope_version: "1.0.0".into(),
            repository_id: "repo-a".into(),
            workspace_id: "ws-1".into(),
            agent_id: "pi".into(),
            events: vec![EnvelopeEvent {
                producer_event_id: "evt-001".into(),
                client_timestamp: "2026-06-24T10:05:00Z".into(), // different timestamp does not change key
                event: inner.clone(),
            }],
        };

        // Different agent_id — different key.
        let env3_diff_agent = ServerIngestEnvelope {
            envelope_version: "1.0.0".into(),
            repository_id: "repo-a".into(),
            workspace_id: "ws-1".into(),
            agent_id: "claude-code".into(),
            events: vec![EnvelopeEvent {
                producer_event_id: "evt-001".into(),
                client_timestamp: "2026-06-24T10:00:00Z".into(),
                event: inner.clone(),
            }],
        };

        // Different repository_id — different key.
        let env4_diff_repo = ServerIngestEnvelope {
            envelope_version: "1.0.0".into(),
            repository_id: "repo-b".into(),
            workspace_id: "ws-1".into(),
            agent_id: "pi".into(),
            events: vec![EnvelopeEvent {
                producer_event_id: "evt-001".into(),
                client_timestamp: "2026-06-24T10:00:00Z".into(),
                event: inner.clone(),
            }],
        };

        // Same repository+workspace+agent, different producer_event_id — different key.
        let env5_diff_producer = ServerIngestEnvelope {
            envelope_version: "1.0.0".into(),
            repository_id: "repo-a".into(),
            workspace_id: "ws-1".into(),
            agent_id: "pi".into(),
            events: vec![EnvelopeEvent {
                producer_event_id: "evt-002".into(),
                client_timestamp: "2026-06-24T10:00:00Z".into(),
                event: inner.clone(),
            }],
        };

        // Matching 4-tuple should be equal across all identity fields.
        assert_eq!(
            env1.repository_id, env2_same_key.repository_id,
            "same repository_id for same key"
        );
        assert_eq!(
            env1.workspace_id, env2_same_key.workspace_id,
            "same workspace_id for same key"
        );
        assert_eq!(
            env1.agent_id, env2_same_key.agent_id,
            "same agent_id for same key"
        );
        assert_eq!(
            env1.events[0].producer_event_id, env2_same_key.events[0].producer_event_id,
            "same producer_event_id for same key"
        );

        // Different agent_id produces different key.
        assert_ne!(env1.agent_id, env3_diff_agent.agent_id);
        // Different repository_id produces different key.
        assert_ne!(env1.repository_id, env4_diff_repo.repository_id);
        // Different producer_event_id produces different key.
        assert_ne!(
            env1.events[0].producer_event_id,
            env5_diff_producer.events[0].producer_event_id
        );
    }

    #[test]
    fn server_ingest_envelope_with_empty_events_array() {
        let envelope = ServerIngestEnvelope {
            envelope_version: "1.0.0".into(),
            repository_id: "repo-a".into(),
            workspace_id: "ws-1".into(),
            agent_id: "pi".into(),
            events: vec![],
        };
        let json = serialize_json(&envelope);
        let reconstructed: ServerIngestEnvelope = deserialize_json(&json);
        assert_eq!(reconstructed, envelope);
        assert!(reconstructed.events.is_empty());
    }

    #[test]
    fn batch_ingest_response_empty_events() {
        let response = BatchIngestResponse {
            accepted_count: 0,
            duplicate_count: 0,
            rejected_count: 0,
            received_count: 0,
            events: vec![],
            received_at: "2026-06-24T10:00:07Z".into(),
        };
        let json = serialize_json(&response);
        let reconstructed: BatchIngestResponse = deserialize_json(&json);
        assert_eq!(reconstructed, response);
        assert!(reconstructed.events.is_empty());
    }

    #[test]
    fn live_hotspots_response_with_empty_entries() {
        let response = LiveHotspotsResponse {
            schemaVersion: LIVE_HOTSPOT_SCHEMA_VERSION.into(),
            repositoryId: "unknown-repo".into(),
            cursor: "cursor-0".into(),
            generatedAt: "2026-06-24T12:00:00Z".into(),
            entries: vec![],
        };
        let json = serialize_json(&response);
        let reconstructed: LiveHotspotsResponse = deserialize_json(&json);
        assert_eq!(reconstructed, response);
        assert!(reconstructed.entries.is_empty());
    }

    #[test]
    fn event_ack_rejected_variant_round_trips() {
        let ack = EventAck {
            index: 0,
            producer_event_id: Some("evt-099".into()),
            status: EventAckStatus::Rejected,
            server_event_id: None,
            error_reason: Some("invalid TraceEvent: missing session_id".into()),
            received_at: "2026-06-24T10:00:07Z".into(),
        };
        let json = serialize_json(&ack);
        let reconstructed: EventAck = deserialize_json(&json);
        assert_eq!(reconstructed, ack);
        assert!(json.contains("\"status\":\"rejected\""));
        assert!(json.contains("error_reason"));
    }

    #[test]
    fn event_ack_rejected_variant_error_reason_is_optional() {
        // error_reason should be omitted when None
        let ack_no_reason = EventAck {
            index: 0,
            producer_event_id: Some("evt-099".into()),
            status: EventAckStatus::Rejected,
            server_event_id: None,
            error_reason: None,
            received_at: "2026-06-24T10:00:07Z".into(),
        };
        let json = serialize_json(&ack_no_reason);
        assert!(!json.contains("error_reason"));

        // error_reason should be present when Some
        let ack_with_reason = EventAck {
            index: 1,
            producer_event_id: Some("evt-099".into()),
            status: EventAckStatus::Rejected,
            server_event_id: None,
            error_reason: Some("validation failed".into()),
            received_at: "2026-06-24T10:00:07Z".into(),
        };
        let json = serialize_json(&ack_with_reason);
        assert!(json.contains("error_reason"));
    }

    // --- Extended EventAck: index field ---

    #[test]
    fn event_ack_includes_request_index_in_serialization() {
        let ack = EventAck {
            index: 42,
            producer_event_id: Some("evt-042".into()),
            status: EventAckStatus::Accepted,
            server_event_id: None,
            error_reason: None,
            received_at: "2026-06-24T10:00:07Z".into(),
        };
        let json = serialize_json(&ack);
        assert!(
            json.contains("\"index\":42"),
            "serialized EventAck must include index field"
        );
    }

    #[test]
    fn event_ack_index_round_trips() {
        let ack = EventAck {
            index: 7,
            producer_event_id: Some("evt-007".into()),
            status: EventAckStatus::Accepted,
            server_event_id: None,
            error_reason: None,
            received_at: "2026-06-24T10:00:07Z".into(),
        };
        let json = serialize_json(&ack);
        let reconstructed: EventAck = deserialize_json(&json);
        assert_eq!(reconstructed.index, 7);
    }

    // --- Extended EventAck: optional producer_event_id ---

    #[test]
    fn event_ack_with_absent_producer_event_id_serializes_without_it() {
        let ack = EventAck {
            index: 0,
            producer_event_id: None,
            status: EventAckStatus::Rejected,
            server_event_id: None,
            error_reason: Some("malformed request item".into()),
            received_at: "2026-06-24T10:00:07Z".into(),
        };
        let json = serialize_json(&ack);
        // The field "producer_event_id" must not appear as a JSON key.
        // Parse to Value and check keys.
        let parsed: serde_json::Value =
            serde_json::from_str(&json).unwrap_or_else(|e| panic!("deserialize: {e}"));
        let obj = parsed
            .as_object()
            .unwrap_or_else(|| panic!("expected JSON object"));
        assert!(
            !obj.contains_key("producer_event_id"),
            "serialized JSON object must NOT have producer_event_id key when absent: {json}"
        );
    }

    #[test]
    fn event_ack_with_absent_producer_event_id_round_trips() {
        let ack = EventAck {
            index: 1,
            producer_event_id: None,
            status: EventAckStatus::Rejected,
            server_event_id: None,
            error_reason: Some("missing producer_event_id".into()),
            received_at: "2026-06-24T10:00:07Z".into(),
        };
        let json = serialize_json(&ack);
        let reconstructed: EventAck = deserialize_json(&json);
        assert_eq!(reconstructed, ack);
        assert!(reconstructed.producer_event_id.is_none());
    }

    // --- Extended BatchIngestResponse: accepted_count, rejected_count ---

    #[test]
    fn batch_ingest_response_includes_accepted_and_rejected_counts() {
        let response = BatchIngestResponse {
            accepted_count: 3,
            duplicate_count: 1,
            rejected_count: 2,
            received_count: 3,
            events: vec![
                EventAck {
                    index: 0,
                    producer_event_id: Some("evt-001".into()),
                    status: EventAckStatus::Accepted,
                    server_event_id: Some("srv-1".into()),
                    error_reason: None,
                    received_at: "2026-06-24T10:00:07Z".into(),
                },
                EventAck {
                    index: 1,
                    producer_event_id: None,
                    status: EventAckStatus::Rejected,
                    server_event_id: None,
                    error_reason: Some("invalid TraceEvent".into()),
                    received_at: "2026-06-24T10:00:07Z".into(),
                },
            ],
            received_at: "2026-06-24T10:00:07Z".into(),
        };
        let json = serialize_json(&response);
        assert!(
            json.contains("\"accepted_count\":3"),
            "serialized BatchIngestResponse must include accepted_count"
        );
        assert!(
            json.contains("\"duplicate_count\":1"),
            "serialized BatchIngestResponse must include duplicate_count"
        );
        assert!(
            json.contains("\"rejected_count\":2"),
            "serialized BatchIngestResponse must include rejected_count"
        );
        assert!(
            json.contains("\"received_count\":3"),
            "serialized BatchIngestResponse must include received_count"
        );
    }

    #[test]
    fn batch_ingest_response_counts_round_trip() {
        let response = BatchIngestResponse {
            accepted_count: 5,
            duplicate_count: 3,
            rejected_count: 1,
            received_count: 5,
            events: vec![],
            received_at: "2026-06-24T10:00:07Z".into(),
        };
        let json = serialize_json(&response);
        let reconstructed: BatchIngestResponse = deserialize_json(&json);
        assert_eq!(reconstructed, response);
        assert_eq!(reconstructed.accepted_count, 5);
        assert_eq!(reconstructed.rejected_count, 1);
    }

    // --- HotspotSignal round-trip tests ---

    #[test]
    fn hotspot_signal_round_trips() {
        let signal = HotspotSignal {
            repositoryId: "repo-a".into(),
            subjectKind: "file".into(),
            subject: "src/main.rs".into(),
            score: 15,
            delta: 3,
            window: "cumulative".into(),
            threshold: 10,
            evidenceRowIds: vec![1, 5, 9],
            createdAt: "2026-06-25T12:00:00Z".into(),
        };
        let json = serialize_json(&signal);
        let reconstructed: HotspotSignal = deserialize_json(&json);
        assert_eq!(reconstructed, signal);
        assert!(json.contains("\"subjectKind\":\"file\""));
        assert!(json.contains("\"window\":\"cumulative\""));
        assert!(json.contains("\"threshold\":10"));
        assert!(json.contains("\"evidenceRowIds\":[1,5,9]"));
    }

    #[test]
    fn hotspot_signal_empty_evidence_row_ids() {
        let signal = HotspotSignal {
            repositoryId: "repo-a".into(),
            subjectKind: "search".into(),
            subject: "routing".into(),
            score: 0,
            delta: 0,
            window: "cumulative".into(),
            threshold: 10,
            evidenceRowIds: vec![],
            createdAt: "2026-06-25T12:00:00Z".into(),
        };
        let json = serialize_json(&signal);
        let reconstructed: HotspotSignal = deserialize_json(&json);
        assert_eq!(reconstructed, signal);
    }

    #[test]
    fn hotspot_signal_delta_independent_of_score() {
        // Signal score and delta are separate fields: score is cumulative, delta is per-event.
        let signal = HotspotSignal {
            repositoryId: "repo-a".into(),
            subjectKind: "command".into(),
            subject: "cargo test".into(),
            score: 12,
            delta: 3,
            window: "cumulative".into(),
            threshold: 10,
            evidenceRowIds: vec![42],
            createdAt: "2026-06-25T12:00:00Z".into(),
        };
        assert_eq!(signal.score, 12);
        assert_eq!(signal.delta, 3);
    }
}
