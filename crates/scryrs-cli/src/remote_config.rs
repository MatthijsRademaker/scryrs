//! Remote ingest configuration resolver.
//!
//! Discovers the nearest ancestor `scryrs.json`, reads optional `remote` defaults,
//! applies environment overrides, and returns deterministic local-vs-remote transport
//! mode. Called before input/store I/O so that mode selection is side-effect free.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::live_bootstrap::SCRYRS_DOCKER_NETWORK_ENV;

/// Remote ingest configuration, resolved from manifest defaults and environment overrides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RemoteConfig {
    pub ingest_url: String,
    pub repository_id: String,
    pub workspace_id: String,
    pub agent_id: String,
    pub timeout_ms: u64,
}

/// Transport mode for a `scryrs record` invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum TransportMode {
    /// Local mode — write to `.scryrs/scryrs.db`.
    Local,
    /// Remote mode — submit to the configured ingest URL.
    Remote,
}

/// Resolved remote configuration result.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct ResolvedRemote {
    pub mode: TransportMode,
    pub config: RemoteConfig,
}

/// Error returned when required remote identity cannot be resolved.
#[derive(Debug, Clone)]
pub(crate) enum RemoteConfigError {
    /// The ingest URL is configured, but a required identity field is missing.
    MissingIdentity { field: &'static str },
}

impl RemoteConfigError {
    /// The configuration field that could not be resolved.
    pub(crate) fn missing_field(&self) -> &'static str {
        match self {
            RemoteConfigError::MissingIdentity { field } => field,
        }
    }
}

impl std::fmt::Display for RemoteConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RemoteConfigError::MissingIdentity { field } => {
                write!(
                    f,
                    "remote ingest requires {field} — set {0} in .scryrs/.env, set the {0} environment variable, or configure scryrs.json remote.{field}",
                    env_key(field)
                )
            }
        }
    }
}

/// Write deterministic remediation guidance when required live configuration
/// cannot be resolved from any layer. Names the missing field and both
/// remediation paths: populating `.scryrs/.env`, or selecting local mode.
pub(crate) fn write_missing_config_guidance(
    err: &mut impl std::io::Write,
    command: &str,
    missing_field: &str,
) {
    let _ = writeln!(
        err,
        "scryrs {command}: live mode is the default but {missing_field} is not configured"
    );
    let _ = writeln!(
        err,
        "Populate .scryrs/.env with SCRYRS_REMOTE_INGEST_URL, SCRYRS_REPOSITORY_ID, SCRYRS_WORKSPACE_ID, and SCRYRS_AGENT_ID,"
    );
    let _ = writeln!(
        err,
        "or rerun with --mode local for local-only, zero-network operation."
    );
    let _ = writeln!(err, "See `scryrs --help`");
}

/// Default remote timeout in milliseconds.
pub(crate) const DEFAULT_REMOTE_TIMEOUT_MS: u64 = 3000;

/// Environment variable names for remote configuration overrides.
const ENV_INGEST_URL: &str = "SCRYRS_REMOTE_INGEST_URL";
const ENV_REPOSITORY_ID: &str = "SCRYRS_REPOSITORY_ID";
const ENV_WORKSPACE_ID: &str = "SCRYRS_WORKSPACE_ID";
const ENV_AGENT_ID: &str = "SCRYRS_AGENT_ID";
const ENV_TIMEOUT_MS: &str = "SCRYRS_REMOTE_TIMEOUT_MS";

fn env_key(field: &str) -> &'static str {
    match field {
        "ingest_url" => ENV_INGEST_URL,
        "repository_id" => ENV_REPOSITORY_ID,
        "workspace_id" => ENV_WORKSPACE_ID,
        "agent_id" => ENV_AGENT_ID,
        "timeout_ms" => ENV_TIMEOUT_MS,
        _ => "SCRYRS_REMOTE_<FIELD>",
    }
}

