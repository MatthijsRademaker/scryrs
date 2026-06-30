use crate::live_bootstrap::SCRYRS_COMPOSE_TEMPLATE;
use crate::run_with_writers;
use crate::test_support::with_cwd;
use crate::up::set_docker_bin_override;

fn with_docker_bin_override(path: std::path::PathBuf, f: impl FnOnce()) {
    set_docker_bin_override(Some(path));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    set_docker_bin_override(None);
    if let Err(error) = result {
        std::panic::resume_unwind(error);
    }
}

fn write_executable(path: &std::path::Path, contents: &str) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::write(path, contents).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
        let mut perms = std::fs::metadata(path)
            .unwrap_or_else(|e| panic!("metadata {}: {e}", path.display()))
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(path, perms)
            .unwrap_or_else(|e| panic!("chmod {}: {e}", path.display()));
    }
}

/// Scaffold only the managed `.scryrs/compose.yml`. The Docker network name is
/// no longer sourced from `.scryrs/.env` — it resolves from `scryrs.json`
/// `remote.docker_network` (or override layers).
fn scaffold_compose(root: &std::path::Path) {
    let scryrs_dir = root.join(".scryrs");
    std::fs::create_dir_all(&scryrs_dir).unwrap_or_else(|e| panic!("create .scryrs: {e}"));
    std::fs::write(scryrs_dir.join("compose.yml"), SCRYRS_COMPOSE_TEMPLATE)
        .unwrap_or_else(|e| panic!("write compose: {e}"));
}

/// Commit `remote.docker_network` into the project `scryrs.json`.
fn write_manifest_network(root: &std::path::Path, docker_network: &str) {
    std::fs::write(
        root.join("scryrs.json"),
        format!("{{\"remote\": {{\"docker_network\": \"{docker_network}\"}}}}\n"),
    )
    .unwrap_or_else(|e| panic!("write scryrs.json: {e}"));
}

/// Write a `.scryrs/.env` override carrying a network name.
fn write_env_network(root: &std::path::Path, docker_network: &str) {
    let scryrs_dir = root.join(".scryrs");
    std::fs::create_dir_all(&scryrs_dir).unwrap_or_else(|e| panic!("create .scryrs: {e}"));
    std::fs::write(
        scryrs_dir.join(".env"),
        format!("SCRYRS_DOCKER_NETWORK={docker_network}\n"),
    )
    .unwrap_or_else(|e| panic!("write .env: {e}"));
}

/// A fake `docker` that records each invocation's argv plus the resolved
/// `SCRYRS_DOCKER_NETWORK` from the child environment. `network inspect`
/// succeeds; `compose ... up -d` echoes a marker and succeeds.
fn write_success_docker(bin: &std::path::Path, log_path: &std::path::Path) {
    write_executable(
        bin,
        &format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{log}'\nprintf 'NET=%s\\n' \"$SCRYRS_DOCKER_NETWORK\" >> '{log}'\nif [ \"$1\" = \"network\" ] && [ \"$2\" = \"inspect\" ]; then\n  exit 0\nfi\nif [ \"$1\" = \"compose\" ] && [ \"$2\" = \"-f\" ] && [ \"$4\" = \"up\" ] && [ \"$5\" = \"-d\" ]; then\n  echo 'compose-started'\n  exit 0\nfi\necho 'unexpected docker invocation' >&2\nexit 90\n",
            log = log_path.display()
        ),
    );
}

#[test]
fn up_missing_scaffold_files_exits_2() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(run_with_writers(["up"], &mut out, &mut err), 2);
        assert!(out.is_empty());
        let stderr = String::from_utf8_lossy(&err);
        assert!(stderr.contains("missing required scaffold file"));
        assert!(stderr.contains("scryrs init --agent <NAME>"));
    });
}

#[test]
fn up_unexpected_argument_exits_2() {
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(run_with_writers(["up", "extra"], &mut out, &mut err), 2);
    assert!(out.is_empty());
    let stderr = String::from_utf8_lossy(&err);
    assert!(stderr.contains("scryrs up: unexpected argument"));
    assert!(stderr.contains("Usage: scryrs up"));
}

#[test]
fn up_unresolved_network_exits_2_before_compose() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    let bin_dir = dir.path().join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap_or_else(|e| panic!("create bin: {e}"));
    let log_path = dir.path().join("docker.log");
    write_success_docker(&bin_dir.join("docker"), &log_path);
    // compose.yml present, but no manifest, no .env, no env var → unresolved.
    scaffold_compose(dir.path());

    with_cwd(dir.path(), || {
        with_docker_bin_override(bin_dir.join("docker"), || {
            if std::env::var("SCRYRS_DOCKER_NETWORK").is_err() {
                let mut out = Vec::new();
                let mut err = Vec::new();

                assert_eq!(run_with_writers(["up"], &mut out, &mut err), 2);
                assert!(out.is_empty());
                let stderr = String::from_utf8_lossy(&err);
                assert!(stderr.contains("could not be resolved from any layer"));
                assert!(stderr.contains("scryrs.json"));
            }
        });
    });

    // Docker is never invoked when the network cannot be resolved.
    assert!(
        !log_path.exists(),
        "docker must not run before the network resolves"
    );
}

