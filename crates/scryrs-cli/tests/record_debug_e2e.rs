use std::io::Write;
use std::process::{Command, Stdio};

use scryrs_types::SCHEMA_VERSION;

fn make_valid_event_json(session_id: &str, doc_ref: &str) -> String {
    format!(
        r#"{{"schema_version":"{}","timestamp":"2026-06-20T00:00:00Z","session_id":"{}","event_type":"DocRetrieved","tool_name":"read","payload":{{"type":"DocRetrieved","doc_ref":"{}"}},"outcome":{{"result":"Success"}}}}"#,
        SCHEMA_VERSION, session_id, doc_ref
    )
}

#[allow(clippy::disallowed_methods)]
fn run_record_stdin(input: &str, debug: Option<&str>) -> std::process::Output {
    let cwd = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    let mut command = Command::new(env!("CARGO_BIN_EXE_scryrs"));
    command
        .args(["record", "--stdin", "--mode", "local"])
        .current_dir(cwd.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    match debug {
        Some(debug) => {
            command.env("SCRYRS_DEBUG", debug);
        }
        None => {
            command.env_remove("SCRYRS_DEBUG");
        }
    }

    let mut child = command
        .spawn()
        .unwrap_or_else(|e| panic!("spawn scryrs record: {e}"));
    child
        .stdin
        .as_mut()
        .unwrap_or_else(|| panic!("stdin missing"))
        .write_all(input.as_bytes())
        .unwrap_or_else(|e| panic!("write stdin: {e}"));

    child
        .wait_with_output()
        .unwrap_or_else(|e| panic!("wait for output: {e}"))
}

#[test]
fn record_debug_unset_emits_no_debug_lines_and_preserves_contracts() {
    let input = format!(
        "{}\nnot valid json\n",
        make_valid_event_json("s1", "doc/a.md"),
    );

    let output = run_record_stdin(&input, None);
    assert_eq!(output.status.code(), Some(1));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.trim(),
        format!(
            r#"{{"command":"record","schemaVersion":"{}","accepted":1,"rejected":1}}"#,
            SCHEMA_VERSION
        )
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("[scryrs-record]"), "{stderr}");

    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(
        lines.len(),
        1,
        "only rejection JSON expected, got: {stderr}"
    );
    let diag: serde_json::Value =
        serde_json::from_str(lines[0]).unwrap_or_else(|e| panic!("stderr JSON: {e}"));
    assert_eq!(diag["line"], 2);
    assert!(diag["field"].is_null());
    assert!(diag["reason"].is_string());
}

#[test]
fn record_debug_enabled_emits_received_accepted_rejected_inserted_and_summary_lines() {
    let input = format!(
        "{}\nnot valid json\n{}\n",
        make_valid_event_json("s1", "doc/a.md"),
        make_valid_event_json("s2", "doc/b.md"),
    );

    let output = run_record_stdin(&input, Some("1"));
    assert_eq!(output.status.code(), Some(1));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"accepted\":2"));
    assert!(stdout.contains("\"rejected\":1"));

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("[scryrs-record] stage=received line=1"),
        "{stderr}"
    );
    assert!(
        stderr.contains("[scryrs-record] stage=received line=2"),
        "{stderr}"
    );
    assert!(
        stderr.contains("[scryrs-record] stage=received line=3"),
        "{stderr}"
    );
    assert!(stderr.contains("[scryrs-record] stage=accepted line=1 event_type=DocRetrieved session_id=s1 tool_name=read"), "{stderr}");
    assert!(
        stderr.contains("[scryrs-record] stage=rejected line=2"),
        "{stderr}"
    );
    assert!(
        stderr.contains("[scryrs-record] stage=datastore_open"),
        "{stderr}"
    );
    assert!(stderr.contains("[scryrs-record] stage=inserted index=1 event_type=DocRetrieved session_id=s1 tool_name=read"), "{stderr}");
    assert!(stderr.contains("[scryrs-record] stage=inserted index=2 event_type=DocRetrieved session_id=s2 tool_name=read"), "{stderr}");
    assert!(
        stderr.contains("[scryrs-record] stage=transaction_commit accepted=2 rejected=1"),
        "{stderr}"
    );
    assert!(
        stderr.contains("[scryrs-record] stage=summary accepted=2 rejected=1 exit=rejections"),
        "{stderr}"
    );
    assert!(
        stderr.contains(r#"{"line":2,"field":null,"reason":""#),
        "{stderr}"
    );
}