/// Optional CLI-provided overrides that take precedence over every other layer.
#[derive(Debug, Default, Clone)]
#[allow(dead_code)]
pub(crate) struct RemoteOverrides {
    pub ingest_url: Option<String>,
    pub repository_id: Option<String>,
    pub workspace_id: Option<String>,
    pub agent_id: Option<String>,
    pub timeout_ms: Option<u64>,
}

/// Resolve the remote config using the default precedence with no CLI overrides.
///
/// When `base_path` is provided, ancestor discovery starts from that directory
/// (used by hook-triggered resolution rooted at the event cwd). When `None`,
/// ancestor discovery starts from `std::env::current_dir()` (record path).
///
/// Returns `Ok(None)` when no ingest URL resolves (local mode).
/// Returns `Err(RemoteConfigError)` when remote mode is active but missing required identity.
pub(crate) fn resolve_remote_config(
    base_path: Option<&Path>,
) -> Result<Option<ResolvedRemote>, RemoteConfigError> {
    resolve_remote_config_with(base_path, &RemoteOverrides::default())
}

/// Resolve the remote config across the full precedence chain, highest first:
/// CLI overrides > process environment > `.scryrs/.env` > `scryrs.json` `remote`.
///
/// `repository_id` falls back to the normalized Git remote-origin identity when
/// unresolved by any layer. `timeout_ms` defaults to [`DEFAULT_REMOTE_TIMEOUT_MS`].
#[allow(dead_code)]
pub(crate) fn resolve_remote_config_with(
    base_path: Option<&Path>,
    overrides: &RemoteOverrides,
) -> Result<Option<ResolvedRemote>, RemoteConfigError> {
    let manifest_remote = discover_manifest_remote(base_path);
    let dotenv = load_dotenv(base_path);

    let ingest_url = resolve_field(
        overrides.ingest_url.as_deref(),
        ENV_INGEST_URL,
        &dotenv,
        &manifest_remote.ingest_url,
    );
    let repository_id = resolve_field(
        overrides.repository_id.as_deref(),
        ENV_REPOSITORY_ID,
        &dotenv,
        &manifest_remote.repository_id,
    );
    let workspace_id = resolve_field(
        overrides.workspace_id.as_deref(),
        ENV_WORKSPACE_ID,
        &dotenv,
        &manifest_remote.workspace_id,
    );
    let agent_id = resolve_field(
        overrides.agent_id.as_deref(),
        ENV_AGENT_ID,
        &dotenv,
        &manifest_remote.agent_id,
    );
    let timeout_ms = resolve_timeout(overrides.timeout_ms, &dotenv, manifest_remote.timeout_ms);

    // If no ingest URL resolved, stay in local mode.
    if ingest_url.is_empty() {
        return Ok(None);
    }

    // Remote mode active — resolve repository_id.
    let repo_id = if repository_id.is_empty() {
        git_remote_origin_url().unwrap_or_default()
    } else {
        repository_id
    };

    if repo_id.is_empty() {
        return Err(RemoteConfigError::MissingIdentity {
            field: "repository_id",
        });
    }

    if workspace_id.is_empty() {
        return Err(RemoteConfigError::MissingIdentity {
            field: "workspace_id",
        });
    }

    // `agent_id` is never a hard requirement: when no override layer supplies it,
    // autogenerate a stable per-container identity from the hostname. The hook
    // runs as a fresh process per tool call, so a hostname-derived value stays
    // constant for the container's lifetime where a pid/random value would not.
    let agent_id = if agent_id.is_empty() {
        autogenerated_agent_id()
    } else {
        agent_id
    };

    Ok(Some(ResolvedRemote {
        mode: TransportMode::Remote,
        config: RemoteConfig {
            ingest_url,
            repository_id: repo_id,
            workspace_id,
            agent_id,
            timeout_ms,
        },
    }))
}

