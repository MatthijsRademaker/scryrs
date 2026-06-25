//! Remote ingest configuration resolver.
//!
//! Discovers the nearest ancestor `scryrs.json`, reads optional `remote` defaults,
//! applies environment overrides, and returns deterministic local-vs-remote transport
//! mode. Called before input/store I/O so that mode selection is side-effect free.

use std::path::{Path, PathBuf};

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

impl std::fmt::Display for RemoteConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RemoteConfigError::MissingIdentity { field } => {
                write!(
                    f,
                    "remote ingest requires {field} — configure it in scryrs.json remote.{field} or set SCRYRS_{}",
                    env_key(field)
                )
            }
        }
    }
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

/// Resolve the remote config by walking ancestor directories for `scryrs.json`,
/// reading its optional `remote` section, applying environment overrides, and
/// determining transport mode.
///
/// Returns `Ok(None)` when no ingest URL resolves (local mode).
/// Returns `Err(RemoteConfigError)` when remote mode is active but missing required identity.
pub(crate) fn resolve_remote_config() -> Result<Option<ResolvedRemote>, RemoteConfigError> {
    let manifest_remote = discover_manifest_remote();

    // Apply env overrides on top of manifest defaults.
    let ingest_url = env_or(ENV_INGEST_URL, manifest_remote.ingest_url.clone());
    let repository_id = env_or(ENV_REPOSITORY_ID, manifest_remote.repository_id.clone());
    let workspace_id = env_or(ENV_WORKSPACE_ID, manifest_remote.workspace_id.clone());
    let agent_id = env_or(ENV_AGENT_ID, manifest_remote.agent_id.clone());
    let timeout_ms: u64 = match std::env::var(ENV_TIMEOUT_MS) {
        Ok(val) => val.trim().parse().unwrap_or(DEFAULT_REMOTE_TIMEOUT_MS),
        Err(_) => manifest_remote
            .timeout_ms
            .unwrap_or(DEFAULT_REMOTE_TIMEOUT_MS),
    };

    // If no ingest URL resolved, stay in local mode.
    if ingest_url.is_empty() {
        return Ok(None);
    }

    // Remote mode active — resolve repository_id.
    let repo_id = if repository_id.is_empty() {
        // Try git remote origin as fallback.
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

    if agent_id.is_empty() {
        return Err(RemoteConfigError::MissingIdentity { field: "agent_id" });
    }

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

/// Raw remote defaults parsed from the nearest ancestor `scryrs.json`.
#[derive(Debug, Default, Clone)]
struct ManifestRemote {
    ingest_url: String,
    repository_id: String,
    workspace_id: String,
    agent_id: String,
    timeout_ms: Option<u64>,
}

/// Walk ancestor directories from CWD and read the first `scryrs.json` found.
/// Return its `remote` section defaults, or empty defaults if none found.
fn discover_manifest_remote() -> ManifestRemote {
    let manifest_path = match find_nearest_ancestor("scryrs.json") {
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
        timeout_ms: remote.get("timeout_ms").and_then(|v| v.as_u64()),
    }
}

/// Walk ancestor directories starting from the current working directory,
/// return the first path where `filename` exists.
fn find_nearest_ancestor(filename: &str) -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    let mut dir: Option<&Path> = Some(&cwd);

    while let Some(current) = dir {
        let candidate = current.join(filename);
        if candidate.is_file() {
            return Some(candidate);
        }
        dir = current.parent();
    }

    None
}

/// Return the value of `env_var` if non-empty, otherwise `default`.
fn env_or(env_var: &str, default: String) -> String {
    match std::env::var(env_var) {
        Ok(val) if !val.trim().is_empty() => val.trim().to_string(),
        _ => default,
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
