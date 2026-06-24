//! Tests for the native `scryrs hook <harness>` subcommand.
//!
//! Covers dispatch/discoverability, end-to-end translation + persistence for
//! both harnesses, and the fail-open contract (malformed input, unknown
//! harness, unwritable store all exit 0 and log a warning).

use crate::{run_with_io, run_with_writers};

// --- helpers ---------------------------------------------------------------

/// A Claude Code PreToolUse payload rooted at `cwd`.
fn cc_payload(
    cwd: &std::path::Path,
    session_id: &str,
    tool: &str,
    input: serde_json::Value,
) -> String {
    serde_json::json!({
        "session_id": session_id,
        "cwd": cwd.to_str().unwrap_or("."),
        "tool_name": tool,
        "tool_input": input,
    })
    .to_string()
}

/// Open the store and return the row count.
fn event_count(store_path: &std::path::Path) -> i64 {
    let conn = rusqlite::Connection::open(store_path)
        .unwrap_or_else(|e| panic!("open store {}: {e}", store_path.display()));
    conn.query_row("SELECT COUNT(*) FROM trace_events", [], |row| row.get(0))
        .unwrap_or_else(|e| panic!("count: {e}"))
}

/// Open the store and return `(tool_name, session_id, event_type, outcome)` for the single row.
fn single_row(store_path: &std::path::Path) -> (Option<String>, String, String, String) {
    let conn = rusqlite::Connection::open(store_path).unwrap_or_else(|e| panic!("open store: {e}"));
    conn.query_row(
        "SELECT tool_name, session_id, event_type, outcome FROM trace_events LIMIT 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )
    .unwrap_or_else(|e| panic!("row: {e}"))
}

fn read_warning_log(cwd: &std::path::Path, harness: &str) -> String {
    let path = cwd
        .join(".scryrs")
        .join("hooks")
        .join(format!("{harness}-warnings.log"));
    std::fs::read_to_string(&path).unwrap_or_default()
}

// --- 2.1 discoverability / dispatch ----------------------------------------

#[test]
fn hook_is_recognized_not_unknown_command() {
    // `scryrs hook claude-code` with empty stdin must not be "unknown command";
    // it fails open with exit 0.
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run_with_io(["hook", "claude-code"], &mut out, &mut err, &b""[..]);
    assert_eq!(code, 0, "hook must reach dispatch and fail open");
    let stderr = String::from_utf8_lossy(&err);
    assert!(
        !stderr.contains("unknown command"),
        "hook must be a known command, got: {stderr}"
    );
}

#[test]
fn hook_missing_harness_is_usage_error() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run_with_writers(["hook"], &mut out, &mut err);
    assert_eq!(code, 2, "missing HARNESS positional is a usage error");
    let stderr = String::from_utf8_lossy(&err);
    assert!(stderr.contains("scryrs hook: missing required HARNESS argument"));
}

#[test]
fn hook_appears_in_help() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(run_with_writers(["--help"], &mut out, &mut err), 0);
    let help = String::from_utf8_lossy(&out);
    assert!(help.contains("scryrs hook <HARNESS>"));
    assert!(help.contains("claude-code (stdin), pi (--file)"));
}

// --- 5.1 claude-code end-to-end --------------------------------------------

#[test]
fn claude_code_tracked_tool_persists_one_event_under_cwd() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    let payload = cc_payload(
        dir.path(),
        "abc123",
        "Read",
        serde_json::json!({"file_path": "src/main.rs"}),
    );

    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run_with_io(
        ["hook", "claude-code"],
        &mut out,
        &mut err,
        payload.as_bytes(),
    );

    assert_eq!(code, 0, "hook must exit 0");
    assert!(out.is_empty(), "hook must write nothing to stdout");

    // Store resolved against payload cwd (D5).
    let store_path = dir.path().join(".scryrs/scryrs.db");
    assert!(
        store_path.exists(),
        "store must be created under payload cwd"
    );
    assert_eq!(event_count(&store_path), 1);

    let (tool_name, session_id, event_type, outcome) = single_row(&store_path);
    assert_eq!(tool_name.as_deref(), Some("Read"));
    assert_eq!(session_id, "abc123");
    assert_eq!(event_type, "FileOpened");
    assert_eq!(outcome, "Success");
}

#[test]
fn claude_code_untracked_tool_persists_nothing() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    let payload = cc_payload(dir.path(), "s1", "TodoWrite", serde_json::json!({}));

    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run_with_io(
        ["hook", "claude-code"],
        &mut out,
        &mut err,
        payload.as_bytes(),
    );

    assert_eq!(code, 0);
    let store_path = dir.path().join(".scryrs/scryrs.db");
    // No event → store may exist (open creates it) but holds zero rows, or not
    // be created at all. Either way: zero tracked events.
    if store_path.exists() {
        assert_eq!(event_count(&store_path), 0);
    }
}

// --- 5.2 fail-open ---------------------------------------------------------

#[test]
fn malformed_input_exits_0_and_logs_warning() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    // Run with cwd as the temp dir so the warning log lands there. Malformed
    // JSON has no `cwd`, so base_dir falls back to process cwd.
    crate::test_support::with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = run_with_io(
            ["hook", "claude-code"],
            &mut out,
            &mut err,
            &b"this is not json"[..],
        );
        assert_eq!(code, 0, "malformed input must exit 0");
        assert!(out.is_empty(), "stdout must be empty");
    });

    let log = read_warning_log(dir.path(), "claude-code");
    assert!(
        log.contains("malformed JSON"),
        "warning log must record malformed input, got: {log:?}"
    );
}

