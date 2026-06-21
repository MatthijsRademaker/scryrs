use scryrs_types::SCHEMA_VERSION;

use crate::run_with_io;

/// Set a thread-local store path override so tests don't pollute the
/// real CWD's .scryrs/scryrs.db. Returns the tempdir guard.
fn set_test_store() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    let store_path = dir.path().join("scryrs.db");
    crate::store_override::set(
        store_path
            .to_str()
            .unwrap_or_else(|| panic!("store path not valid UTF-8"))
            .to_string(),
    );
    dir
}

fn make_valid_event_json(session_id: &str, doc_ref: &str) -> String {
    format!(
        r#"{{"schema_version":"{}","timestamp":"2026-06-20T00:00:00Z","session_id":"{}","event_type":"DocRetrieved","tool_name":"read","payload":{{"type":"DocRetrieved","doc_ref":"{}"}},"outcome":{{"result":"Success"}}}}"#,
        SCHEMA_VERSION, session_id, doc_ref
    )
}

// --- stdin ingestion ---

#[test]
fn record_stdin_all_valid_exits_0() {
    let _store_dir = set_test_store();
    let input = format!(
        "{}\n{}\n",
        make_valid_event_json("s1", "doc/a.md"),
        make_valid_event_json("s2", "doc/b.md"),
    );
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(
        run_with_io(["record", "--stdin"], &mut out, &mut err, input.as_bytes(),),
        0
    );

    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("\"command\":\"record\""));
    assert!(stdout.contains("\"accepted\":2"));
    assert!(stdout.contains("\"rejected\":0"));
    assert!(err.is_empty());
}

#[test]
fn record_stdin_some_invalid_exits_1() {
    let _store_dir = set_test_store();
    let input = format!(
        "{}\nnot valid json\n{}\n",
        make_valid_event_json("s1", "doc/a.md"),
        make_valid_event_json("s2", "doc/b.md"),
    );
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(
        run_with_io(["record", "--stdin"], &mut out, &mut err, input.as_bytes(),),
        1
    );

    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("\"accepted\":2"));
    assert!(stdout.contains("\"rejected\":1"));

    let stderr = String::from_utf8_lossy(&err);
    assert!(stderr.contains("\"line\":2"));
    assert!(stderr.contains("\"reason\":"));
}

#[test]
fn record_stdin_blank_lines_are_skipped() {
    let _store_dir = set_test_store();
    let input = format!(
        "\n{}\n\n{}\n  \n",
        make_valid_event_json("s1", "doc/a.md"),
        make_valid_event_json("s2", "doc/b.md"),
    );
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(
        run_with_io(["record", "--stdin"], &mut out, &mut err, input.as_bytes(),),
        0
    );

    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("\"accepted\":2"));
    assert!(stdout.contains("\"rejected\":0"));
    assert!(err.is_empty());
}

// --- file ingestion ---

#[test]
fn record_file_all_valid_exits_0() {
    let _store_dir = set_test_store();
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    let file_path = dir.path().join("events.jsonl");
    let content = format!(
        "{}\n{}\n",
        make_valid_event_json("s1", "doc/a.md"),
        make_valid_event_json("s2", "doc/b.md"),
    );
    if let Err(e) = std::fs::write(&file_path, content) {
        panic!("write test file: {e}");
    }

    let mut out = Vec::new();
    let mut err = Vec::new();
    let stdin = &[] as &[u8];

    assert_eq!(
        run_with_io(
            ["record", "--file", &file_path.display().to_string()],
            &mut out,
            &mut err,
            stdin,
        ),
        0
    );

    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("\"accepted\":2"));
    assert!(err.is_empty());
}

#[test]
fn record_file_some_invalid_exits_1() {
    let _store_dir = set_test_store();
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    let file_path = dir.path().join("events.jsonl");
    let content = format!(
        "{}\nbad line\n{}\n",
        make_valid_event_json("s1", "doc/a.md"),
        make_valid_event_json("s2", "doc/b.md"),
    );
    if let Err(e) = std::fs::write(&file_path, content) {
        panic!("write test file: {e}");
    }

    let mut out = Vec::new();
    let mut err = Vec::new();
    let stdin = &[] as &[u8];

    assert_eq!(
        run_with_io(
            ["record", "--file", &file_path.display().to_string()],
            &mut out,
            &mut err,
            stdin,
        ),
        1
    );

    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("\"accepted\":2"));
    assert!(stdout.contains("\"rejected\":1"));

    let stderr = String::from_utf8_lossy(&err);
    assert!(stderr.contains("\"line\":2"));
}

// --- mutually exclusive input modes ---