#[test]
fn up_missing_external_network_exits_2_without_compose() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    let bin_dir = dir.path().join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap_or_else(|e| panic!("create bin: {e}"));
    let log_path = dir.path().join("docker.log");
    write_executable(
        &bin_dir.join("docker"),
        &format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\nif [ \"$1\" = \"network\" ] && [ \"$2\" = \"inspect\" ]; then\n  echo 'no such network' >&2\n  exit 1\nfi\nexit 99\n",
            log_path.display()
        ),
    );
    scaffold_compose(dir.path());
    write_manifest_network(dir.path(), "agent-net");

    with_cwd(dir.path(), || {
        with_docker_bin_override(bin_dir.join("docker"), || {
            if std::env::var("SCRYRS_DOCKER_NETWORK").is_err() {
                let mut out = Vec::new();
                let mut err = Vec::new();

                assert_eq!(run_with_writers(["up"], &mut out, &mut err), 2);
                assert!(out.is_empty());
                let stderr = String::from_utf8_lossy(&err);
                assert!(stderr.contains("external Docker network 'agent-net' does not exist"));
                assert!(stderr.contains("Create the network first"));
            }
        });
    });

    let log = std::fs::read_to_string(&log_path).unwrap_or_else(|e| panic!("read log: {e}"));
    assert!(log.contains("network inspect agent-net"));
    assert!(!log.contains("compose"));
}

#[test]
fn up_resolves_network_from_manifest_and_injects_into_compose_env() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    let bin_dir = dir.path().join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap_or_else(|e| panic!("create bin: {e}"));
    let log_path = dir.path().join("docker.log");
    write_success_docker(&bin_dir.join("docker"), &log_path);
    // No .scryrs/.env — the network resolves purely from the committed manifest.
    scaffold_compose(dir.path());
    write_manifest_network(dir.path(), "agent-net");
    let expected_compose = dir.path().join(".scryrs/compose.yml");

    with_cwd(dir.path(), || {
        with_docker_bin_override(bin_dir.join("docker"), || {
            if std::env::var("SCRYRS_DOCKER_NETWORK").is_err() {
                let mut out = Vec::new();
                let mut err = Vec::new();

                assert_eq!(run_with_writers(["up"], &mut out, &mut err), 0);
                assert!(err.is_empty(), "stderr: {}", String::from_utf8_lossy(&err));
                let stdout = String::from_utf8_lossy(&out);
                assert!(stdout.contains("compose-started"));
                assert!(stdout.contains("started workspace-managed live server"));
            }
        });
    });

    let log = std::fs::read_to_string(&log_path).unwrap_or_else(|e| panic!("read log: {e}"));
    // Network resolved from the manifest and used for the inspect.
    assert!(log.contains("network inspect agent-net"));
    // Compose launched from the workspace-managed compose file.
    assert!(log.contains("compose -f"));
    assert!(log.contains(&expected_compose.display().to_string()));
    assert!(log.contains("up -d"));
    // The resolved network was injected into the Compose child environment.
    assert!(
        log.contains("NET=agent-net"),
        "SCRYRS_DOCKER_NETWORK must be injected into the compose env, log: {log}"
    );
}

#[test]
fn up_override_layer_wins_over_committed_network() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    let bin_dir = dir.path().join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap_or_else(|e| panic!("create bin: {e}"));
    let log_path = dir.path().join("docker.log");
    write_success_docker(&bin_dir.join("docker"), &log_path);
    scaffold_compose(dir.path());
    // Manifest declares one network; .scryrs/.env overrides with another.
    write_manifest_network(dir.path(), "committed-net");
    write_env_network(dir.path(), "override-net");

    with_cwd(dir.path(), || {
        with_docker_bin_override(bin_dir.join("docker"), || {
            if std::env::var("SCRYRS_DOCKER_NETWORK").is_err() {
                let mut out = Vec::new();
                let mut err = Vec::new();

                assert_eq!(run_with_writers(["up"], &mut out, &mut err), 0);
                assert!(err.is_empty(), "stderr: {}", String::from_utf8_lossy(&err));
            }
        });
    });

    let log = std::fs::read_to_string(&log_path).unwrap_or_else(|e| panic!("read log: {e}"));
    // The .scryrs/.env override wins over the committed manifest value.
    assert!(log.contains("network inspect override-net"));
    assert!(log.contains("NET=override-net"));
    assert!(!log.contains("committed-net"));
}
