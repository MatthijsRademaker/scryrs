use crate::*;
use scryrs_core::EventStore;
use scryrs_types::{
    FileOpenedPayload, Outcome, SCHEMA_VERSION, TraceEvent, TraceEventPayload, TraceEventType,
};

fn make_file_opened(session_id: &str, path: &str, timestamp: &str) -> TraceEvent {
    TraceEvent {
        schema_version: SCHEMA_VERSION.into(),
        timestamp: timestamp.into(),
        session_id: session_id.into(),
        event_type: TraceEventType::FileOpened,
        tool_name: Some("read".into()),
        payload: TraceEventPayload::FileOpened(FileOpenedPayload { path: path.into() }),
        outcome: Outcome::Success,
    }
}

fn make_search_run(session_id: &str, query: &str, timestamp: &str) -> TraceEvent {
    use scryrs_types::SearchRunPayload;
    TraceEvent {
        schema_version: SCHEMA_VERSION.into(),
        timestamp: timestamp.into(),
        session_id: session_id.into(),
        event_type: TraceEventType::SearchRun,
        tool_name: Some("search".into()),
        payload: TraceEventPayload::SearchRun(SearchRunPayload {
            query: query.into(),
        }),
        outcome: Outcome::Success,
    }
}

fn make_symbol_inspected(session_id: &str, name: &str, timestamp: &str) -> TraceEvent {
    use scryrs_types::SymbolInspectedPayload;
    TraceEvent {
        schema_version: SCHEMA_VERSION.into(),
        timestamp: timestamp.into(),
        session_id: session_id.into(),
        event_type: TraceEventType::SymbolInspected,
        tool_name: Some("inspect".into()),
        payload: TraceEventPayload::SymbolInspected(SymbolInspectedPayload { name: name.into() }),
        outcome: Outcome::Success,
    }
}

fn make_command_executed(
    session_id: &str,
    command: &str,
    timestamp: &str,
    outcome: Outcome,
) -> TraceEvent {
    use scryrs_types::CommandExecutedPayload;
    TraceEvent {
        schema_version: SCHEMA_VERSION.into(),
        timestamp: timestamp.into(),
        session_id: session_id.into(),
        event_type: TraceEventType::CommandExecuted,
        tool_name: Some("bash".into()),
        payload: TraceEventPayload::CommandExecuted(CommandExecutedPayload {
            command: command.into(),
        }),
        outcome,
    }
}

fn make_doc_retrieved(session_id: &str, doc_ref: &str, timestamp: &str) -> TraceEvent {
    use scryrs_types::DocRetrievedPayload;
    TraceEvent {
        schema_version: SCHEMA_VERSION.into(),
        timestamp: timestamp.into(),
        session_id: session_id.into(),
        event_type: TraceEventType::DocRetrieved,
        tool_name: Some("read".into()),
        payload: TraceEventPayload::DocRetrieved(DocRetrievedPayload {
            doc_ref: doc_ref.into(),
        }),
        outcome: Outcome::Success,
    }
}

fn make_edit_made(session_id: &str, target: &str, timestamp: &str, outcome: Outcome) -> TraceEvent {
    use scryrs_types::EditMadePayload;
    TraceEvent {
        schema_version: SCHEMA_VERSION.into(),
        timestamp: timestamp.into(),
        session_id: session_id.into(),
        event_type: TraceEventType::EditMade,
        tool_name: Some("edit".into()),
        payload: TraceEventPayload::EditMade(EditMadePayload {
            target: target.into(),
        }),
        outcome,
    }
}

fn make_failed_lookup(
    session_id: &str,
    subject: &str,
    reason: &str,
    timestamp: &str,
) -> TraceEvent {
    use scryrs_types::FailedLookupPayload;
    TraceEvent {
        schema_version: SCHEMA_VERSION.into(),
        timestamp: timestamp.into(),
        session_id: session_id.into(),
        event_type: TraceEventType::FailedLookup,
        tool_name: Some("search".into()),
        payload: TraceEventPayload::FailedLookup(FailedLookupPayload {
            subject: subject.into(),
        }),
        outcome: Outcome::Failure {
            reason: Some(reason.into()),
        },
    }
}

