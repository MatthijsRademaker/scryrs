//! End-to-end integration tests for the public CLI pipeline:
//! `scryrs record --stdin → .scryrs/scryrs.db → scryrs hotspots <PATH> → .scryrs/hotspots.json`
//!
//! These tests exercise the canonical CWD-based path that real users experience,
//! and include `insta` snapshot assertions for hotspot output drift detection.

use scryrs_cli::test_support::with_cwd;
use scryrs_types::SCHEMA_VERSION;

// ---------------------------------------------------------------------------
// JSONL fixture helpers — one per subject-bearing event family
// ---------------------------------------------------------------------------

fn make_file_opened_json(session_id: &str, path: &str, timestamp: &str) -> String {
    format!(
        r#"{{"schema_version":"{}","timestamp":"{}","session_id":"{}","event_type":"FileOpened","tool_name":"read","payload":{{"type":"FileOpened","path":"{}"}},"outcome":{{"result":"Success"}}}}"#,
        SCHEMA_VERSION, timestamp, session_id, path,
    )
}

fn make_search_run_json(session_id: &str, query: &str, timestamp: &str) -> String {
    format!(
        r#"{{"schema_version":"{}","timestamp":"{}","session_id":"{}","event_type":"SearchRun","tool_name":"search","payload":{{"type":"SearchRun","query":"{}"}},"outcome":{{"result":"Success"}}}}"#,
        SCHEMA_VERSION, timestamp, session_id, query,
    )
}

fn make_symbol_inspected_json(session_id: &str, name: &str, timestamp: &str) -> String {
    format!(
        r#"{{"schema_version":"{}","timestamp":"{}","session_id":"{}","event_type":"SymbolInspected","tool_name":"inspect","payload":{{"type":"SymbolInspected","name":"{}"}},"outcome":{{"result":"Success"}}}}"#,
        SCHEMA_VERSION, timestamp, session_id, name,
    )
}

fn make_command_executed_json(
    session_id: &str,
    command: &str,
    timestamp: &str,
    success: bool,
) -> String {
    let outcome = if success {
        r#"{"result":"Success"}"#
    } else {
        r#"{"result":"Failure","reason":"exit code 1"}"#
    };
    format!(
        r#"{{"schema_version":"{}","timestamp":"{}","session_id":"{}","event_type":"CommandExecuted","tool_name":"bash","payload":{{"type":"CommandExecuted","command":"{}"}},"outcome":{}}}"#,
        SCHEMA_VERSION, timestamp, session_id, command, outcome,
    )
}

fn make_doc_retrieved_json(session_id: &str, doc_ref: &str, timestamp: &str) -> String {
    format!(
        r#"{{"schema_version":"{}","timestamp":"{}","session_id":"{}","event_type":"DocRetrieved","tool_name":"read","payload":{{"type":"DocRetrieved","doc_ref":"{}"}},"outcome":{{"result":"Success"}}}}"#,
        SCHEMA_VERSION, timestamp, session_id, doc_ref,
    )
}

fn make_edit_made_json(session_id: &str, target: &str, timestamp: &str, success: bool) -> String {
    let outcome = if success {
        r#"{"result":"Success"}"#.to_string()
    } else {
        r#"{"result":"Failure","reason":"write error"}"#.to_string()
    };
    format!(
        r#"{{"schema_version":"{}","timestamp":"{}","session_id":"{}","event_type":"EditMade","tool_name":"edit","payload":{{"type":"EditMade","target":"{}"}},"outcome":{}}}"#,
        SCHEMA_VERSION, timestamp, session_id, target, outcome,
    )
}

fn make_failed_lookup_json(
    session_id: &str,
    subject: &str,
    reason: &str,
    timestamp: &str,
) -> String {
    format!(
        r#"{{"schema_version":"{}","timestamp":"{}","session_id":"{}","event_type":"FailedLookup","tool_name":"search","payload":{{"type":"FailedLookup","subject":"{}"}},"outcome":{{"result":"Failure","reason":"{}"}}}}"#,
        SCHEMA_VERSION, timestamp, session_id, subject, reason,
    )
}

// ---------------------------------------------------------------------------
// Snapshot normalization — replace volatile fields with placeholders
// ---------------------------------------------------------------------------

fn normalize_hotspot_json(json: &str) -> String {
    let mut value: serde_json::Value =
        serde_json::from_str(json).unwrap_or_else(|e| panic!("parse hotspot JSON: {e}"));

    if let Some(obj) = value.as_object_mut() {
        if let Some(v) = obj.get_mut("generatedAt") {
            *v = serde_json::Value::String("<GENERATED_AT>".into());
        }
        if let Some(v) = obj.get_mut("repositoryPath") {
            *v = serde_json::Value::String("<REPO>".into());
        }
        if let Some(v) = obj.get_mut("storePath") {
            *v = serde_json::Value::String("<STORE>".into());
        }
    }

    serde_json::to_string_pretty(&value).unwrap_or_else(|e| panic!("re-serialize: {e}"))
}

// ---------------------------------------------------------------------------
// Multi-event fixture — builds a JSONL string covering all 7 families + failure
// ---------------------------------------------------------------------------

