use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

struct ServerProcess(Child);

impl Drop for ServerProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .map(|address| address.port())
        .unwrap_or_else(|error| panic!("allocate port: {error}"))
}

fn write_manifest(root: &std::path::Path) {
    let artifact_dir = root.join(".scryrs");
    std::fs::create_dir_all(&artifact_dir)
        .unwrap_or_else(|error| panic!("create artifact directory: {error}"));
    let manifest = serde_json::json!({
        "schemaVersion": "1.0.0",
        "metadata": {},
        "routes": [{
            "id": "file:src/auth.rs",
            "subjectKind": "file",
            "subject": "src/auth.rs",
            "label": "Authentication",
            "target": "file:src/auth.rs",
            "loadTarget": {"kind": "file", "reference": "src/auth.rs"},
            "kind": "file",
            "evidenceLinks": [{
                "sourceKind": "local_trace_row",
                "subject": "src/auth.rs",
                "rowIds": [1],
                "score": 9
            }],
            "relatedEdges": [],
            "grouping": null
        }]
    });
    std::fs::write(artifact_dir.join("routes.json"), manifest.to_string())
        .unwrap_or_else(|error| panic!("write route manifest: {error}"));
}

fn wait_until_listening(url: &str, server: &mut Child) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = server
            .try_wait()
            .unwrap_or_else(|error| panic!("server status: {error}"))
        {
            panic!("server exited before readiness: {status}");
        }
        match ureq::get(&format!(
            "{url}/v1/repositories/repo-e2e/routes/explain?query=auth"
        ))
        .call()
        {
            Err(ureq::Error::Status(404, _)) => return,
            Ok(_) | Err(ureq::Error::Status(_, _)) | Err(ureq::Error::Transport(_)) => {}
        }
        assert!(Instant::now() < deadline, "server readiness timed out");
        thread::sleep(Duration::from_millis(50));
    }
}

#[allow(clippy::disallowed_methods)]
#[test]
fn publishes_manifest_and_returns_deterministic_live_hints() {
    let temp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_manifest(temp.path());
    let port = free_port();
    let server_url = format!("http://127.0.0.1:{port}");
    let credentials = serde_json::json!([{
        "repositoryId": "repo-e2e",
        "publisherId": "e2e-test",
        "token": "e2e-secret"
    }])
    .to_string();

    let child = Command::new(env!("CARGO_BIN_EXE_scryrs"))
        .args([
            "server",
            "--bind",
            "127.0.0.1",
            "--port",
            &port.to_string(),
            "--store",
            temp.path()
                .join("server.db")
                .to_str()
                .unwrap_or_else(|| panic!("store path must be UTF-8")),
        ])
        .env("SCRYRS_ROUTE_PUBLISH_CREDENTIALS", credentials)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|error| panic!("spawn server: {error}"));
    let mut server = ServerProcess(child);
    wait_until_listening(&server_url, &mut server.0);

    let publication = Command::new(env!("CARGO_BIN_EXE_scryrs"))
        .args([
            "route",
            "publish",
            temp.path()
                .to_str()
                .unwrap_or_else(|| panic!("repository path must be UTF-8")),
            "--server-url",
            &server_url,
            "--repository-id",
            "repo-e2e",
        ])
        .env("SCRYRS_ROUTE_PUBLISH_TOKEN", "e2e-secret")
        .output()
        .unwrap_or_else(|error| panic!("publish command: {error}"));
    assert!(
        publication.status.success(),
        "publication failed: {}",
        String::from_utf8_lossy(&publication.stderr)
    );
    let publication_json: serde_json::Value = serde_json::from_slice(&publication.stdout)
        .unwrap_or_else(|error| panic!("publication JSON: {error}"));
    assert_eq!(publication_json["repositoryId"], "repo-e2e");
    assert_eq!(publication_json["publisherId"], "e2e-test");

    let explain_url = format!("{server_url}/v1/repositories/repo-e2e/routes/explain?query=auth");
    let first = ureq::get(&explain_url)
        .call()
        .unwrap_or_else(|error| panic!("first explain: {error}"))
        .into_string()
        .unwrap_or_else(|error| panic!("first explain body: {error}"));
    let second = ureq::get(&explain_url)
        .call()
        .unwrap_or_else(|error| panic!("second explain: {error}"))
        .into_string()
        .unwrap_or_else(|error| panic!("second explain body: {error}"));

    assert_eq!(first, second);
    let hints: serde_json::Value =
        serde_json::from_str(&first).unwrap_or_else(|error| panic!("hint JSON: {error}"));
    assert_eq!(hints["schemaVersion"], "1.0.0");
    assert_eq!(hints["hints"][0]["label"], "Authentication");
    assert_eq!(hints["hints"][0]["rank"], 1);
}