fn populate_store(dir: &tempfile::TempDir, events: &[TraceEvent]) {
    let scryrs_dir = dir.path().join(".scryrs");
    std::fs::create_dir_all(&scryrs_dir).unwrap_or_else(|e| panic!("create .scryrs: {e}"));
    let store_path = scryrs_dir.join("scryrs.db");
    {
        let mut store = EventStore::open(&store_path).unwrap_or_else(|e| panic!("open store: {e}"));
        store
            .begin_transaction()
            .unwrap_or_else(|e| panic!("begin: {e}"));
        for ev in events {
            store
                .append(ev)
                .unwrap_or_else(|e| panic!("append {ev:?}: {e}"));
        }
        store
            .commit_transaction()
            .unwrap_or_else(|e| panic!("commit: {e}"));
    }
}

// 3.5.1: Populated store produces correct HotspotsReport JSON.
#[test]
fn populated_store_produces_correct_hotspots_report() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    let events = vec![
        make_file_opened("s1", "src/a.rs", "2026-06-21T09:00:00Z"),
        make_file_opened("s1", "src/b.rs", "2026-06-21T09:01:00Z"),
        make_file_opened("s2", "src/a.rs", "2026-06-21T09:02:00Z"),
    ];
    populate_store(&dir, &events);

    let mut out = Vec::new();
    let mut err = Vec::new();

    let exit_code = run_with_writers(
        ["hotspots", &dir.path().display().to_string()],
        &mut out,
        &mut err,
    );

    assert_eq!(exit_code, 0, "stderr: {:?}", String::from_utf8_lossy(&err));

    let stdout = String::from_utf8_lossy(&out);
    let report: serde_json::Value =
        serde_json::from_str(stdout.trim()).unwrap_or_else(|e| panic!("parse JSON: {e}"));

    assert_eq!(report["schemaVersion"], "1.0.0");
    assert_eq!(report["command"], "hotspots");
    assert!(!report["repositoryPath"].as_str().unwrap_or("").is_empty());
    assert!(
        report["storePath"]
            .as_str()
            .unwrap_or("")
            .ends_with(".scryrs/scryrs.db")
    );
    assert_eq!(report["runMetadata"]["analyzedEventCount"], 3);
    assert_eq!(report["runMetadata"]["analyzedSubjectCount"], 2);
    assert!(!report["generatedAt"].as_str().unwrap_or("").is_empty());

    let entries = report["entries"]
        .as_array()
        .unwrap_or_else(|| panic!("entries not array"));
    assert!(!entries.is_empty(), "entries should not be empty");

    // src/a.rs should have higher score (2) than src/b.rs (1)
    assert_eq!(entries[0]["subject"], "src/a.rs");
    assert_eq!(entries[0]["score"], 2);
    assert_eq!(entries[1]["subject"], "src/b.rs");
    assert_eq!(entries[1]["score"], 1);
}

// 3.5.2: Empty store produces entries: [] with exit 0.
#[test]
fn empty_store_produces_empty_entries_with_exit_0() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    // Create empty store with no events.
    populate_store(&dir, &[]);

    let mut out = Vec::new();
    let mut err = Vec::new();

    let exit_code = run_with_writers(
        ["hotspots", &dir.path().display().to_string()],
        &mut out,
        &mut err,
    );

    assert_eq!(
        exit_code,
        0,
        "exit should be 0 for empty store, stderr: {:?}",
        String::from_utf8_lossy(&err)
    );

    let stdout = String::from_utf8_lossy(&out);
    let report: serde_json::Value =
        serde_json::from_str(stdout.trim()).unwrap_or_else(|e| panic!("parse JSON: {e}"));

    assert_eq!(
        report["entries"]
            .as_array()
            .unwrap_or_else(|| panic!("entries not array"))
            .len(),
        0
    );
    assert_eq!(report["runMetadata"]["analyzedEventCount"], 0);
    assert_eq!(report["runMetadata"]["analyzedSubjectCount"], 0);
}

