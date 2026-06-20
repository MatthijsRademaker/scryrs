//! Shared contracts for scryrs workspace crates.

use serde::{Deserialize, Serialize};

/// Version for machine-facing contracts emitted by this scaffold.
pub const SCHEMA_VERSION: &str = "0.1.0";

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

/// Ranked knowledge hotspot from deterministic analysis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hotspot {
    pub subject: String,
    pub score: u32,
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
}
