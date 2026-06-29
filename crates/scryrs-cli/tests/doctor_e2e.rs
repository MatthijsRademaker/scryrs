use std::process::Command;

#[test]
#[allow(clippy::disallowed_methods)]
fn doctor_reports_env_only_live_configuration_without_claiming_implicit_local_defaults() {
    let dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));

    let output = Command::new(env!("CARGO_BIN_EXE_scryrs"))
        .args(["doctor", "--json"])
        .current_dir(dir.path())
        .env("SCRYRS_REMOTE_INGEST_URL", "http://127.0.0.1:1")
        .env("SCRYRS_REPOSITORY_ID", "repo-a")
        .env("SCRYRS_WORKSPACE_ID", "workspace-a")
        .env("SCRYRS_AGENT_ID", "agent-a")
        .env("SCRYRS_REMOTE_TIMEOUT_MS", "50")
        .output()
        .unwrap_or_else(|error| panic!("run doctor: {error}"));

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty(), "stderr: {:?}", output.stderr);

    let stdout =
        String::from_utf8(output.stdout).unwrap_or_else(|error| panic!("stdout utf8: {error}"));
    let report: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|error| panic!("parse json: {error}\n{stdout}"));

    assert_eq!(report["mode"], "live");
    let config = report["findings"]
        .as_array()
        .and_then(|findings| {
            findings
                .iter()
                .find(|finding| finding["category"] == "config")
        })
        .unwrap_or_else(|| panic!("missing config finding: {report}"));
    assert_eq!(config["status"], "ok");
    assert!(
        config["summary"]
            .as_str()
            .unwrap_or_default()
            .contains("environment overrides")
    );
    assert_eq!(
        config["details"]["envOverrides"],
        serde_json::json!([
            "SCRYRS_REMOTE_INGEST_URL",
            "SCRYRS_REPOSITORY_ID",
            "SCRYRS_WORKSPACE_ID",
            "SCRYRS_AGENT_ID",
            "SCRYRS_REMOTE_TIMEOUT_MS"
        ])
    );
}
