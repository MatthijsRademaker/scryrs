//! Standalone trace and hotspot foundation for scryrs.

pub mod ingestion;
pub mod store;

pub use ingestion::{IngestionOutcome, Rejection, ingest_jsonl};
pub use store::EventStore;

use scryrs_types::{FeatureDescriptor, Hotspot, TraceEvent};

pub fn descriptor() -> FeatureDescriptor {
    FeatureDescriptor {
        id: "core",
        title: "scryrs-core",
        summary: "standalone trace ingestion and hotspot detection foundation",
    }
}

/// Minimal deterministic hotspot scorer for scaffold validation.
///
/// Lifecycle events (SessionStart, SessionEnd) return no subject and are
/// ignored. Subject-bearing events contribute to hotspot scores.
pub fn score_events(events: &[TraceEvent]) -> Vec<Hotspot> {
    let mut hotspots = Vec::new();

    for event in events {
        let Some(subject) = event.subject() else {
            continue;
        };
        if let Some(index) = hotspots
            .iter()
            .position(|hotspot: &Hotspot| hotspot.subject.as_str() == subject)
        {
            hotspots[index].score += 1;
        } else {
            hotspots.push(Hotspot {
                subject: subject.to_string(),
                score: 1,
            });
        }
    }

    hotspots.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.subject.cmp(&right.subject))
    });

    hotspots
}

#[cfg(test)]
mod tests {
    use scryrs_types::{
        CommandExecutedPayload, DocRetrievedPayload, EditMadePayload, FileOpenedPayload, Outcome,
        SCHEMA_VERSION, SearchRunPayload, SessionEndPayload, SessionStartPayload, TraceEvent,
        TraceEventPayload, TraceEventType,
    };

    use super::*;

    fn make_event(
        event_type: TraceEventType,
        tool_name: Option<&str>,
        payload: TraceEventPayload,
        outcome: Outcome,
    ) -> TraceEvent {
        TraceEvent {
            schema_version: SCHEMA_VERSION.into(),
            timestamp: "2026-06-20T12:00:00Z".into(),
            session_id: "test".into(),
            event_type,
            tool_name: tool_name.map(Into::into),
            payload,
            outcome,
        }
    }

    #[test]
    fn scores_repeated_subjects_first() {
        let events = vec![
            make_event(
                TraceEventType::FileOpened,
                Some("read"),
                TraceEventPayload::FileOpened(FileOpenedPayload {
                    path: "src/a.rs".into(),
                }),
                Outcome::Success,
            ),
            make_event(
                TraceEventType::SearchRun,
                Some("search"),
                TraceEventPayload::SearchRun(SearchRunPayload {
                    query: "routing".into(),
                }),
                Outcome::Success,
            ),
            make_event(
                TraceEventType::FileOpened,
                Some("read"),
                TraceEventPayload::FileOpened(FileOpenedPayload {
                    path: "src/a.rs".into(),
                }),
                Outcome::Success,
            ),
        ];

        let hotspots = score_events(&events);

        assert_eq!(hotspots[0].subject, "src/a.rs");
        assert_eq!(hotspots[0].score, 2);
    }

    #[test]
    fn lifecycle_events_are_ignored() {
        let events = vec![
            make_event(
                TraceEventType::SessionStart,
                None,
                TraceEventPayload::SessionStart(SessionStartPayload),
                Outcome::Success,
            ),
            make_event(
                TraceEventType::FileOpened,
                Some("read"),
                TraceEventPayload::FileOpened(FileOpenedPayload {
                    path: "src/b.rs".into(),
                }),
                Outcome::Success,
            ),
            make_event(
                TraceEventType::SessionEnd,
                None,
                TraceEventPayload::SessionEnd(SessionEndPayload),
                Outcome::Success,
            ),
        ];

        let hotspots = score_events(&events);

        // Only the FileOpened event should contribute.
        assert_eq!(hotspots.len(), 1);
        assert_eq!(hotspots[0].subject, "src/b.rs");
        assert_eq!(hotspots[0].score, 1);
    }

    #[test]
    fn deterministic_tie_break_on_equal_scores() {
        let events = vec![
            make_event(
                TraceEventType::CommandExecuted,
                Some("bash"),
                TraceEventPayload::CommandExecuted(CommandExecutedPayload {
                    command: "cargo build".into(),
                }),
                Outcome::Success,
            ),
            make_event(
                TraceEventType::DocRetrieved,
                Some("read"),
                TraceEventPayload::DocRetrieved(DocRetrievedPayload {
                    doc_ref: "api.md".into(),
                }),
                Outcome::Success,
            ),
        ];

        let hotspots = score_events(&events);

        // Both score 1. Lexicographic tie-break: "api.md" < "cargo build".
        assert_eq!(hotspots.len(), 2);
        assert_eq!(hotspots[0].subject, "api.md");
        assert_eq!(hotspots[0].score, 1);
        assert_eq!(hotspots[1].subject, "cargo build");
        assert_eq!(hotspots[1].score, 1);
    }

    #[test]
    fn empty_input_returns_empty() {
        let hotspots = score_events(&[]);
        assert!(hotspots.is_empty());
    }

    #[test]
    fn only_lifecycle_events_return_empty() {
        let events = vec![
            make_event(
                TraceEventType::SessionStart,
                None,
                TraceEventPayload::SessionStart(SessionStartPayload),
                Outcome::Success,
            ),
            make_event(
                TraceEventType::SessionEnd,
                None,
                TraceEventPayload::SessionEnd(SessionEndPayload),
                Outcome::Success,
            ),
        ];

        let hotspots = score_events(&events);
        assert!(hotspots.is_empty());
    }

    #[test]
    fn failure_outcome_does_not_affect_scoring() {
        // Outcome does not influence hotspot scoring — subject extraction is
        // independent of success/failure.
        let events = vec![make_event(
            TraceEventType::EditMade,
            Some("edit"),
            TraceEventPayload::EditMade(EditMadePayload {
                target: "src/x.rs".into(),
            }),
            Outcome::Failure {
                reason: Some("write error".into()),
            },
        )];

        let hotspots = score_events(&events);

        assert_eq!(hotspots.len(), 1);
        assert_eq!(hotspots[0].subject, "src/x.rs");
        assert_eq!(hotspots[0].score, 1);
    }
}