// 3.5.3: Missing store exits 2 with error message on stderr.
#[test]
fn missing_store_exits_2_with_error() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    // Don't create .scryrs/scryrs.db.

    let mut out = Vec::new();
    let mut err = Vec::new();

    let exit_code = run_with_writers(
        ["hotspots", &dir.path().display().to_string()],
        &mut out,
        &mut err,
    );

    assert_eq!(exit_code, 2);
    assert!(out.is_empty());

    let stderr = String::from_utf8_lossy(&err);
    assert!(
        stderr.contains("datastore not found"),
        "should mention datastore not found, got: {stderr}"
    );
}

// 3.5.4: Unsupported store (schema version mismatch) exits 2 with error on stderr.
#[test]
fn unsupported_store_exits_2_with_error() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    // Create a valid store first.
    populate_store(
        &dir,
        &[make_file_opened("s1", "src/a.rs", "2026-06-21T09:00:00Z")],
    );

    // Tamper with the schema version via direct SQLite connection.
    let store_path = dir.path().join(".scryrs/scryrs.db");
    {
        let conn = rusqlite::Connection::open(&store_path)
            .unwrap_or_else(|e| panic!("open store for tamper: {e}"));
        conn.execute(
            "UPDATE schema_meta SET value = '99' WHERE key = 'datastore_schema_version'",
            [],
        )
        .unwrap_or_else(|e| panic!("tamper schema version: {e}"));
    }

    let mut out = Vec::new();
    let mut err = Vec::new();

    let exit_code = run_with_writers(
        ["hotspots", &dir.path().display().to_string()],
        &mut out,
        &mut err,
    );

    assert_eq!(exit_code, 2, "stderr: {:?}", String::from_utf8_lossy(&err));
    assert!(out.is_empty());

    let stderr = String::from_utf8_lossy(&err);
    assert!(
        stderr.contains("unsupported datastore"),
        "should mention unsupported datastore, got: {stderr}"
    );
    assert!(
        stderr.contains("version mismatch"),
        "should mention version mismatch, got: {stderr}"
    );
}

// 3.5.5: Corrupt/non-SQLite file exits 1 with error message on stderr.
#[test]
fn corrupt_store_exits_1_with_error() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    let scryrs_dir = dir.path().join(".scryrs");
    std::fs::create_dir_all(&scryrs_dir).unwrap_or_else(|e| panic!("create .scryrs: {e}"));
    std::fs::write(scryrs_dir.join("scryrs.db"), "not a sqlite database\n")
        .unwrap_or_else(|e| panic!("write corrupt file: {e}"));

    let mut out = Vec::new();
    let mut err = Vec::new();

    let exit_code = run_with_writers(
        ["hotspots", &dir.path().display().to_string()],
        &mut out,
        &mut err,
    );

    assert_eq!(exit_code, 1, "stderr: {:?}", String::from_utf8_lossy(&err));
    assert!(out.is_empty());

    let stderr = String::from_utf8_lossy(&err);
    assert!(
        stderr.contains("storage error"),
        "should mention storage error, got: {stderr}"
    );
}