#[test]
fn record_both_modes_exits_2() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let stdin = &[] as &[u8];

    assert_eq!(
        run_with_io(
            ["record", "--stdin", "--file", "some.jsonl"],
            &mut out,
            &mut err,
            stdin,
        ),
        2
    );

    let stderr = String::from_utf8_lossy(&err);
    assert!(stderr.contains("mutually exclusive"));
    assert!(stderr.contains("See `scryrs --help`"));
}

#[test]
fn record_neither_mode_exits_2() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let stdin = &[] as &[u8];

    assert_eq!(run_with_io(["record"], &mut out, &mut err, stdin,), 2);

    let stderr = String::from_utf8_lossy(&err);
    assert!(stderr.contains("must specify one of --stdin or --file"));
    assert!(stderr.contains("See `scryrs --help`"));
}

// --- unreadable file ---

#[test]
fn record_unreadable_file_exits_2() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let stdin = &[] as &[u8];

    assert_eq!(
        run_with_io(
            ["record", "--file", "/nonexistent/path/events.jsonl"],
            &mut out,
            &mut err,
            stdin,
        ),
        2
    );

    let stderr = String::from_utf8_lossy(&err);
    assert!(stderr.contains("cannot read"));
    assert!(stderr.contains("See `scryrs --help`"));
}

// --- deterministic output ---

#[test]
fn record_output_is_valid_json() {
    let _store_dir = set_test_store();
    let input = make_valid_event_json("s1", "doc/x.md");
    let mut out = Vec::new();
    let mut err = Vec::new();

    run_with_io(["record", "--stdin"], &mut out, &mut err, input.as_bytes());

    let out_str = String::from_utf8_lossy(&out);
    let summary: serde_json::Value = match serde_json::from_str(out_str.trim()) {
        Ok(v) => v,
        Err(e) => panic!("stdout must be valid JSON: {e}"),
    };
    assert_eq!(summary["command"], "record");
    assert_eq!(summary["accepted"], 1);
    assert_eq!(summary["rejected"], 0);
}

#[test]
fn record_rejection_diagnostics_are_valid_json() {
    let _store_dir = set_test_store();
    let input = "not valid json\n";
    let mut out = Vec::new();
    let mut err = Vec::new();

    run_with_io(["record", "--stdin"], &mut out, &mut err, input.as_bytes());

    let stderr = String::from_utf8_lossy(&err);
    let diag: serde_json::Value = match serde_json::from_str(stderr.trim()) {
        Ok(v) => v,
        Err(e) => panic!("stderr must be valid JSON: {e}"),
    };
    assert_eq!(diag["line"], 1);
    assert!(diag["field"].is_null());
    assert!(diag["reason"].is_string());
}

#[test]
fn record_multiple_rejections_all_on_stderr() {
    let _store_dir = set_test_store();
    let input = "bad1\nbad2\nbad3\n";
    let mut out = Vec::new();
    let mut err = Vec::new();

    run_with_io(["record", "--stdin"], &mut out, &mut err, input.as_bytes());

    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("\"accepted\":0"));
    assert!(stdout.contains("\"rejected\":3"));

    let stderr = String::from_utf8_lossy(&err);
    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(lines.len(), 3, "must have 3 rejection diagnostics");
    for (i, line) in lines.iter().enumerate() {
        let diag: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => panic!("each stderr line must be valid JSON: {e}"),
        };
        assert_eq!(diag["line"], (i + 1) as u64);
    }
}

// --- invalid arguments to record (error handler disambiguates subcommands) ---

#[test]
fn record_help_flag_exits_2() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let stdin = &[] as &[u8];

    assert_eq!(
        run_with_io(["record", "--help"], &mut out, &mut err, stdin),
        2,
        "record --help must exit 2 (help flag disabled on subcommand)"
    );
    assert!(out.is_empty());
    let err_str = String::from_utf8_lossy(&err);
    assert!(
        err_str.contains("scryrs record:"),
        "must name record, not hotspots, got: {err_str}"
    );
    assert!(
        err_str.contains("unexpected argument"),
        "must report unexpected argument, got: {err_str}"
    );
    assert!(
        err_str.contains("See `scryrs --help`"),
        "must escalate to --help, got: {err_str}"
    );
}

#[test]
fn record_version_flag_exits_2() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let stdin = &[] as &[u8];

    assert_eq!(
        run_with_io(["record", "--version"], &mut out, &mut err, stdin),
        2,
        "record --version must exit 2 (version flag disabled on subcommand)"
    );
    assert!(out.is_empty());
    let err_str = String::from_utf8_lossy(&err);
    assert!(
        err_str.contains("scryrs record:"),
        "must name record, not hotspots, got: {err_str}"
    );
}

