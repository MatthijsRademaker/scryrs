//! End-to-end integration tests for the public CLI pipeline:
//! `scryrs record --stdin → .scryrs/scryrs.db → scryrs hotspots <PATH> → .scryrs/hotspots.json`
//!
//! These tests exercise the canonical CWD-based path that real users experience,
//! and include `insta` snapshot assertions for hotspot output drift detection.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Mutex, mpsc};
use std::thread;

use scryrs_types::SCHEMA_VERSION;

static CWD_GUARD: Mutex<()> = Mutex::new(());

fn with_cwd(dir: &std::path::Path, f: impl FnOnce()) {
    let _lock = CWD_GUARD
        .lock()
        .unwrap_or_else(|e| panic!("CWD guard poisoned: {e}"));
    let original = std::env::current_dir().unwrap_or_else(|e| panic!("current_dir: {e}"));
    std::env::set_current_dir(dir).unwrap_or_else(|e| panic!("set_current_dir: {e}"));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    std::env::set_current_dir(&original).unwrap_or_else(|e| panic!("restore cwd: {e}"));
    if let Err(error) = result {
        std::panic::resume_unwind(error);
    }
}

fn spawn_live_hotspots_server(response_body: String) -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap_or_else(|e| panic!("bind server: {e}"));
    let addr = listener
        .local_addr()
        .unwrap_or_else(|e| panic!("local addr: {e}"));
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap_or_else(|e| panic!("accept: {e}"));
        let mut buf = [0_u8; 8192];
        let bytes = stream
            .read(&mut buf)
            .unwrap_or_else(|e| panic!("read request: {e}"));
        let request = String::from_utf8_lossy(&buf[..bytes]).to_string();
        let request_line = request
            .lines()
            .next()
            .unwrap_or_else(|| panic!("missing request line"))
            .to_string();
        tx.send(request_line)
            .unwrap_or_else(|e| panic!("send request line: {e}"));

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response_body.len(),
            response_body
        );
        stream
            .write_all(response.as_bytes())
            .unwrap_or_else(|e| panic!("write response: {e}"));
    });

    (format!("http://{addr}"), rx)
}

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
            ["record", "--stdin", "--mode", "local"],
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
            ["record", "--stdin", "--mode", "local"],
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

#[test]
#[allow(clippy::disallowed_methods)]
fn e2e_live_hotspots_uses_process_env_precedence_and_percent_encodes_repository_id() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));

    with_cwd(dir.path(), || {
        std::fs::create_dir_all(dir.path().join(".scryrs"))
            .unwrap_or_else(|e| panic!("create .scryrs: {e}"));
        std::fs::write(
            dir.path().join("scryrs.json"),
            r#"{"remote":{"ingest_url":"http://manifest.invalid:9","repository_id":"manifest-repo"}}"#,
        )
        .unwrap_or_else(|e| panic!("write manifest: {e}"));
        std::fs::write(
            dir.path().join(".scryrs/.env"),
            "SCRYRS_REMOTE_INGEST_URL=http://dotenv.invalid:9\nSCRYRS_REPOSITORY_ID=dotenv-repo\n",
        )
        .unwrap_or_else(|e| panic!("write dotenv: {e}"));

        let repository_id = "https://github.com/acme/widgets";
        let encoded_repository_id = "https%3A%2F%2Fgithub.com%2Facme%2Fwidgets";
        let response = serde_json::json!({
            "schemaVersion": "1.0.0",
            "repositoryId": repository_id,
            "cursor": "",
            "generatedAt": "2026-07-01T12:00:00Z",
            "entries": [
                {
                    "rank": 1,
                    "subjectKind": "file",
                    "subject": "src/live.rs",
                    "score": 2,
                    "counts": {"eventType": {}, "outcome": {}},
                    "sessionCount": 1,
                    "firstSeen": "2026-07-01T11:00:00Z",
                    "lastSeen": "2026-07-01T11:00:00Z",
                    "evidence": {"rowIds": [7, 8]}
                }
            ]
        })
        .to_string();
        let (server_url, request_rx) = spawn_live_hotspots_server(response);

        let output = std::process::Command::new(env!("CARGO_BIN_EXE_scryrs"))
            .current_dir(dir.path())
            .args(["hotspots", ".", "--mode", "live"])
            .env("SCRYRS_REMOTE_INGEST_URL", &server_url)
            .env("SCRYRS_REPOSITORY_ID", repository_id)
            .output()
            .unwrap_or_else(|e| panic!("run scryrs: {e}"));

        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let request_line = request_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap_or_else(|e| panic!("request line: {e}"));
        assert_eq!(
            request_line,
            format!(
                "GET /v1/repositories/{encoded_repository_id}/hotspots?window=cumulative HTTP/1.1"
            )
        );

        let stdout =
            String::from_utf8(output.stdout).unwrap_or_else(|e| panic!("stdout utf8: {e}"));
        let report: serde_json::Value =
            serde_json::from_str(stdout.trim_end()).unwrap_or_else(|e| panic!("parse stdout: {e}"));
        assert_eq!(
            report["storePath"],
            format!(
                "live:{server_url}/v1/repositories/{encoded_repository_id}/hotspots?window=cumulative"
            )
        );
        assert_eq!(report["entries"][0]["subject"], "src/live.rs");

        let artifact = std::fs::read_to_string(dir.path().join(".scryrs/hotspots.json"))
            .unwrap_or_else(|e| panic!("read artifact: {e}"));
        assert_eq!(artifact, stdout);
        let stderr =
            String::from_utf8(output.stderr).unwrap_or_else(|e| panic!("stderr utf8: {e}"));
        assert!(stderr.contains(&server_url));
        assert!(stderr.contains(repository_id));
    });
}