#[test]
fn unwritable_store_exits_0_and_logs_warning() {
    // Warning log dir (payload cwd) is writable; the store path is not.
    let cwd_dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    let bad_dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    // Make `<bad>/.scryrs` a regular file so EventStore::open cannot create the db.
    std::fs::write(bad_dir.path().join(".scryrs"), "blocked")
        .unwrap_or_else(|e| panic!("write blocker: {e}"));
    let store_path = bad_dir.path().join(".scryrs/scryrs.db");
    crate::store_override::set(
        store_path
            .to_str()
            .unwrap_or_else(|| panic!("store path not UTF-8"))
            .to_string(),
    );

    let payload = cc_payload(
        cwd_dir.path(),
        "s1",
        "Read",
        serde_json::json!({"file_path": "a.rs"}),
    );
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run_with_io(
        ["hook", "claude-code"],
        &mut out,
        &mut err,
        payload.as_bytes(),
    );

    assert_eq!(code, 0, "unwritable store must still exit 0");
    let log = read_warning_log(cwd_dir.path(), "claude-code");
    assert!(
        log.contains("cannot open store") || log.contains("cannot append"),
        "warning log must record persistence failure, got: {log:?}"
    );
}

#[test]
fn unknown_harness_exits_0_and_persists_nothing() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    crate::test_support::with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();
        // A well-formed payload but a bogus harness.
        let payload = serde_json::json!({"cwd": ".", "tool_name": "Read"}).to_string();
        let code = run_with_io(["hook", "bogus"], &mut out, &mut err, payload.as_bytes());
        assert_eq!(code, 0, "unknown harness must fail open");
        assert!(out.is_empty());
    });
    let log = read_warning_log(dir.path(), "bogus");
    assert!(log.contains("unknown harness 'bogus'"), "got: {log:?}");
}

// --- 5.3 pi via --file -----------------------------------------------------

fn run_pi_file(store_dir: &std::path::Path, raw: &str) -> i32 {
    let tmp = store_dir.join("event.json");
    std::fs::write(&tmp, raw).unwrap_or_else(|e| panic!("write tmp: {e}"));
    let store_path = store_dir.join(".scryrs/scryrs.db");
    crate::store_override::set(
        store_path
            .to_str()
            .unwrap_or_else(|| panic!("store path not UTF-8"))
            .to_string(),
    );
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run_with_io(
        [
            "hook",
            "pi",
            "--file",
            tmp.to_str().unwrap_or_else(|| panic!("tmp path not UTF-8")),
        ],
        &mut out,
        &mut err,
        &b""[..],
    );
    assert!(out.is_empty(), "hook must write nothing to stdout");
    code
}

#[test]
fn pi_tool_result_maps_and_persists() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    let raw = serde_json::json!({
        "session_id": "pi-1",
        "toolName": "read",
        "input": {"path": "src/a.rs"},
        "isError": false,
    })
    .to_string();
    assert_eq!(run_pi_file(dir.path(), &raw), 0);

    let store_path = dir.path().join(".scryrs/scryrs.db");
    assert_eq!(event_count(&store_path), 1);
    let (tool_name, session_id, event_type, outcome) = single_row(&store_path);
    assert_eq!(tool_name.as_deref(), Some("read"));
    assert_eq!(session_id, "pi-1");
    assert_eq!(event_type, "FileOpened");
    assert_eq!(outcome, "Success");
}

#[test]
fn pi_is_error_yields_failure_outcome() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    let raw = serde_json::json!({
        "session_id": "pi-1",
        "toolName": "read",
        "input": {"path": "src/a.rs"},
        "isError": true,
    })
    .to_string();
    assert_eq!(run_pi_file(dir.path(), &raw), 0);
    let store_path = dir.path().join(".scryrs/scryrs.db");
    let (_, _, event_type, outcome) = single_row(&store_path);
    assert_eq!(event_type, "FileOpened");
    assert_eq!(outcome, "Failure");
}

#[test]
fn pi_lsp_navigation_success_and_failure_branches() {
    // success → SymbolInspected
    let dir1 = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    let ok = serde_json::json!({
        "session_id": "pi-1", "toolName": "lsp_navigation",
        "input": {"symbol": "Dispatcher"}, "isError": false,
    })
    .to_string();
    assert_eq!(run_pi_file(dir1.path(), &ok), 0);
    let (_, _, et, oc) = single_row(&dir1.path().join(".scryrs/scryrs.db"));
    assert_eq!(et, "SymbolInspected");
    assert_eq!(oc, "Success");

    // error → FailedLookup
    let dir2 = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    let bad = serde_json::json!({
        "session_id": "pi-1", "toolName": "lsp_navigation",
        "input": {"symbol": "Missing"}, "isError": true,
    })
    .to_string();
    assert_eq!(run_pi_file(dir2.path(), &bad), 0);
    let (_, _, et2, oc2) = single_row(&dir2.path().join(".scryrs/scryrs.db"));
    assert_eq!(et2, "FailedLookup");
    assert_eq!(oc2, "Failure");
}

#[test]
fn pi_session_start_persists_lifecycle_event() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    let raw = serde_json::json!({"session_id": "pi-1", "reason": "startup"}).to_string();
    assert_eq!(run_pi_file(dir.path(), &raw), 0);
    let store_path = dir.path().join(".scryrs/scryrs.db");
    assert_eq!(event_count(&store_path), 1);
    let (tool_name, _, event_type, _) = single_row(&store_path);
    assert_eq!(event_type, "SessionStart");
    assert!(tool_name.is_none(), "lifecycle event has no tool_name");
}