#[test]
fn record_unknown_flag_exits_2() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let stdin = &[] as &[u8];

    assert_eq!(
        run_with_io(["record", "--unknown-flag"], &mut out, &mut err, stdin),
        2
    );
    assert!(out.is_empty());
    let err_str = String::from_utf8_lossy(&err);
    assert!(
        err_str.contains("scryrs record:"),
        "must name record, not hotspots, got: {err_str}"
    );
    assert!(
        err_str.contains("unexpected argument"),
        "must report unexpected argument, got: {err_str}"
    );
}

#[test]
fn record_extra_args_exits_2() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let stdin = &[] as &[u8];

    assert_eq!(
        run_with_io(["record", "--stdin", "extra"], &mut out, &mut err, stdin,),
        2
    );
    let err_str = String::from_utf8_lossy(&err);
    assert!(
        err_str.contains("scryrs record:"),
        "must name record, not hotspots, got: {err_str}"
    );
}

// --- SQLite store integration (Tasks 2.2, 2.4) ---

#[test]
fn record_persists_to_scryrs_db() {
    // Use set_test_store to override the store path to a temp db.
    let _store_dir = set_test_store();
    let input = make_valid_event_json("s1", "doc/x.md");
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(
        run_with_io(["record", "--stdin"], &mut out, &mut err, input.as_bytes()),
        0
    );

    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("\"accepted\":1"));
    assert!(stdout.contains("\"rejected\":0"));
}

#[test]
fn record_does_not_create_events_jsonl() {
    // Prove that canonical persistence uses the SQLite store, not JSONL.
    // The store_override sends writes to a temp scryrs.db; .scryrs/events.jsonl
    // should never be touched in the test temp dir.
    let dir = set_test_store();
    let override_path = dir.path().join("scryrs.db");
    assert!(
        crate::store_override::get()
            .map(|p| p.contains("scryrs.db"))
            .unwrap_or(false),
        "override path should point to scryrs.db"
    );

    let input = make_valid_event_json("s1", "doc/x.md");
    let mut out = Vec::new();
    let mut err = Vec::new();

    let _exit_code = run_with_io(["record", "--stdin"], &mut out, &mut err, input.as_bytes());

    // The SQLite store should be created at the override path.
    assert!(
        override_path.exists(),
        "scryrs.db must be created at override path"
    );
    // No events.jsonl should be created alongside.
    assert!(
        !dir.path().join("events.jsonl").exists(),
        "events.jsonl must NOT be created"
    );
}

#[test]
fn record_default_path_uses_canonical_db() {
    // If no store_override is set (empty matches nothing), the fallback
    // should be CANONICAL_STORE_PATH.
    let fallback: String = crate::store_override::get()
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| scryrs_core::CANONICAL_STORE_PATH.into());
    assert_eq!(fallback, scryrs_core::CANONICAL_STORE_PATH);
}

// --- 2.1: --stdin SQLite row-level verification ---

#[test]
fn record_stdin_persists_rows_to_sqlite() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    let store_path = dir.path().join("test.db");
    crate::store_override::set(
        store_path
            .to_str()
            .unwrap_or_else(|| panic!("store path not valid UTF-8"))
            .to_string(),
    );

    let input = format!(
        "{}\n{}\n",
        make_valid_event_json("s1", "doc/a.md"),
        make_valid_event_json("s2", "doc/b.md"),
    );
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(
        run_with_io(["record", "--stdin"], &mut out, &mut err, input.as_bytes()),
        0
    );
    assert!(
        err.is_empty(),
        "stderr must be empty: {:?}",
        String::from_utf8_lossy(&err)
    );

    // Re-open the SQLite store and assert rows.
    let conn = rusqlite::Connection::open(&store_path).unwrap_or_else(|e| panic!("reopen db: {e}"));
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM trace_events", [], |row| row.get(0))
        .unwrap_or_else(|e| panic!("count query: {e}"));
    assert_eq!(count, 2, "two events must be persisted");

    // Verify session_ids are correct.
    let mut stmt = conn
        .prepare("SELECT session_id FROM trace_events ORDER BY rowid")
        .unwrap_or_else(|e| panic!("prepare: {e}"));
    let sessions: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .unwrap_or_else(|e| panic!("query_map: {e}"))
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|e| panic!("collect: {e}"));
    assert_eq!(sessions, vec!["s1", "s2"]);
}

// --- 2.2: --file SQLite row-level verification ---