// 3.5.6: Deterministic ordering: same store produces identical output on repeated runs
// (ignoring the generatedAt wall-clock timestamp which may differ).
#[test]
fn deterministic_ordering_repeated_runs() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    let events = vec![
        make_file_opened("s1", "src/a.rs", "2026-06-21T09:00:00Z"),
        make_file_opened("s1", "src/b.rs", "2026-06-21T09:01:00Z"),
    ];
    populate_store(&dir, &events);

    let mut out1 = Vec::new();
    let mut err1 = Vec::new();
    let exit1 = run_with_writers(
        ["hotspots", &dir.path().display().to_string()],
        &mut out1,
        &mut err1,
    );

    let mut out2 = Vec::new();
    let mut err2 = Vec::new();
    let exit2 = run_with_writers(
        ["hotspots", &dir.path().display().to_string()],
        &mut out2,
        &mut err2,
    );

    assert_eq!(exit1, 0);
    assert_eq!(exit2, 0);

    // Parse both outputs and compare everything except generatedAt.
    let stdout1 = String::from_utf8_lossy(&out1);
    let stdout2 = String::from_utf8_lossy(&out2);
    let mut r1: serde_json::Value =
        serde_json::from_str(stdout1.trim()).unwrap_or_else(|e| panic!("parse 1: {e}"));
    let mut r2: serde_json::Value =
        serde_json::from_str(stdout2.trim()).unwrap_or_else(|e| panic!("parse 2: {e}"));

    // Strip generatedAt before comparing.
    r1.as_object_mut()
        .unwrap_or_else(|| panic!("r1 not object"))
        .remove("generatedAt");
    r2.as_object_mut()
        .unwrap_or_else(|| panic!("r2 not object"))
        .remove("generatedAt");

    assert_eq!(
        r1, r2,
        "repeated runs must produce identical deterministic output"
    );
}

// 3.5.8: .scryrs/hotspots.json artifact file is written when store is valid.
#[test]
fn artifact_file_is_written_on_success() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    let events = vec![make_file_opened("s1", "src/a.rs", "2026-06-21T09:00:00Z")];
    populate_store(&dir, &events);

    let mut out = Vec::new();
    let mut err = Vec::new();

    let exit_code = run_with_writers(
        ["hotspots", &dir.path().display().to_string()],
        &mut out,
        &mut err,
    );

    assert_eq!(exit_code, 0);

    let artifact_path = dir.path().join(".scryrs/hotspots.json");
    assert!(
        artifact_path.exists(),
        "hotspots.json should be written at {}",
        artifact_path.display()
    );

    let artifact_content =
        std::fs::read_to_string(&artifact_path).unwrap_or_else(|e| panic!("read artifact: {e}"));
    let stdout = String::from_utf8_lossy(&out);
    assert_eq!(
        stdout.trim(),
        artifact_content.trim(),
        "stdout and artifact file must match"
    );
}

// Additional: empty store also writes artifact file.
#[test]
fn artifact_file_is_written_for_empty_store() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    populate_store(&dir, &[]);

    let mut out = Vec::new();
    let mut err = Vec::new();

    let exit_code = run_with_writers(
        ["hotspots", &dir.path().display().to_string()],
        &mut out,
        &mut err,
    );

    assert_eq!(exit_code, 0);

    let artifact_path = dir.path().join(".scryrs/hotspots.json");
    assert!(artifact_path.exists());
}

// --- 3.1: Lifecycle-only store exits 0 with entries: [] and zero subject-bearing events ---

#[test]
fn lifecycle_only_store_produces_empty_entries_with_exit_0() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    use scryrs_types::{SessionEndPayload, SessionStartPayload};
    let events = vec![
        TraceEvent {
            schema_version: SCHEMA_VERSION.into(),
            timestamp: "2026-06-21T09:00:00Z".into(),
            session_id: "s-life".into(),
            event_type: TraceEventType::SessionStart,
            tool_name: None,
            payload: TraceEventPayload::SessionStart(SessionStartPayload),
            outcome: Outcome::Success,
        },
        TraceEvent {
            schema_version: SCHEMA_VERSION.into(),
            timestamp: "2026-06-21T09:01:00Z".into(),
            session_id: "s-life".into(),
            event_type: TraceEventType::SessionEnd,
            tool_name: None,
            payload: TraceEventPayload::SessionEnd(SessionEndPayload),
            outcome: Outcome::Success,
        },
    ];
    populate_store(&dir, &events);

    let mut out = Vec::new();
    let mut err = Vec::new();

    let exit_code = run_with_writers(
        ["hotspots", &dir.path().display().to_string()],
        &mut out,
        &mut err,
    );

    assert_eq!(
        exit_code,
        0,
        "exit should be 0 for lifecycle-only store, stderr: {:?}",
        String::from_utf8_lossy(&err)
    );

    let stdout = String::from_utf8_lossy(&out);
    let report: serde_json::Value =
        serde_json::from_str(stdout.trim()).unwrap_or_else(|e| panic!("parse JSON: {e}"));

    assert_eq!(
        report["entries"]
            .as_array()
            .unwrap_or_else(|| panic!("entries not array"))
            .len(),
        0
    );
    assert_eq!(report["runMetadata"]["analyzedEventCount"], 0);
    assert_eq!(report["runMetadata"]["analyzedSubjectCount"], 0);
    // firstEventId/lastEventId are 0 when no subject-bearing events exist.
    assert_eq!(report["runMetadata"]["firstEventId"], 0);
    assert_eq!(report["runMetadata"]["lastEventId"], 0);
    assert!(!report["generatedAt"].as_str().unwrap_or("").is_empty());
}