/// Resolve the dashboard live target (server URL + repository id) from the
/// precedence chain, without requiring the workspace/agent identity that
/// ingest needs. Returns `Ok(None)` when no server URL resolves.
#[allow(dead_code)]
pub(crate) fn resolve_dashboard_target(
    base_path: Option<&Path>,
    server_url_override: Option<&str>,
    repository_id_override: Option<&str>,
) -> Result<Option<(String, String)>, RemoteConfigError> {
    let manifest_remote = discover_manifest_remote(base_path);
    let dotenv = load_dotenv(base_path);

    let server_url = resolve_field(
        server_url_override,
        ENV_INGEST_URL,
        &dotenv,
        &manifest_remote.ingest_url,
    );
    if server_url.is_empty() {
        return Ok(None);
    }

    let repository_id = resolve_field(
        repository_id_override,
        ENV_REPOSITORY_ID,
        &dotenv,
        &manifest_remote.repository_id,
    );
    let repo_id = if repository_id.is_empty() {
        git_remote_origin_url().unwrap_or_default()
    } else {
        repository_id
    };
    if repo_id.is_empty() {
        return Err(RemoteConfigError::MissingIdentity {
            field: "repository_id",
        });
    }

    Ok(Some((server_url, repo_id)))
}

/// Resolve a single string field by precedence: CLI override, process env,
/// `.scryrs/.env`, then manifest default. First non-empty (trimmed) value wins.
fn resolve_field(
    override_value: Option<&str>,
    env_var: &str,
    dotenv: &HashMap<String, String>,
    manifest_value: &str,
) -> String {
    if let Some(value) = override_value {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    if let Ok(value) = std::env::var(env_var) {
        if !value.trim().is_empty() {
            return value.trim().to_string();
        }
    }
    if let Some(value) = dotenv.get(env_var) {
        if !value.trim().is_empty() {
            return value.trim().to_string();
        }
    }
    manifest_value.trim().to_string()
}

/// Resolve the timeout by the same precedence; unparseable values fall through.
fn resolve_timeout(
    override_value: Option<u64>,
    dotenv: &HashMap<String, String>,
    manifest_value: Option<u64>,
) -> u64 {
    if let Some(value) = override_value {
        return value;
    }
    if let Ok(value) = std::env::var(ENV_TIMEOUT_MS) {
        if let Ok(parsed) = value.trim().parse::<u64>() {
            return parsed;
        }
    }
    if let Some(value) = dotenv.get(ENV_TIMEOUT_MS) {
        if let Ok(parsed) = value.trim().parse::<u64>() {
            return parsed;
        }
    }
    manifest_value.unwrap_or(DEFAULT_REMOTE_TIMEOUT_MS)
}

/// Load `.scryrs/.env` from the nearest ancestor directory that contains one.
/// A missing file is not an error — it contributes no values.
pub(crate) fn load_dotenv(base_path: Option<&Path>) -> HashMap<String, String> {
    let path = match find_nearest_ancestor(".scryrs/.env", base_path) {
        Some(p) => p,
        None => return HashMap::new(),
    };
    match std::fs::read_to_string(&path) {
        Ok(content) => parse_dotenv(&content),
        Err(_) => HashMap::new(),
    }
}

/// Parse `KEY=value` dotenv content. Blank lines and lines beginning with `#`
/// are ignored. Surrounding single or double quotes on the value are stripped.
pub(crate) fn parse_dotenv(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        let value = value.trim();
        let value = value
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
            .unwrap_or(value);
        map.insert(key.to_string(), value.to_string());
    }
    map
}

/// Raw remote defaults parsed from the nearest ancestor `scryrs.json`.
#[derive(Debug, Default, Clone)]
struct ManifestRemote {
    ingest_url: String,
    repository_id: String,
    workspace_id: String,
    agent_id: String,
    docker_network: String,
    timeout_ms: Option<u64>,
}

