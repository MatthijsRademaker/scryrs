use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(test)]
use std::sync::Mutex;

use crate::live_bootstrap::SCRYRS_DOCKER_NETWORK_ENV;
use crate::remote_config::resolve_docker_network;

#[cfg(test)]
static DOCKER_BIN_OVERRIDE: Mutex<Option<PathBuf>> = Mutex::new(None);

#[allow(clippy::disallowed_methods)]
pub(crate) fn execute_up(out: &mut impl Write, err: &mut impl Write) -> i32 {
    let cwd = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(error) => {
            let _ = writeln!(
                err,
                "scryrs up: cannot determine current directory: {error}"
            );
            return 1;
        }
    };

    let scryrs_dir = cwd.join(".scryrs");
    let compose_path = scryrs_dir.join("compose.yml");

    if !compose_path.is_file() {
        let _ = writeln!(
            err,
            "scryrs up: missing required scaffold file {}",
            compose_path.display()
        );
        let _ = writeln!(err, "Run `scryrs init --agent <NAME>` in live mode first.");
        let _ = writeln!(err, "Usage: scryrs up");
        let _ = writeln!(err, "See `scryrs --help`");
        return 2;
    }

    // Resolve the external network from the shared precedence chain (CLI flag is
    // not applicable here): SCRYRS_DOCKER_NETWORK env > .scryrs/.env > scryrs.json
    // remote.docker_network. Fail loudly before invoking Compose when unresolved.
    let docker_network = match resolve_docker_network(Some(&cwd), None) {
        Some(value) => value,
        None => {
            let _ = writeln!(
                err,
                "scryrs up: {} could not be resolved from any layer",
                SCRYRS_DOCKER_NETWORK_ENV
            );
            let _ = writeln!(
                err,
                "Set remote.docker_network in scryrs.json (or {SCRYRS_DOCKER_NETWORK_ENV} in the environment / .scryrs/.env), then rerun `scryrs up`."
            );
            let _ = writeln!(
                err,
                "Run `scryrs init --agent <NAME>` in live mode with --docker-network <NAME> to configure it."
            );
            let _ = writeln!(err, "Usage: scryrs up");
            let _ = writeln!(err, "See `scryrs --help`");
            return 2;
        }
    };
    let docker_network = docker_network.as_str();

    let network_check = match Command::new(docker_binary())
        .args(["network", "inspect", docker_network])
        .output()
    {
        Ok(output) => output,
        Err(error) => {
            let _ = writeln!(err, "scryrs up: cannot run docker: {error}");
            return 1;
        }
    };
    if !network_check.status.success() {
        let stderr_text = String::from_utf8_lossy(&network_check.stderr);
        let trimmed = stderr_text.trim();
        let _ = writeln!(
            err,
            "scryrs up: external Docker network '{docker_network}' does not exist"
        );
        if !trimmed.is_empty() {
            let _ = writeln!(err, "{trimmed}");
        }
        let _ = writeln!(err, "Create the network first, then rerun `scryrs up`.");
        return 2;
    }

    let compose_output = match docker_compose_up(&compose_path, docker_network) {
        Ok(output) => output,
        Err(error) => {
            let _ = writeln!(err, "scryrs up: cannot run docker compose: {error}");
            return 1;
        }
    };

    if !compose_output.status.success() {
        if !compose_output.stdout.is_empty() {
            let _ = out.write_all(&compose_output.stdout);
        }
        if !compose_output.stderr.is_empty() {
            let _ = err.write_all(&compose_output.stderr);
        }
        let _ = writeln!(err, "scryrs up: docker compose up -d failed");
        return 1;
    }

    if !compose_output.stdout.is_empty() {
        let _ = out.write_all(&compose_output.stdout);
    }
    if !compose_output.stderr.is_empty() {
        let _ = err.write_all(&compose_output.stderr);
    }
    let _ = writeln!(
        out,
        "scryrs up: started workspace-managed live server from {}",
        compose_path.display()
    );
    0
}

/// Launch the workspace-managed Compose stack. The resolved external network
/// name is injected into the child process environment as
/// `SCRYRS_DOCKER_NETWORK` so the compose file's `${SCRYRS_DOCKER_NETWORK}`
/// substitution resolves; `compose.yml` itself is left unchanged.
#[allow(clippy::disallowed_methods)]
fn docker_compose_up(
    compose_path: &Path,
    docker_network: &str,
) -> std::io::Result<std::process::Output> {
    Command::new(docker_binary())
        .args([
            "compose",
            "-f",
            compose_path.to_string_lossy().as_ref(),
            "up",
            "-d",
        ])
        .env(SCRYRS_DOCKER_NETWORK_ENV, docker_network)
        .output()
}

fn docker_binary() -> PathBuf {
    #[cfg(test)]
    {
        if let Some(path) = DOCKER_BIN_OVERRIDE
            .lock()
            .unwrap_or_else(|e| panic!("docker override lock poisoned: {e}"))
            .clone()
        {
            return path;
        }
    }

    PathBuf::from("docker")
}

#[cfg(test)]
pub(crate) fn set_docker_bin_override(path: Option<PathBuf>) {
    *DOCKER_BIN_OVERRIDE
        .lock()
        .unwrap_or_else(|e| panic!("docker override lock poisoned: {e}")) = path;
}