// --- 3.2: Non-monotonic timestamp/id deterministic ordering ---

#[test]
fn non_monotonic_timestamp_id_deterministic_ordering() {
    // Events inserted in non-monotonic timestamp order:
    // id=1 ts=09:00:03, id=2 ts=09:00:01, id=3 ts=09:00:02
    // TraceQuery orders by timestamp ASC, id ASC → id=2, id=3, id=1
    // So evidence.rowIds should be [2, 3, 1].
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    let events = vec![
        make_file_opened("s1", "src/c.rs", "2026-06-21T09:00:03Z"),
        make_file_opened("s1", "src/a.rs", "2026-06-21T09:00:01Z"),
        make_file_opened("s1", "src/b.rs", "2026-06-21T09:00:02Z"),
    ];
    populate_store(&dir, &events);

    let mut out = Vec::new();
    let mut err = Vec::new();

    let exit_code = run_with_writers(
        ["hotspots", &dir.path().display().to_string()],
        &mut out,
        &mut err,
    );

    assert_eq!(exit_code, 0);

    let stdout = String::from_utf8_lossy(&out);
    let report: serde_json::Value =
        serde_json::from_str(stdout.trim()).unwrap_or_else(|e| panic!("parse JSON: {e}"));

    let entries = report["entries"]
        .as_array()
        .unwrap_or_else(|| panic!("entries not array"));
    assert_eq!(entries.len(), 3);

    // Order: a.rs (earliest timestamp), b.rs, c.rs (all score 1, same session → lastSeen DESC)
    // a.rs lastSeen=09:00:01, b.rs=09:00:02, c.rs=09:00:03 → c.rs first, b.rs second, a.rs third
    assert_eq!(entries[0]["subject"], "src/c.rs");
    assert_eq!(entries[1]["subject"], "src/b.rs");
    assert_eq!(entries[2]["subject"], "src/a.rs");

    // Evidence rowIds: TraceQuery orders by timestamp ASC, id ASC
    // So for src/c.rs (id=1): when appearing in loop, it comes after id 2 and 3 in query order.
    // Actually each subject has only one event, so evidence.rowIds is single-element.
    // a.rs: event with ts=09:00:01 is id=2 → rowIds=[2]
    // b.rs: event with ts=09:00:02 is id=3 → rowIds=[3]
    // c.rs: event with ts=09:00:03 is id=1 → rowIds=[1]
    assert_eq!(
        entries[0]["evidence"]["rowIds"]
            .as_array()
            .unwrap_or_else(|| panic!("rowIds not array"))
            .len(),
        1
    );
    assert_eq!(
        entries[1]["evidence"]["rowIds"]
            .as_array()
            .unwrap_or_else(|| panic!("rowIds not array"))
            .len(),
        1
    );
    assert_eq!(
        entries[2]["evidence"]["rowIds"]
            .as_array()
            .unwrap_or_else(|| panic!("rowIds not array"))
            .len(),
        1
    );

    // firstEventId/lastEventId should be min/max of subject-bearing row IDs (1,2,3) → min=1, max=3
    assert_eq!(report["runMetadata"]["firstEventId"], 1);
    assert_eq!(report["runMetadata"]["lastEventId"], 3);
}