fn build_multi_event_fixture() -> String {
    let lines = vec![
        // FileOpened — src/main.rs, session s1, score 1
        make_file_opened_json("s1", "src/main.rs", "2026-06-21T09:00:00Z"),
        // SearchRun — "error handling", session s1, score 2
        make_search_run_json("s1", "error handling", "2026-06-21T09:01:00Z"),
        // SymbolInspected — "Dispatcher", session s1, score 2
        make_symbol_inspected_json("s1", "Dispatcher", "2026-06-21T09:02:00Z"),
        // CommandExecuted (success) — "cargo build", session s1, score 1
        make_command_executed_json("s1", "cargo build", "2026-06-21T09:03:00Z", true),
        // CommandExecuted (failure) — "cargo test", session s2, score 1+2=3
        make_command_executed_json("s2", "cargo test", "2026-06-21T09:04:00Z", false),
        // DocRetrieved — "docs/api.md", session s1, score 2
        make_doc_retrieved_json("s1", "docs/api.md", "2026-06-21T09:05:00Z"),
        // EditMade (success) — src/lib.rs, session s1, score 3
        make_edit_made_json("s1", "src/lib.rs", "2026-06-21T09:06:00Z", true),
        // EditMade (failure) — src/broken.rs, session s2, score 3+2=5
        make_edit_made_json("s2", "src/broken.rs", "2026-06-21T09:07:00Z", false),
        // FailedLookup — "nonexistent_fn", session s1, score 4+2=6
        make_failed_lookup_json(
            "s1",
            "nonexistent_fn",
            "symbol not found",
            "2026-06-21T09:08:00Z",
        ),
        // FileOpened — src/main.rs again (same session s1), adds +1 to src/main.rs score
        make_file_opened_json("s1", "src/main.rs", "2026-06-21T09:09:00Z"),
    ];

    lines.join("\n") + "\n"
}

// ---------------------------------------------------------------------------
// E2E Tests
// ---------------------------------------------------------------------------

/// Full pipeline: record --stdin → SQLite → hotspots → artifact + snapshots.
#[test]
fn e2e_record_to_hotspots_pipeline() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));

    with_cwd(dir.path(), || {
        // Step 1: Pipe multi-event JSONL through `scryrs record --stdin`.
        let fixture = build_multi_event_fixture();
        let mut out = Vec::new();
        let mut err = Vec::new();

        let exit = scryrs_cli::run_with_io(
            ["record", "--stdin"],
            &mut out,
            &mut err,
            fixture.as_bytes(),
        );

        assert_eq!(
            exit,
            0,
            "record stderr: {:?}",
            String::from_utf8_lossy(&err)
        );

        let stdout = String::from_utf8_lossy(&out);
        assert!(stdout.contains("\"command\":\"record\""));
        assert!(stdout.contains("\"accepted\":10"));
        assert!(stdout.contains("\"rejected\":0"));

        // Step 2: Verify SQLite rows.
        let store_path = dir.path().join(".scryrs/scryrs.db");
        assert!(store_path.exists(), "store must exist at canonical path");

        let conn =
            rusqlite::Connection::open(&store_path).unwrap_or_else(|e| panic!("open store: {e}"));
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM trace_events", [], |row| row.get(0))
            .unwrap_or_else(|e| panic!("count: {e}"));
        assert_eq!(count, 10);

        // Verify at least one row per subject-bearing event family.
        let families = [
            "FileOpened",
            "SearchRun",
            "SymbolInspected",
            "CommandExecuted",
            "DocRetrieved",
            "EditMade",
            "FailedLookup",
        ];
        for family in &families {
            let fc: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM trace_events WHERE event_type = ?1",
                    [family],
                    |row| row.get(0),
                )
                .unwrap_or_else(|e| panic!("count {family}: {e}"));
            assert!(fc > 0, "must have at least one {family} event");
        }

        // Step 3: Run `scryrs hotspots <PATH>`.
        let mut hout = Vec::new();
        let mut herr = Vec::new();

        let exit = scryrs_cli::run_with_writers(
            ["hotspots", &dir.path().display().to_string()],
            &mut hout,
            &mut herr,
        );

        assert_eq!(
            exit,
            0,
            "hotspots exit {exit}, stderr: {:?}",
            String::from_utf8_lossy(&herr)
        );

        let hstdout = String::from_utf8_lossy(&hout);
        let report: serde_json::Value =
            serde_json::from_str(hstdout.trim()).unwrap_or_else(|e| panic!("parse: {e}"));

        // Verify schema and metadata.
        assert_eq!(report["schemaVersion"], "1.0.0");
        assert_eq!(report["command"], "hotspots");
        assert_eq!(report["runMetadata"]["analyzedEventCount"], 10);
        // 9 distinct (subject_kind, subject) pairs:
        // file:src/main.rs, search:error handling, symbol:Dispatcher,
        // command:cargo build, command:cargo test, document:docs/api.md,
        // file:src/lib.rs, file:src/broken.rs, symbol:nonexistent_fn
        assert_eq!(report["runMetadata"]["analyzedSubjectCount"], 9);

        // Step 4: Verify artifact file exists and matches stdout.
        let artifact_path = dir.path().join(".scryrs/hotspots.json");
        assert!(artifact_path.exists(), "hotspots.json must exist");

        let artifact_bytes =
            std::fs::read(&artifact_path).unwrap_or_else(|e| panic!("read artifact: {e}"));
        // stdout has trailing newline from writeln!, artifact does not
        let stdout_bytes = hout.trim_ascii_end();
        assert_eq!(
            artifact_bytes, stdout_bytes,
            "artifact must match stdout byte-for-byte (modulo trailing newline)"
        );

        // Step 5: Verify ranking — highest score first.
        let entries = report["entries"]
            .as_array()
            .unwrap_or_else(|| panic!("entries not array"));
        assert!(!entries.is_empty());

        // Expected scores:
        // nonexistent_fn (FailedLookup): 4 + 2 failure = 6
        // src/broken.rs (EditMade Failure): 3 + 2 = 5
        // src/lib.rs (EditMade Success): 3
        // cargo test (CommandExecuted Failure): 1 + 2 = 3
        // src/main.rs (2x FileOpened): 1+1 = 2
        // error handling (SearchRun): 2
        // Dispatcher (SymbolInspected): 2
        // docs/api.md (DocRetrieved): 2
        // cargo build (CommandExecuted Success): 1

        // Top entry: nonexistent_fn with score 6
        assert_eq!(entries[0]["subject"], "nonexistent_fn");
        assert_eq!(entries[0]["subjectKind"], "symbol");
        assert_eq!(entries[0]["score"], 6);

        // Second: src/broken.rs with score 5
        assert_eq!(entries[1]["subject"], "src/broken.rs");
        assert_eq!(entries[1]["subjectKind"], "file");
        assert_eq!(entries[1]["score"], 5);

        // Step 6: Snapshot assertions with volatile-field normalization.
        let normalized = normalize_hotspot_json(hstdout.trim());
        insta::assert_snapshot!("hotspot_stdout", normalized);

        let normalized_artifact = normalize_hotspot_json(
            std::str::from_utf8(&artifact_bytes).unwrap_or_else(|e| panic!("utf8: {e}")),
        );
        let artifact_value: serde_json::Value =
            serde_json::from_str(&normalized_artifact).unwrap_or_else(|e| panic!("parse: {e}"));
        insta::assert_json_snapshot!("hotspot_artifact", &artifact_value);
    });
}

