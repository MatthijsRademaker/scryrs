//! Shared contracts for scryrs workspace crates.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Version for machine-facing contracts emitted by this scaffold.
pub const SCHEMA_VERSION: &str = "0.1.0";

/// Version for the hotspot report output contract, independent of
/// `SCHEMA_VERSION` which governs trace event wire format.
pub const HOTSPOT_SCHEMA_VERSION: &str = "1.0.0";

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HotspotCounts {
    pub eventType: HashMap<String, u32>,
    pub outcome: HashMap<String, u32>,
}

/// Ordered SQLite row ID references for all contributing events.
#[allow(non_snake_case)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
        // Can't deserialize because HotspotEntry only derives Serialize, not Deserialize.
        // But we can verify the JSON is valid.
        let _v: serde_json::Value = match serde_json::from_str(&json) {
            Ok(v) => v,
            Err(e) => panic!("valid JSON: {e}"),
        };
        assert!(json.contains("\"rank\":2"));
        assert!(json.contains("\"score\":1"));
        assert!(json.contains("\"rowIds\":[42]"));
    }
}