// See scoring.rs unit tests for the first-in-evidence-order tie-break verification
// (non-monotonic row-id edge case: first row in evidence order, not min, is used).

// Additional: artifact file is not written on error.
#[test]
fn artifact_file_not_written_on_error() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    let scryrs_dir = dir.path().join(".scryrs");
    std::fs::create_dir_all(&scryrs_dir).unwrap_or_else(|e| panic!("create .scryrs: {e}"));
    std::fs::write(scryrs_dir.join("scryrs.db"), "corrupt\n")
        .unwrap_or_else(|e| panic!("write: {e}"));

    let mut out = Vec::new();
    let mut err = Vec::new();

    let _exit_code = run_with_writers(
        ["hotspots", &dir.path().display().to_string()],
        &mut out,
        &mut err,
    );

    let artifact_path = dir.path().join(".scryrs/hotspots.json");
    assert!(
        !artifact_path.exists(),
        "hotspots.json must not be written on error"
    );
}

// --- 3.3: Byte-for-byte artifact equality ---

#[test]
fn artifact_matches_stdout_byte_for_byte() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    let events = vec![make_file_opened("s1", "src/x.rs", "2026-06-21T09:00:00Z")];
    populate_store(&dir, &events);

    let mut out = Vec::new();
    let mut err = Vec::new();

    let exit_code = run_with_writers(
        ["hotspots", &dir.path().display().to_string()],
        &mut out,
        &mut err,
    );

    assert_eq!(exit_code, 0);

    let artifact_path = dir.path().join(".scryrs/hotspots.json");
    assert!(artifact_path.exists());
    let artifact_bytes =
        std::fs::read(&artifact_path).unwrap_or_else(|e| panic!("read artifact: {e}"));
    // Strip trailing newline from stdout for comparison (writeln! adds one).
    let stdout_bytes = out.trim_ascii_end().to_vec();
    assert_eq!(
        artifact_bytes, stdout_bytes,
        "artifact file must match stdout byte-for-byte"
    );
}

#[test]
fn empty_store_artifact_matches_stdout_byte_for_byte() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    populate_store(&dir, &[]);

    let mut out = Vec::new();
    let mut err = Vec::new();

    let exit_code = run_with_writers(
        ["hotspots", &dir.path().display().to_string()],
        &mut out,
        &mut err,
    );

    assert_eq!(exit_code, 0);

    let artifact_path = dir.path().join(".scryrs/hotspots.json");
    assert!(artifact_path.exists());
    let artifact_bytes =
        std::fs::read(&artifact_path).unwrap_or_else(|e| panic!("read artifact: {e}"));
    let stdout_bytes = out.trim_ascii_end().to_vec();
    assert_eq!(
        artifact_bytes, stdout_bytes,
        "artifact file must match stdout byte-for-byte for empty store"
    );
}

// --- 3.4: Artifact write failure exits non-zero with stderr ---

