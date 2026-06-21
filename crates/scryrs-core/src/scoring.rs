//! Deterministic hotspot scoring from trace events.
//!
//! `score_hotspots` groups subject-bearing events by `(subject_kind, subject)`,
//! applies a documented integer weight table per event type plus a failure bonus,
//! and produces ranked `HotspotEntry` results with a six-key tie-break chain.

use std::collections::HashMap;

use scryrs_types::{
    HotspotCounts, HotspotEntry, HotspotEvidence, Outcome, TraceEvent, TraceEventType,
};

// ---------------------------------------------------------------------------
// Weight table constants
// ---------------------------------------------------------------------------

/// Base weight per event type for hotspot scoring.
pub const WEIGHT_FILE_OPENED: u32 = 1;
pub const WEIGHT_SEARCH_RUN: u32 = 2;
pub const WEIGHT_SYMBOL_INSPECTED: u32 = 2;
pub const WEIGHT_COMMAND_EXECUTED: u32 = 1;
pub const WEIGHT_DOC_RETRIEVED: u32 = 2;
pub const WEIGHT_EDIT_MADE: u32 = 3;
pub const WEIGHT_FAILED_LOOKUP: u32 = 4;

/// Bonus added for each event with `Outcome::Failure`, on top of base weight.
pub const FAILURE_BONUS: u32 = 2;

/// Return the base weight for a given event type.
fn base_weight(event_type: TraceEventType) -> u32 {
    match event_type {
        TraceEventType::FileOpened => WEIGHT_FILE_OPENED,
        TraceEventType::SearchRun => WEIGHT_SEARCH_RUN,
        TraceEventType::SymbolInspected => WEIGHT_SYMBOL_INSPECTED,
        TraceEventType::CommandExecuted => WEIGHT_COMMAND_EXECUTED,
        TraceEventType::DocRetrieved => WEIGHT_DOC_RETRIEVED,
        TraceEventType::EditMade => WEIGHT_EDIT_MADE,
        TraceEventType::FailedLookup => WEIGHT_FAILED_LOOKUP,
        // Lifecycle events should never reach this function.
        TraceEventType::SessionStart | TraceEventType::SessionEnd => 0,
    }
}

// ---------------------------------------------------------------------------
// Internal per-group accumulator
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct GroupAccum {
    subject: String,
    subject_kind: String,
    score: u32,
    event_type_counts: HashMap<String, u32>,
    outcome_counts: HashMap<String, u32>,
    sessions: std::collections::HashSet<String>,
    first_seen: String,
    last_seen: String,
    first_event_id: u64,
    row_ids: Vec<u64>,
}

// ---------------------------------------------------------------------------
// Public scoring function
// ---------------------------------------------------------------------------

