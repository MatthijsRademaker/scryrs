use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use scryrs_types::SCHEMA_VERSION;

fn free_port() -> u16 {
    let listener =
        TcpListener::bind("127.0.0.1:0").unwrap_or_else(|err| panic!("bind free port: {err}"));
    listener
        .local_addr()
        .unwrap_or_else(|err| panic!("local addr: {err}"))
        .port()
}

fn fixture_jsonl() -> String {
    format!(
        r#"{{"schema_version":"{}","timestamp":"2026-06-21T09:00:00Z","session_id":"dash-e2e","event_type":"FileOpened","tool_name":"pi","payload":{{"type":"FileOpened","path":"src/main.rs"}},"outcome":{{"result":"Success"}}}}"#,
        SCHEMA_VERSION,
    ) + "\n"
}

#[allow(clippy::disallowed_methods)]
fn run_record_and_hotspots(bin: &str, dir: &std::path::Path) {
    let mut record = Command::new(bin)
        .args(["record", "--stdin"])
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|err| panic!("spawn record: {err}"));
    record
        .stdin
        .as_mut()
        .unwrap_or_else(|| panic!("record stdin missing"))
        .write_all(fixture_jsonl().as_bytes())
        .unwrap_or_else(|err| panic!("write record stdin: {err}"));
    let output = record
        .wait_with_output()
        .unwrap_or_else(|err| panic!("record wait: {err}"));
    assert!(
        output.status.success(),
        "record stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output = Command::new(bin)
        .args(["hotspots", &dir.display().to_string()])
        .current_dir(dir)
        .output()
        .unwrap_or_else(|err| panic!("run hotspots: {err}"));
    assert!(
        output.status.success(),
        "hotspots stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn http_get(port: u16, path: &str) -> String {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match TcpStream::connect(("127.0.0.1", port)) {
            Ok(mut stream) => {
                let request =
                    format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
                stream
                    .write_all(request.as_bytes())
                    .unwrap_or_else(|err| panic!("write request: {err}"));
                let mut response = String::new();
                stream
                    .read_to_string(&mut response)
                    .unwrap_or_else(|err| panic!("read response: {err}"));
                return response;
            }
            Err(err) if Instant::now() < deadline => {
                let _ = err;
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(err) => panic!("server did not accept connection: {err}"),
        }
    }
}

#[test]
#[allow(clippy::disallowed_methods)]
fn dashboard_command_serves_api_against_fixture_store() {
    let bin = env!("CARGO_BIN_EXE_scryrs");
    let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir: {err}"));
    run_record_and_hotspots(bin, dir.path());

    let port = free_port();
    let mut child = Command::new(bin)
        .args(["dashboard", "--port", &port.to_string(), "--no-open"])
        .current_dir(dir.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|err| panic!("spawn dashboard: {err}"));

    let hotspots = http_get(port, "/api/hotspots");
    assert!(
        hotspots.starts_with("HTTP/1.1 200"),
        "hotspots response: {hotspots}"
    );
    assert!(
        hotspots.contains("src/main.rs"),
        "hotspots body: {hotspots}"
    );

    let sessions = http_get(port, "/api/sessions");
    assert!(
        sessions.starts_with("HTTP/1.1 200"),
        "sessions response: {sessions}"
    );
    assert!(sessions.contains("dash-e2e"), "sessions body: {sessions}");

    child
        .kill()
        .unwrap_or_else(|err| panic!("kill dashboard: {err}"));
    let _ = child.wait();
}