/// Empty store (valid schema, no events) produces entries: [] with exit 0.
#[test]
fn e2e_empty_store_produces_success() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));

    // Create an empty store with proper schema by recording zero events.
    with_cwd(dir.path(), || {
        // Initialize the store by piping empty input through record --stdin.
        // This creates the canonical .scryrs/scryrs.db with schema but no trace_events.
        let mut out = Vec::new();
        let mut err = Vec::new();
        let exit = scryrs_cli::run_with_io(
            ["record", "--stdin"],
            &mut out,
            &mut err,
            "\n".as_bytes(), // blank line is skipped by ingest_jsonl
        );
        // Blank-only input: all lines skipped, no accepted events, no rejections.
        // execute_record will still commit a transaction with 0 events.
        assert_eq!(
            exit,
            0,
            "record empty stderr: {:?}",
            String::from_utf8_lossy(&err)
        );

        let mut hout = Vec::new();
        let mut herr = Vec::new();

        let exit = scryrs_cli::run_with_writers(
            ["hotspots", &dir.path().display().to_string()],
            &mut hout,
            &mut herr,
        );

        assert_eq!(
            exit,
            0,
            "empty store exit {exit}, stderr: {:?}",
            String::from_utf8_lossy(&herr)
        );

        let hstdout = String::from_utf8_lossy(&hout);
        let report: serde_json::Value =
            serde_json::from_str(hstdout.trim()).unwrap_or_else(|e| panic!("parse: {e}"));

        let entries = report["entries"]
            .as_array()
            .unwrap_or_else(|| panic!("entries not array"));
        assert_eq!(entries.len(), 0);
        assert_eq!(report["runMetadata"]["analyzedEventCount"], 0);
        assert_eq!(report["runMetadata"]["analyzedSubjectCount"], 0);

        // Snapshot the normalized empty report.
        let normalized = normalize_hotspot_json(hstdout.trim());
        insta::assert_snapshot!("hotspot_empty_stdout", normalized);
    });
}

/// Missing store (no .scryrs/scryrs.db) exits 2 with error.
#[test]
fn e2e_missing_store_exits_2() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));

    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        let exit = scryrs_cli::run_with_writers(
            ["hotspots", &dir.path().display().to_string()],
            &mut out,
            &mut err,
        );

        assert_eq!(exit, 2);
        assert!(out.is_empty(), "stdout must be empty for missing store");

        let stderr = String::from_utf8_lossy(&err);
        assert!(
            stderr.contains("datastore not found"),
            "stderr must mention datastore not found, got: {stderr}"
        );
    });
}