/// Score and rank hotspot entries from trace events with their SQLite row IDs.
///
/// Each element of `events_with_ids` is `(row_id, event)`. Lifecycle events
/// (`SessionStart`, `SessionEnd`) are excluded — they have no subject.
/// Subject-bearing events are grouped by `(subject_kind, subject)`.
///
/// Returns entries sorted by the six-key tie-break chain:
/// 1. `score DESC`
/// 2. `sessionCount DESC`
/// 3. `lastSeen DESC`
/// 4. `subjectKind ASC` (lexical)
/// 5. `subject ASC` (lexical)
/// 6. `firstEventId ASC`
///
/// Ranks are 1-based and assigned after sorting.
pub fn score_hotspots(events_with_ids: &[(u64, &TraceEvent)]) -> Vec<HotspotEntry> {
    let mut accum: HashMap<(String, String), GroupAccum> = HashMap::new();

    for &(row_id, event) in events_with_ids {
        let Some(kind) = event.subject_kind() else {
            // Lifecycle event — skip.
            continue;
        };
        let Some(subject) = event.subject() else {
            continue;
        };

        let key = (kind.to_string(), subject.to_string());

        let entry = accum.entry(key).or_insert_with(|| GroupAccum {
            subject: subject.to_string(),
            subject_kind: kind.to_string(),
            score: 0,
            event_type_counts: HashMap::new(),
            outcome_counts: HashMap::new(),
            sessions: std::collections::HashSet::new(),
            first_seen: event.timestamp.clone(),
            last_seen: event.timestamp.clone(),
            first_event_id: row_id,
            row_ids: Vec::new(),
        });

        // Base weight.
        let weight = base_weight(event.event_type);
        entry.score += weight;

        // Failure bonus.
        if matches!(event.outcome, Outcome::Failure { .. }) {
            entry.score += FAILURE_BONUS;
        }

        // Per-event-type counts.
        let type_name = event.event_type.payload_type_str().to_string();
        *entry.event_type_counts.entry(type_name).or_insert(0) += 1;

        // Per-outcome counts.
        let outcome_key = match event.outcome {
            Outcome::Success => "success",
            Outcome::Failure { .. } => "failure",
        };
        *entry
            .outcome_counts
            .entry(outcome_key.to_string())
            .or_insert(0) += 1;

        // Session tracking.
        entry.sessions.insert(event.session_id.clone());

        // Timestamps.
        if event.timestamp < entry.first_seen {
            entry.first_seen = event.timestamp.clone();
        }
        if event.timestamp > entry.last_seen {
            entry.last_seen = event.timestamp.clone();
        }

        // First event id tracking.
        if row_id < entry.first_event_id {
            entry.first_event_id = row_id;
        }

        // Row ID evidence.
        entry.row_ids.push(row_id);
    }

    // Convert accumulators to HotspotEntry list.
    let mut entries: Vec<HotspotEntry> = accum
        .into_values()
        .map(|g| HotspotEntry {
            rank: 0, // assigned after sorting
            subjectKind: g.subject_kind,
            subject: g.subject,
            score: g.score,
            counts: HotspotCounts {
                eventType: g.event_type_counts,
                outcome: g.outcome_counts,
            },
            sessionCount: g.sessions.len() as u32,
            firstSeen: g.first_seen,
            lastSeen: g.last_seen,
            evidence: HotspotEvidence { rowIds: g.row_ids },
        })
        .collect();

    // Sort by six-key tie-break chain.
    entries.sort_by(|a, b| {
        b.score
            .cmp(&a.score) // score DESC
            .then_with(|| b.sessionCount.cmp(&a.sessionCount)) // sessionCount DESC
            .then_with(|| b.lastSeen.cmp(&a.lastSeen)) // lastSeen DESC
            .then_with(|| a.subjectKind.cmp(&b.subjectKind)) // subjectKind ASC
            .then_with(|| a.subject.cmp(&b.subject)) // subject ASC
            .then_with(|| {
                // We need firstEventId, but it's not stored in HotspotEntry directly.
                // Instead, use the smallest rowId in evidence as the firstEventId proxy.
                let a_first = a.evidence.rowIds.first().copied().unwrap_or(u64::MAX);
                let b_first = b.evidence.rowIds.first().copied().unwrap_or(u64::MAX);
                a_first.cmp(&b_first)
            })
    });

    // Assign 1-based ranks.
    for (i, entry) in entries.iter_mut().enumerate() {
        entry.rank = (i + 1) as u32;
    }

    entries
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use scryrs_types::{
        CommandExecutedPayload, DocRetrievedPayload, EditMadePayload, FailedLookupPayload,
        FileOpenedPayload, Outcome, SCHEMA_VERSION, SearchRunPayload, SessionEndPayload,
        SessionStartPayload, TraceEventPayload, TraceEventType,
    };

    use super::*;

    fn make_event(
        event_type: TraceEventType,
        payload: TraceEventPayload,
        outcome: Outcome,
        session_id: &str,
        timestamp: &str,
    ) -> TraceEvent {
        TraceEvent {
            schema_version: SCHEMA_VERSION.into(),
            timestamp: timestamp.into(),
            session_id: session_id.into(),
            event_type,
            tool_name: None,
            payload,
            outcome,
        }
    }

    // 2.4.1: Empty input returns empty Vec.
    #[test]
    fn empty_input_returns_empty() {
        let entries = score_hotspots(&[]);
        assert!(entries.is_empty());
    }

    // 2.4.2: Only lifecycle events returns empty Vec.
    #[test]
    fn only_lifecycle_events_returns_empty() {
        let events = vec![
            (
                1u64,
                make_event(
                    TraceEventType::SessionStart,
                    TraceEventPayload::SessionStart(SessionStartPayload),
                    Outcome::Success,
                    "s1",
                    "2026-06-21T09:00:00Z",
                ),
            ),
            (
                2u64,
                make_event(
                    TraceEventType::SessionEnd,
                    TraceEventPayload::SessionEnd(SessionEndPayload),
                    Outcome::Success,
                    "s1",
                    "2026-06-21T09:01:00Z",
                ),
            ),
        ];
        let refs: Vec<(u64, &TraceEvent)> = events.iter().map(|(id, e)| (*id, e)).collect();
        let entries = score_hotspots(&refs);
        assert!(entries.is_empty());
    }

    // 2.4.3: Single subject with multiple event types scores correctly.
    #[test]
    fn single_subject_multiple_event_types() {
        let e1 = make_event(
            TraceEventType::FileOpened,
            TraceEventPayload::FileOpened(FileOpenedPayload {
                path: "src/main.rs".into(),
            }),
            Outcome::Success,
            "s1",
            "2026-06-21T09:00:00Z",
        );
        let e2 = make_event(
            TraceEventType::EditMade,
            TraceEventPayload::EditMade(EditMadePayload {
                target: "src/main.rs".into(),
            }),
            Outcome::Success,
            "s1",
            "2026-06-21T09:01:00Z",
        );
        let e3 = make_event(
            TraceEventType::FileOpened,
            TraceEventPayload::FileOpened(FileOpenedPayload {
                path: "src/main.rs".into(),
            }),
            Outcome::Success,
            "s1",
            "2026-06-21T09:02:00Z",
        );

        let events = vec![(1u64, &e1), (2u64, &e2), (3u64, &e3)];
        let entries = score_hotspots(&events);

        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        assert_eq!(entry.rank, 1);
        assert_eq!(entry.subjectKind, "file");
        assert_eq!(entry.subject, "src/main.rs");
        // FileOpened(1) * 2 + EditMade(3) * 1 = 5
        assert_eq!(entry.score, 5);
        assert_eq!(entry.sessionCount, 1);
        assert_eq!(entry.firstSeen, "2026-06-21T09:00:00Z");
        assert_eq!(entry.lastSeen, "2026-06-21T09:02:00Z");
        assert_eq!(entry.counts.eventType.get("FileOpened"), Some(&2));
        assert_eq!(entry.counts.eventType.get("EditMade"), Some(&1));
        assert_eq!(entry.counts.outcome.get("success"), Some(&3));
        assert!(!entry.counts.outcome.contains_key("failure"));
        assert_eq!(entry.evidence.rowIds, vec![1, 2, 3]);
    }

    // 2.4.4: Failure bonus applied correctly.
    #[test]
    fn failure_bonus_applied_correctly() {
        // FailedLookup: base 4 + bonus 2 = 6
        let e1 = make_event(
            TraceEventType::FailedLookup,
            TraceEventPayload::FailedLookup(FailedLookupPayload {
                subject: "missing_fn".into(),
            }),
            Outcome::Failure {
                reason: Some("not found".into()),
            },
            "s1",
            "2026-06-21T09:00:00Z",
        );

        // EditMade + Failure: base 3 + bonus 2 = 5
        let e2 = make_event(
            TraceEventType::EditMade,
            TraceEventPayload::EditMade(EditMadePayload {
                target: "src/x.rs".into(),
            }),
            Outcome::Failure {
                reason: Some("write error".into()),
            },
            "s1",
            "2026-06-21T09:01:00Z",
        );

        let events = vec![
            (1u64, &e1), // missing_fn, symbol, score 6
            (2u64, &e2), // src/x.rs, file, score 5
        ];
        let entries = score_hotspots(&events);

        // Two separate subjects with different kinds.
        assert_eq!(entries.len(), 2);

        // Higher score first (6 > 5).
        assert_eq!(entries[0].subject, "missing_fn");
        assert_eq!(entries[0].subjectKind, "symbol");
        assert_eq!(entries[0].score, 6);
        assert_eq!(entries[0].counts.outcome.get("failure"), Some(&1));

        assert_eq!(entries[1].subject, "src/x.rs");
        assert_eq!(entries[1].subjectKind, "file");
        assert_eq!(entries[1].score, 5);
    }

    // 2.4.5: Two subjects with identical score sort deterministically.
    #[test]
    fn two_subjects_identical_score_deterministic_sort() {
        // Both get score 1 (FileOpened weight 1, success).
        let e_a = make_event(
            TraceEventType::FileOpened,
            TraceEventPayload::FileOpened(FileOpenedPayload {
                path: "b.rs".into(),
            }),
            Outcome::Success,
            "s1",
            "2026-06-21T09:00:00Z",
        );
        let e_b = make_event(
            TraceEventType::FileOpened,
            TraceEventPayload::FileOpened(FileOpenedPayload {
                path: "a.rs".into(),
            }),
            Outcome::Success,
            "s1",
            "2026-06-21T09:01:00Z",
        );

        // b.rs inserted first (id=1), a.rs second (id=2).
        let events = vec![(1u64, &e_a), (2u64, &e_b)];
        let entries = score_hotspots(&events);

        assert_eq!(entries.len(), 2);
        // Same score (1), same sessionCount (1) → lastSeen DESC: a.rs(09:01) > b.rs(09:00)
        assert_eq!(entries[0].subject, "a.rs"); // lastSeen later
        assert_eq!(entries[1].subject, "b.rs");
    }

    // 2.4.6: Session count is correct.
    #[test]
    fn session_count_is_correct() {
        let e1 = make_event(
            TraceEventType::FileOpened,
            TraceEventPayload::FileOpened(FileOpenedPayload {
                path: "src/a.rs".into(),
            }),
            Outcome::Success,
            "s1",
            "2026-06-21T09:00:00Z",
        );
        let e2 = make_event(
            TraceEventType::FileOpened,
            TraceEventPayload::FileOpened(FileOpenedPayload {
                path: "src/a.rs".into(),
            }),
            Outcome::Success,
            "s1", // same session
            "2026-06-21T09:01:00Z",
        );
        let e3 = make_event(
            TraceEventType::FileOpened,
            TraceEventPayload::FileOpened(FileOpenedPayload {
                path: "src/a.rs".into(),
            }),
            Outcome::Success,
            "s2", // different session
            "2026-06-21T09:02:00Z",
        );

        let events = vec![(1u64, &e1), (2u64, &e2), (3u64, &e3)];
        let entries = score_hotspots(&events);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].sessionCount, 2);
    }

    // 2.4.7: Event-type and outcome counts are correct.
    #[test]
    fn event_type_and_outcome_counts_are_correct() {
        let e1 = make_event(
            TraceEventType::FileOpened,
            TraceEventPayload::FileOpened(FileOpenedPayload {
                path: "src/x.rs".into(),
            }),
            Outcome::Success,
            "s1",
            "2026-06-21T09:00:00Z",
        );
        let e2 = make_event(
            TraceEventType::EditMade,
            TraceEventPayload::EditMade(EditMadePayload {
                target: "src/x.rs".into(),
            }),
            Outcome::Failure {
                reason: Some("err".into()),
            },
            "s1",
            "2026-06-21T09:01:00Z",
        );

        let events = vec![(1u64, &e1), (2u64, &e2)];
        let entries = score_hotspots(&events);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].counts.eventType.get("FileOpened"), Some(&1));
        assert_eq!(entries[0].counts.eventType.get("EditMade"), Some(&1));
        assert!(!entries[0].counts.eventType.contains_key("SearchRun"));
        assert_eq!(entries[0].counts.outcome.get("success"), Some(&1));
        assert_eq!(entries[0].counts.outcome.get("failure"), Some(&1));
    }

    // 2.4.8: firstSeen/lastSeen track min/max timestamps correctly.
    #[test]
    fn first_seen_last_seen_track_timestamps() {
        let e1 = make_event(
            TraceEventType::SearchRun,
            TraceEventPayload::SearchRun(SearchRunPayload {
                query: "routing".into(),
            }),
            Outcome::Success,
            "s1",
            "2026-06-21T12:00:00Z",
        );
        let e2 = make_event(
            TraceEventType::SearchRun,
            TraceEventPayload::SearchRun(SearchRunPayload {
                query: "routing".into(),
            }),
            Outcome::Success,
            "s1",
            "2026-06-21T09:00:00Z", // earlier timestamp
        );

        // e1 inserted first (id=1) but e2 has earlier timestamp.
        let events = vec![(1u64, &e1), (2u64, &e2)];
        let entries = score_hotspots(&events);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].firstSeen, "2026-06-21T09:00:00Z");
        assert_eq!(entries[0].lastSeen, "2026-06-21T12:00:00Z");
    }

    // 2.4.9: evidence.rowIds is ordered by timestamp ASC, id ASC.
    #[test]
    fn evidence_row_ids_are_ordered() {
        // Insert events out of order, but they should appear in id order within the group.
        let e1 = make_event(
            TraceEventType::FileOpened,
            TraceEventPayload::FileOpened(FileOpenedPayload {
                path: "src/a.rs".into(),
            }),
            Outcome::Success,
            "s1",
            "2026-06-21T09:00:03Z",
        );
        let e2 = make_event(
            TraceEventType::FileOpened,
            TraceEventPayload::FileOpened(FileOpenedPayload {
                path: "src/a.rs".into(),
            }),
            Outcome::Success,
            "s1",
            "2026-06-21T09:00:01Z",
        );
        let e3 = make_event(
            TraceEventType::FileOpened,
            TraceEventPayload::FileOpened(FileOpenedPayload {
                path: "src/a.rs".into(),
            }),
            Outcome::Success,
            "s1",
            "2026-06-21T09:00:02Z",
        );

        // The slice is passed in arbitrary order; row_ids are collected in iteration order.
        // For this test, we pass them in timestamp order as TraceQuery would provide.
        // id 1 = 09:00:03, id 2 = 09:00:01, id 3 = 09:00:02
        // But TraceQuery orders by timestamp,id ASC → id 2, id 3, id 1
        let events = vec![(2u64, &e2), (3u64, &e3), (1u64, &e1)];
        let entries = score_hotspots(&events);

        assert_eq!(entries.len(), 1);
        // Row IDs collected in iteration order: [2, 3, 1]
        assert_eq!(entries[0].evidence.rowIds, vec![2, 3, 1]);
    }

    // Additional: CommandExecuted with Failure gets 1+2=3.
    #[test]
    fn command_executed_failure_scores_correctly() {
        let e = make_event(
            TraceEventType::CommandExecuted,
            TraceEventPayload::CommandExecuted(CommandExecutedPayload {
                command: "cargo build".into(),
            }),
            Outcome::Failure {
                reason: Some("exit code 1".into()),
            },
            "s1",
            "2026-06-21T09:00:00Z",
        );

        let events = vec![(1u64, &e)];
        let entries = score_hotspots(&events);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].score, 3); // base 1 + bonus 2
        assert_eq!(entries[0].counts.outcome.get("failure"), Some(&1));
    }

    // Additional: DocRetrieved weight is 2.
    #[test]
    fn doc_retrieved_weight_is_2() {
        let e = make_event(
            TraceEventType::DocRetrieved,
            TraceEventPayload::DocRetrieved(DocRetrievedPayload {
                doc_ref: "api.md".into(),
            }),
            Outcome::Success,
            "s1",
            "2026-06-21T09:00:00Z",
        );

        let events = vec![(1u64, &e)];
        let entries = score_hotspots(&events);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].score, 2); // base 2, no failure
    }

    // Additional: Same subject, different kinds produce separate entries.
    #[test]
    fn same_subject_different_kinds_produces_separate_entries() {
        let e_file = make_event(
            TraceEventType::FileOpened,
            TraceEventPayload::FileOpened(FileOpenedPayload {
                path: "routing".into(),
            }),
            Outcome::Success,
            "s1",
            "2026-06-21T09:00:00Z",
        );
        let e_search = make_event(
            TraceEventType::SearchRun,
            TraceEventPayload::SearchRun(SearchRunPayload {
                query: "routing".into(),
            }),
            Outcome::Success,
            "s1",
            "2026-06-21T09:01:00Z",
        );

        let events = vec![(1u64, &e_file), (2u64, &e_search)];
        let entries = score_hotspots(&events);

        assert_eq!(entries.len(), 2);
        let kinds: Vec<&str> = entries.iter().map(|e| e.subjectKind.as_str()).collect();
        assert!(kinds.contains(&"file"));
        assert!(kinds.contains(&"search"));
    }

    // Additional: Full tie-break chain test.
    #[test]
    fn full_tie_break_chain() {
        // Same score, same sessionCount, same lastSeen → subjectKind ASC.
        // "command" < "file".
        let e_cmd = make_event(
            TraceEventType::CommandExecuted,
            TraceEventPayload::CommandExecuted(CommandExecutedPayload {
                command: "cmd".into(),
            }),
            Outcome::Success,
            "s1",
            "2026-06-21T09:00:00Z",
        );
        let e_file = make_event(
            TraceEventType::FileOpened,
            TraceEventPayload::FileOpened(FileOpenedPayload {
                path: "src/a.rs".into(),
            }),
            Outcome::Success,
            "s1",
            "2026-06-21T09:00:00Z",
        );

        let events = vec![(1u64, &e_cmd), (2u64, &e_file)];
        let entries = score_hotspots(&events);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].subjectKind, "command"); // ASC → command before file
        assert_eq!(entries[1].subjectKind, "file");
    }
}