#[test]
fn record_file_persists_rows_to_sqlite() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    let store_path = dir.path().join("test.db");
    crate::store_override::set(
        store_path
            .to_str()
            .unwrap_or_else(|| panic!("store path not valid UTF-8"))
            .to_string(),
    );

    let input_dir = tempfile::tempdir().unwrap_or_else(|e| panic!("input dir: {e}"));
    let file_path = input_dir.path().join("events.jsonl");
    let content = format!(
        "{}\n{}\n",
        make_valid_event_json("s1", "doc/x.md"),
        make_valid_event_json("s2", "doc/y.md"),
    );
    std::fs::write(&file_path, content).unwrap_or_else(|e| panic!("write: {e}"));

    let mut out = Vec::new();
    let mut err = Vec::new();
    let stdin = &[] as &[u8];

    assert_eq!(
        run_with_io(
            ["record", "--file", &file_path.display().to_string()],
            &mut out,
            &mut err,
            stdin,
        ),
        0
    );
    assert!(
        err.is_empty(),
        "stderr must be empty: {:?}",
        String::from_utf8_lossy(&err)
    );

    // Re-open the same canonical store and assert rows.
    let conn = rusqlite::Connection::open(&store_path).unwrap_or_else(|e| panic!("reopen db: {e}"));
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM trace_events", [], |row| row.get(0))
        .unwrap_or_else(|e| panic!("count query: {e}"));
    assert_eq!(count, 2, "two events must be persisted from file");

    let mut stmt = conn
        .prepare("SELECT session_id FROM trace_events ORDER BY rowid")
        .unwrap_or_else(|e| panic!("prepare: {e}"));
    let sessions: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .unwrap_or_else(|e| panic!("query_map: {e}"))
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|e| panic!("collect: {e}"));
    assert_eq!(sessions, vec!["s1", "s2"]);
}

// --- 2.3: Mixed valid/invalid — rejected lines never create rows ---

#[test]
fn mixed_valid_invalid_rejected_lines_no_rows() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    let store_path = dir.path().join("test.db");
    crate::store_override::set(
        store_path
            .to_str()
            .unwrap_or_else(|| panic!("store path not valid UTF-8"))
            .to_string(),
    );

    let input = format!(
        "{}\nnot valid json\n{}\n",
        make_valid_event_json("s1", "doc/a.md"),
        make_valid_event_json("s2", "doc/b.md"),
    );
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(
        run_with_io(["record", "--stdin"], &mut out, &mut err, input.as_bytes()),
        1
    );

    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("\"accepted\":2"));
    assert!(stdout.contains("\"rejected\":1"));

    // Rejected line must produce a diagnostic on stderr.
    let stderr = String::from_utf8_lossy(&err);
    assert!(stderr.contains("\"line\":2"), "must diagnose line 2");

    // Open SQLite — only 2 accepted rows must exist.
    let conn = rusqlite::Connection::open(&store_path).unwrap_or_else(|e| panic!("reopen db: {e}"));
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM trace_events", [], |row| row.get(0))
        .unwrap_or_else(|e| panic!("count query: {e}"));
    assert_eq!(count, 2, "only accepted events must be persisted");

    // No .scryrs/events.jsonl was written as canonical output.
    assert!(
        !dir.path().join(".scryrs/events.jsonl").exists(),
        ".scryrs/events.jsonl must not be created"
    );
}

// --- 2.4: Fatal datastore failure ---

#[test]
fn record_fatal_store_failure_exits_2() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    // Create ".scryrs" as a regular FILE (not a directory) so that
    // EventStore::open cannot create scryrs.db inside it — simulating
    // a fatal filesystem-level failure.
    std::fs::write(dir.path().join(".scryrs"), "blocked")
        .unwrap_or_else(|e| panic!("write blocker: {e}"));
    let store_path = dir.path().join(".scryrs/scryrs.db");
    crate::store_override::set(
        store_path
            .to_str()
            .unwrap_or_else(|| panic!("store path not valid UTF-8"))
            .to_string(),
    );

    let input = make_valid_event_json("s1", "doc/x.md");
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(
        run_with_io(["record", "--stdin"], &mut out, &mut err, input.as_bytes()),
        2,
        "fatal store failure must exit 2"
    );

    // No success summary on stdout.
    let stdout = String::from_utf8_lossy(&out);
    assert!(
        !stdout.contains("\"accepted\""),
        "stdout must not contain success summary, got: {stdout}"
    );

    // Deterministic stderr diagnostic.
    let stderr = String::from_utf8_lossy(&err);
    assert!(
        stderr.contains("scryrs record: cannot open trace datastore"),
        "stderr must report store failure, got: {stderr}"
    );
}