#[test]
fn artifact_write_failure_exits_1_with_stderr_populated() {
    // Make .scryrs a regular file so .scryrs/hotspots.json cannot be created.
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    let events = vec![make_file_opened("s1", "src/a.rs", "2026-06-21T09:00:00Z")];
    populate_store(&dir, &events);
    // Replace .scryrs directory with a regular file.
    let scryrs_path = dir.path().join(".scryrs");
    std::fs::remove_dir_all(&scryrs_path).unwrap_or_else(|e| panic!("remove .scryrs dir: {e}"));
    // Recreate only the db file (no .scryrs directory).
    std::fs::create_dir_all(&scryrs_path).unwrap_or_else(|e| panic!("recreate .scryrs dir: {e}"));
    let events2 = vec![make_file_opened("s2", "src/a.rs", "2026-06-21T09:01:00Z")];
    populate_store(&dir, &events2);
    // Now replace .scryrs with a regular file so hotspots.json write fails.
    let _db_path = scryrs_path.join("scryrs.db");
    // We can't easily make the dir itself a file while keeping the db accessible.
    // Instead, make the artifact path unwritable by making .scryrs a file.
    // But TraceQuery::open needs .scryrs/scryrs.db to exist.
    //
    // Alternative: open .scryrs/hotspots.json as a read-only file before running.
    // Or make the parent dir read-only.
    //
    // Best approach: create .scryrs/hotspots.json as a directory so file write fails.
    let artifact_path = scryrs_path.join("hotspots.json");
    std::fs::create_dir(&artifact_path)
        .unwrap_or_else(|e| panic!("create hotspots.json as dir: {e}"));

    let mut out = Vec::new();
    let mut err = Vec::new();

    let exit_code = run_with_writers(
        ["hotspots", &dir.path().display().to_string()],
        &mut out,
        &mut err,
    );

    assert_eq!(
        exit_code,
        1,
        "artifact write failure must exit 1, out: {:?}, err: {:?}",
        String::from_utf8_lossy(&out),
        String::from_utf8_lossy(&err)
    );

    let stderr = String::from_utf8_lossy(&err);
    assert!(
        stderr.contains("cannot write artifact file"),
        "stderr must report artifact write failure, got: {stderr}"
    );
    // stdout must still contain valid report JSON.
    let stdout = String::from_utf8_lossy(&out);
    assert!(!stdout.is_empty(), "stdout must still have the report");
    let _report: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("stdout must be valid JSON: {e}"));
}

#[test]
fn artifact_write_failure_for_empty_store_exits_1_with_stderr() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    populate_store(&dir, &[]);
    // Make .scryrs/hotspots.json a directory so write fails.
    let artifact_path = dir.path().join(".scryrs/hotspots.json");
    std::fs::create_dir(&artifact_path)
        .unwrap_or_else(|e| panic!("create hotspots.json as dir: {e}"));

    let mut out = Vec::new();
    let mut err = Vec::new();

    let exit_code = run_with_writers(
        ["hotspots", &dir.path().display().to_string()],
        &mut out,
        &mut err,
    );

    assert_eq!(
        exit_code, 1,
        "empty store artifact write failure must exit 1"
    );

    let stderr = String::from_utf8_lossy(&err);
    assert!(
        stderr.contains("cannot write artifact file"),
        "stderr must report artifact write failure, got: {stderr}"
    );
    let stdout = String::from_utf8_lossy(&out);
    assert!(
        stdout.contains("\"entries\":[]"),
        "stdout must contain empty entries"
    );
}

// --- Full subject-family fixture test ---

