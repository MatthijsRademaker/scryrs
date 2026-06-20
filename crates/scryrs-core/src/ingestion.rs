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
        match serde_json::from_str::<TraceEvent>(&line) {
            Ok(event) => accepted.push(event),
            Err(e) => rejected.push(Rejection {
                line: line_1based,
                field: None,
                reason: e.to_string(),
            }),
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
}