/// Walk ancestor directories from `base_path` (or CWD if None) and read the
/// first `scryrs.json` found. Return its `remote` section defaults, or empty
/// defaults if none found.
fn discover_manifest_remote(base_path: Option<&Path>) -> ManifestRemote {
    let manifest_path = match find_nearest_ancestor("scryrs.json", base_path) {
        Some(p) => p,
        None => return ManifestRemote::default(),
    };

    let content = match std::fs::read_to_string(&manifest_path) {
        Ok(c) => c,
        Err(_) => return ManifestRemote::default(),
    };

    let parsed: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return ManifestRemote::default(),
    };

    let remote = match parsed.get("remote") {
        Some(r) => r,
        None => return ManifestRemote::default(),
    };

    ManifestRemote {
        ingest_url: remote
            .get("ingest_url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        repository_id: remote
            .get("repository_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        workspace_id: remote
            .get("workspace_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        agent_id: remote
            .get("agent_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        docker_network: remote
            .get("docker_network")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        timeout_ms: remote.get("timeout_ms").and_then(|v| v.as_u64()),
    }
}

/// Walk ancestor directories starting from `base_path` (or CWD if `None`),
/// return the first path where `filename` exists.
fn find_nearest_ancestor(filename: &str, base_path: Option<&Path>) -> Option<PathBuf> {
    let start = match base_path {
        Some(p) if p.is_absolute() => p.to_path_buf(),
        Some(p) => std::env::current_dir().ok()?.join(p),
        None => std::env::current_dir().ok()?,
    };
    let mut dir: Option<&Path> = Some(&start);

    while let Some(current) = dir {
        let candidate = current.join(filename);
        if candidate.is_file() {
            return Some(candidate);
        }
        dir = current.parent();
    }

    None
}

/// Resolve the external Docker network name across the full precedence chain,
/// highest first: CLI override > `SCRYRS_DOCKER_NETWORK` env > `.scryrs/.env` >
/// `scryrs.json` `remote.docker_network` (committed base layer).
///
/// Returns `None` when no layer supplies a non-empty value.
pub(crate) fn resolve_docker_network(
    base_path: Option<&Path>,
    cli_value: Option<&str>,
) -> Option<String> {
    if let Some(value) = cli_value {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    if let Ok(value) = std::env::var(SCRYRS_DOCKER_NETWORK_ENV) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    let dotenv = load_dotenv(base_path);
    if let Some(value) = dotenv.get(SCRYRS_DOCKER_NETWORK_ENV) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    let manifest = discover_manifest_remote(base_path);
    let trimmed = manifest.docker_network.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Autogenerate a stable per-container `agent_id` when no override layer supplies
/// one. Prefers the container hostname (`HOSTNAME` env, then the `hostname`
/// command), which is constant for the container's lifetime. Falls back to a
/// process-derived value only when the hostname is unavailable.
fn autogenerated_agent_id() -> String {
    if let Ok(value) = std::env::var("HOSTNAME") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    if let Some(value) = hostname_via_command() {
        return value;
    }
    format!("agent-{}", std::process::id())
}

/// Try to resolve the container/host name via the `hostname` command.
#[allow(clippy::disallowed_methods)]
fn hostname_via_command() -> Option<String> {
    let output = std::process::Command::new("hostname").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Try to resolve the Git remote origin URL from the current working directory.
#[allow(clippy::disallowed_methods)]
fn git_remote_origin_url() -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let url = String::from_utf8(output.stdout).ok()?;
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Strip trailing `.git` suffix for normalization.
    let normalized = trimmed.strip_suffix(".git").unwrap_or(trimmed);
    Some(normalized.to_string())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    /// An environment variable name guaranteed not to be set, so `resolve_field`
    /// tests are deterministic regardless of the ambient process environment.
    const UNSET_ENV: &str = "SCRYRS_REMOTE_CONFIG_TEST_DEFINITELY_UNSET";

    fn unique_temp_dir(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("scryrs-remote-config-{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn parse_dotenv_ignores_comments_and_blanks() {
        let content =
            "# a comment\n\n  # indented comment\nSCRYRS_REMOTE_INGEST_URL=http://x:8081\n";
        let map = parse_dotenv(content);
        assert_eq!(map.len(), 1);
        assert_eq!(
            map.get("SCRYRS_REMOTE_INGEST_URL").map(String::as_str),
            Some("http://x:8081")
        );
    }

    #[test]
    fn parse_dotenv_strips_quotes_and_trims() {
        let content = "SCRYRS_WORKSPACE_ID = \"ws-1\" \nSCRYRS_AGENT_ID='agent-1'\n";
        let map = parse_dotenv(content);
        assert_eq!(
            map.get("SCRYRS_WORKSPACE_ID").map(String::as_str),
            Some("ws-1")
        );
        assert_eq!(
            map.get("SCRYRS_AGENT_ID").map(String::as_str),
            Some("agent-1")
        );
    }

    #[test]
    fn parse_dotenv_skips_lines_without_equals_or_empty_key() {
        let content = "no_equals_here\n=missing_key\nSCRYRS_AGENT_ID=ok\n";
        let map = parse_dotenv(content);
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("SCRYRS_AGENT_ID").map(String::as_str), Some("ok"));
    }

    #[test]
    fn resolve_field_override_wins() {
        let dotenv = HashMap::new();
        let resolved = resolve_field(Some("flag-value"), UNSET_ENV, &dotenv, "manifest-value");
        assert_eq!(resolved, "flag-value");
    }

    #[test]
    fn resolve_field_falls_through_override_to_dotenv() {
        let mut dotenv = HashMap::new();
        dotenv.insert(UNSET_ENV.to_string(), "dotenv-value".to_string());
        // Empty override falls through; env unset; dotenv wins over manifest.
        let resolved = resolve_field(Some("   "), UNSET_ENV, &dotenv, "manifest-value");
        assert_eq!(resolved, "dotenv-value");
    }

    #[test]
    fn resolve_field_falls_back_to_manifest() {
        let dotenv = HashMap::new();
        let resolved = resolve_field(None, UNSET_ENV, &dotenv, "  manifest-value  ");
        assert_eq!(resolved, "manifest-value");
    }

    #[test]
    fn resolve_timeout_prefers_override_then_dotenv_then_manifest_then_default() {
        let dotenv = HashMap::new();
        assert_eq!(resolve_timeout(Some(42), &dotenv, Some(99)), 42);

        let mut dotenv = HashMap::new();
        dotenv.insert(ENV_TIMEOUT_MS.to_string(), "1500".to_string());
        // Process env is normally unset in tests; dotenv wins over manifest here.
        if std::env::var(ENV_TIMEOUT_MS).is_err() {
            assert_eq!(resolve_timeout(None, &dotenv, Some(99)), 1500);
        }

        let empty = HashMap::new();
        if std::env::var(ENV_TIMEOUT_MS).is_err() {
            assert_eq!(resolve_timeout(None, &empty, Some(99)), 99);
            assert_eq!(
                resolve_timeout(None, &empty, None),
                DEFAULT_REMOTE_TIMEOUT_MS
            );
        }
    }

    #[test]
    fn load_dotenv_missing_file_is_empty() {
        let dir = unique_temp_dir("missing");
        let map = load_dotenv(Some(&dir));
        assert!(map.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn write_manifest(dir: &Path, remote_json: &str) {
        std::fs::write(
            dir.join("scryrs.json"),
            format!("{{\"remote\": {remote_json}}}\n"),
        )
        .expect("write scryrs.json");
    }

    fn write_scryrs_env(dir: &Path, contents: &str) {
        let scryrs_dir = dir.join(".scryrs");
        std::fs::create_dir_all(&scryrs_dir).expect("create .scryrs");
        std::fs::write(scryrs_dir.join(".env"), contents).expect("write .env");
    }

    #[test]
    fn resolve_docker_network_from_manifest_only() {
        let dir = unique_temp_dir("net-manifest");
        write_manifest(&dir, "{\"docker_network\": \"manifest-net\"}");
        // No CLI value, no .env. (SCRYRS_DOCKER_NETWORK is normally unset in tests.)
        if std::env::var(SCRYRS_DOCKER_NETWORK_ENV).is_err() {
            assert_eq!(
                resolve_docker_network(Some(&dir), None),
                Some("manifest-net".to_string())
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_docker_network_dotenv_beats_manifest() {
        let dir = unique_temp_dir("net-dotenv");
        write_manifest(&dir, "{\"docker_network\": \"manifest-net\"}");
        write_scryrs_env(&dir, "SCRYRS_DOCKER_NETWORK=dotenv-net\n");
        if std::env::var(SCRYRS_DOCKER_NETWORK_ENV).is_err() {
            assert_eq!(
                resolve_docker_network(Some(&dir), None),
                Some("dotenv-net".to_string())
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_docker_network_cli_override_wins() {
        let dir = unique_temp_dir("net-cli");
        write_manifest(&dir, "{\"docker_network\": \"manifest-net\"}");
        write_scryrs_env(&dir, "SCRYRS_DOCKER_NETWORK=dotenv-net\n");
        assert_eq!(
            resolve_docker_network(Some(&dir), Some("cli-net")),
            Some("cli-net".to_string())
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_docker_network_unresolved_is_none() {
        let dir = unique_temp_dir("net-none");
        // No manifest, no .env, no CLI value.
        if std::env::var(SCRYRS_DOCKER_NETWORK_ENV).is_err() {
            assert_eq!(resolve_docker_network(Some(&dir), None), None);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn autogenerated_agent_id_is_stable_across_calls() {
        let first = autogenerated_agent_id();
        let second = autogenerated_agent_id();
        assert!(!first.is_empty());
        assert_eq!(first, second, "autogenerated agent_id must be stable");
    }

    #[test]
    fn resolve_remote_config_autogenerates_agent_id_when_uncommitted() {
        let dir = unique_temp_dir("agent-autogen");
        // ingest_url + workspace_id + repository_id present, agent_id absent.
        write_manifest(
            &dir,
            "{\"ingest_url\": \"http://srv:8081\", \"workspace_id\": \"ws\", \"repository_id\": \"repo\"}",
        );
        crate::test_support::with_cwd(&dir, || {
            if std::env::var(ENV_AGENT_ID).is_err() {
                let resolved = resolve_remote_config(Some(&dir))
                    .expect("resolution must not error")
                    .expect("remote mode active");
                assert!(
                    !resolved.config.agent_id.is_empty(),
                    "agent_id must be autogenerated"
                );
            }
        });
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_remote_config_agent_id_override_wins() {
        let dir = unique_temp_dir("agent-override");
        write_manifest(
            &dir,
            "{\"ingest_url\": \"http://srv:8081\", \"workspace_id\": \"ws\", \"repository_id\": \"repo\", \"agent_id\": \"committed-agent\"}",
        );
        crate::test_support::with_cwd(&dir, || {
            if std::env::var(ENV_AGENT_ID).is_err() {
                let resolved = resolve_remote_config(Some(&dir))
                    .expect("resolution must not error")
                    .expect("remote mode active");
                assert_eq!(resolved.config.agent_id, "committed-agent");
            }
        });
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_remote_config_repository_id_undeterminable_fails_loudly() {
        let dir = unique_temp_dir("repo-missing");
        // ingest_url + workspace_id present, no repository_id and not a Git checkout.
        write_manifest(
            &dir,
            "{\"ingest_url\": \"http://srv:8081\", \"workspace_id\": \"ws\"}",
        );
        crate::test_support::with_cwd(&dir, || {
            if std::env::var(ENV_REPOSITORY_ID).is_err() {
                let err = resolve_remote_config(Some(&dir))
                    .expect_err("must fail loudly when repository_id is undeterminable");
                assert_eq!(err.missing_field(), "repository_id");
            }
        });
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_dotenv_reads_scryrs_env() {
        let dir = unique_temp_dir("present");
        let scryrs_dir = dir.join(".scryrs");
        std::fs::create_dir_all(&scryrs_dir).expect("create .scryrs");
        std::fs::write(
            scryrs_dir.join(".env"),
            "SCRYRS_REMOTE_INGEST_URL=http://srv:8081\nSCRYRS_WORKSPACE_ID=ws\n",
        )
        .expect("write .env");

        let map = load_dotenv(Some(&dir));
        assert_eq!(
            map.get("SCRYRS_REMOTE_INGEST_URL").map(String::as_str),
            Some("http://srv:8081")
        );
        assert_eq!(
            map.get("SCRYRS_WORKSPACE_ID").map(String::as_str),
            Some("ws")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