#[test]
fn full_subject_family_fixture_produces_correct_ranking() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    let events = vec![
        // FileOpened: src/main.rs, session s1, weight 1 → score 1
        make_file_opened("s1", "src/main.rs", "2026-06-21T09:00:00Z"),
        // SearchRun: "error handling", session s1, weight 2 → score 2
        make_search_run("s1", "error handling", "2026-06-21T09:01:00Z"),
        // SymbolInspected: "Dispatcher", session s1, weight 2 → score 2
        make_symbol_inspected("s1", "Dispatcher", "2026-06-21T09:02:00Z"),
        // CommandExecuted (success): "cargo build", session s1, weight 1 → score 1
        make_command_executed(
            "s1",
            "cargo build",
            "2026-06-21T09:03:00Z",
            Outcome::Success,
        ),
        // CommandExecuted (failure): "cargo test", session s2, weight 1 + 2 bonus → score 3
        make_command_executed(
            "s2",
            "cargo test",
            "2026-06-21T09:04:00Z",
            Outcome::Failure {
                reason: Some("exit code 1".into()),
            },
        ),
        // DocRetrieved: "docs/api.md", session s1, weight 2 → score 2
        make_doc_retrieved("s1", "docs/api.md", "2026-06-21T09:05:00Z"),
        // EditMade (success): src/lib.rs, session s1, weight 3 → score 3
        make_edit_made("s1", "src/lib.rs", "2026-06-21T09:06:00Z", Outcome::Success),
        // EditMade (failure): src/broken.rs, session s2, weight 3 + 2 bonus → score 5
        make_edit_made(
            "s2",
            "src/broken.rs",
            "2026-06-21T09:07:00Z",
            Outcome::Failure {
                reason: Some("write error".into()),
            },
        ),
        // FailedLookup: "nonexistent_fn", session s1, weight 4 + 2 bonus → score 6
        make_failed_lookup(
            "s1",
            "nonexistent_fn",
            "symbol not found",
            "2026-06-21T09:08:00Z",
        ),
        // FileOpened: src/main.rs again, same session s1, adds +1 → total 2
        make_file_opened("s1", "src/main.rs", "2026-06-21T09:09:00Z"),
    ];
    populate_store(&dir, &events);

    let mut out = Vec::new();
    let mut err = Vec::new();

    let exit = run_with_writers(
        ["hotspots", &dir.path().display().to_string()],
        &mut out,
        &mut err,
    );

    assert_eq!(exit, 0, "stderr: {:?}", String::from_utf8_lossy(&err));

    let stdout = String::from_utf8_lossy(&out);
    let report: serde_json::Value =
        serde_json::from_str(stdout.trim()).unwrap_or_else(|e| panic!("parse JSON: {e}"));

    let entries = report["entries"]
        .as_array()
        .unwrap_or_else(|| panic!("entries not array"));

    // 9 distinct (subject_kind, subject) pairs expected.
    assert!(
        entries.len() >= 9,
        "expected at least 9 entries, got {}",
        entries.len()
    );

    // Verify ranking and scores against documented weight table.
    // Expected scores:
    // nonexistent_fn (FailedLookup): 4 + 2 = 6
    // src/broken.rs (EditMade Failure): 3 + 2 = 5
    // src/lib.rs (EditMade Success): 3
    // cargo test (CommandExecuted Failure): 1 + 2 = 3
    // src/main.rs (2x FileOpened): 1 + 1 = 2
    // error handling (SearchRun): 2
    // Dispatcher (SymbolInspected): 2
    // docs/api.md (DocRetrieved): 2
    // cargo build (CommandExecuted Success): 1

    assert_eq!(entries[0]["subject"], "nonexistent_fn");
    assert_eq!(entries[0]["subjectKind"], "symbol");
    assert_eq!(entries[0]["score"], 6);

    assert_eq!(entries[1]["subject"], "src/broken.rs");
    assert_eq!(entries[1]["subjectKind"], "file");
    assert_eq!(entries[1]["score"], 5);

    assert_eq!(entries[2]["subject"], "src/lib.rs");
    assert_eq!(entries[2]["subjectKind"], "file");
    assert_eq!(entries[2]["score"], 3);

    assert_eq!(entries[3]["subject"], "cargo test");
    assert_eq!(entries[3]["subjectKind"], "command");
    assert_eq!(entries[3]["score"], 3);

    // Verify counts and evidence for top entry (nonexistent_fn).
    assert_eq!(entries[0]["counts"]["eventType"]["FailedLookup"], 1);
    assert_eq!(entries[0]["counts"]["outcome"]["failure"], 1);
    assert_eq!(entries[0]["sessionCount"], 1);
    assert!(
        !entries[0]["evidence"]["rowIds"]
            .as_array()
            .unwrap_or_else(|| panic!("rowIds not array"))
            .is_empty()
    );

    // Verify artifact file is written and matches stdout.
    let artifact_path = dir.path().join(".scryrs/hotspots.json");
    assert!(artifact_path.exists());
    let artifact_content =
        std::fs::read_to_string(&artifact_path).unwrap_or_else(|e| panic!("read artifact: {e}"));
    assert_eq!(
        stdout.trim(),
        artifact_content.trim(),
        "stdout and artifact file must match"
    );
}
