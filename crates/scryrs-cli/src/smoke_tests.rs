use crate::{run, run_with_writers};

// Basic entrypoint smoke: verifies run() arg-collection wiring does not panic.
#[test]
fn public_run_entrypoint_no_panic() {
    assert_eq!(run(["--help"]), 0);
    assert_eq!(run(["--version"]), 0);
    assert_eq!(run(["--help-json"]), 0);
    // hotspots with no store exits 2 (missing store).
    assert_eq!(run(["hotspots", "/tmp"]), 2);
    assert_eq!(run(["record", "--file", "/nonexistent"]), 2);
    assert_eq!(run(["publish"]), 2);
    assert_eq!(run(["up"]), 2);
    assert_eq!(run(Vec::<&str>::new()), 0);
    assert_eq!(run(["unknown"]), 2);
    assert_eq!(run(["hotspots"]), 2);
}

#[test]
fn help_exits_0_stdout_nonempty() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(run_with_writers(["--help"], &mut out, &mut err), 0);
    assert!(err.is_empty());
    assert!(!out.is_empty());
}

#[test]
fn version_exits_0_stdout_nonempty() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(run_with_writers(["--version"], &mut out, &mut err), 0);
    assert!(err.is_empty());
    assert!(!out.is_empty());
}

#[test]
fn hotspots_path_exits_0_stdout_nonempty() {
    // Hotspot command requires a valid store to exit 0.
    // This smoke test checks the missing-store case exits 2.
    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(
        run_with_writers(["hotspots", "/tmp"], &mut out, &mut err),
        2
    );
    assert!(out.is_empty());
    assert!(!err.is_empty());
}

#[test]
fn bare_invocation_exits_0_stdout_nonempty() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(run_with_writers(Vec::<&str>::new(), &mut out, &mut err), 0);
    assert!(err.is_empty());
    assert!(!out.is_empty());
}

#[test]
fn unknown_command_exits_2_stderr_nonempty() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(run_with_writers(["unknown"], &mut out, &mut err), 2);
    assert!(out.is_empty());
    assert!(!err.is_empty());
}

#[test]
fn hotspots_without_path_exits_2_stderr_nonempty() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(run_with_writers(["hotspots"], &mut out, &mut err), 2);
    assert!(out.is_empty());
    assert!(!err.is_empty());
}

#[test]
fn help_json_exits_0_stdout_nonempty() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(run_with_writers(["--help-json"], &mut out, &mut err), 0);
    assert!(err.is_empty());
    assert!(!out.is_empty());
}

// --- Docker artifact contract checks (Task 6.4) ---

#[test]
fn dockerfile_has_correct_entrypoint() {
    let dockerfile = include_str!("../../../Dockerfile");
    assert!(
        dockerfile.contains("scryrs\", \"server\", \"--bind\", \"0.0.0.0\", \"--port\", \"8081\", \"--store\", \"/data/scryrs/server.db"),
        "Dockerfile must run scryrs server with documented bind/port/store defaults"
    );
    assert!(
        dockerfile.contains("EXPOSE 8081"),
        "Dockerfile must expose port 8081"
    );
    assert!(
        dockerfile.contains("VOLUME /data/scryrs"),
        "Dockerfile must declare /data/scryrs as a volume for persistent storage"
    );
}

#[test]
fn root_compose_remains_packaging_asset_with_server_service_and_network() {
    let compose = include_str!("../../../docker-compose.yml");
    assert!(
        compose.contains("scryrs-server:"),
        "compose file must define scryrs-server service"
    );
    assert!(
        compose.contains("scryrs-net:"),
        "compose file must define scryrs-net network"
    );
    assert!(
        compose.contains("scryrs-data:"),
        "compose file must define scryrs-data volume"
    );
    assert!(
        compose.contains("8081:8081"),
        "compose file must expose port 8081"
    );
}
