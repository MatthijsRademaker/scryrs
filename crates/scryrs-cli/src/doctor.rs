use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{Value, json};

use scryrs_core::{CANONICAL_STORE_PATH, QueryError, TraceQuery};

use crate::remote_config::{self, RemoteConfigError};

const DOCTOR_SCHEMA_VERSION: &str = "1.0.0";
const DEFAULT_LIVE_TIMEOUT_MS: u64 = 3000;
const CLAUDE_HOOK_COMMAND: &str = "scryrs hook claude-code";

const DOCS_LINKS: &[DocLink] = &[
    DocLink {
        label: "CLI Reference",
        path: ".devagent/docs/docs/cli-v0-contract.md",
    },
    DocLink {
        label: "Production Suite Plan",
        path: ".devagent/docs/docs/production-suite.md",
    },
    DocLink {
        label: "Trace Hook Contract",
        path: ".devagent/docs/docs/trace-hook-contract.md",
    },
    DocLink {
        label: "Verification README",
        path: "scripts/verification/README.md",
    },
];

pub(crate) fn execute_doctor_cli(
    out: &mut impl Write,
    err: &mut impl Write,
    args: &[String],
) -> i32 {
    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        return write_doctor_help(out).map_or(1, |_| 0);
    }

    let mut json_mode = false;
    for arg in args {
        match arg.as_str() {
            "--json" => json_mode = true,
            other => {
                return write_usage_error(
                    err,
                    &format!("scryrs doctor: unexpected argument '{other}'"),
                    &["scryrs doctor [--json]"],
                );
            }
        }
    }

    let cwd = match std::env::current_dir() {
        Ok(path) => path,
        Err(error) => {
            let _ = writeln!(
                err,
                "scryrs doctor: cannot determine current directory: {error}"
            );
            return 1;
        }
    };

    let report = build_doctor_report(&cwd);
    let exit_code = if report
        .findings
        .iter()
        .any(|finding| finding.status == Severity::Error)
    {
        2
    } else {
        0
    };

    if json_mode {
        match serde_json::to_string(&report) {
            Ok(json) => writeln!(out, "{json}").map_or(1, |_| exit_code),
            Err(error) => {
                let _ = writeln!(err, "scryrs doctor: serialization error: {error}");
                1
            }
        }
    } else {
        write_human_report(out, &report).map_or(1, |_| exit_code)
    }
}

pub(crate) fn write_doctor_help(out: &mut impl Write) -> io::Result<()> {
    writeln!(
        out,
        "scryrs doctor — installation and readiness diagnostic command\n\n\
USAGE\n\
  scryrs doctor [--json]\n\n\
OPTIONS\n\
  --json    Emit machine-readable JSON using the same diagnostic categories\n\n\
DIAGNOSTIC CATEGORIES\n\
  - binary version\n\
  - shipped command surface / feature availability\n\
  - resolved local vs live mode\n\
  - local store status\n\
  - Claude Code hook status\n\
  - Pi hook status\n\
  - live server reachability when live mode is configured\n\
  - docs links\n\n\
EXIT CODES\n\
  0    All findings are ok or warn\n\
  1    Output write failure\n\
  2    One or more structural error findings were reported"
    )
}

fn write_usage_error(err: &mut impl Write, message: &str, usage_lines: &[&str]) -> i32 {
    if writeln!(err, "{message}").is_err() {
        return 1;
    }
    for usage_line in usage_lines {
        if writeln!(err, "Usage: {usage_line}").is_err() {
            return 1;
        }
    }
    if writeln!(err, "See `scryrs --help`").is_err() {
        return 1;
    }
    2
}

fn build_doctor_report(repo_root: &Path) -> DoctorReport {
    let manifest = inspect_manifest(repo_root);
    let command_surface = CommandSurface {
        commands: available_commands(),
        features: available_features(),
    };

    let remote_requested = live_mode_requested(&manifest);
    let resolved_remote = remote_config::resolve_remote_config(Some(repo_root));
    let mode = if remote_requested {
        ResolvedMode::Live
    } else {
        ResolvedMode::Local
    };

    let mut findings = vec![
        binary_version_finding(),
        command_surface_finding(&command_surface),
        config_finding(&manifest),
        mode_finding(mode, &resolved_remote),
        live_default_config_finding(repo_root, &resolved_remote),
        local_store_finding(repo_root, mode),
        claude_hook_finding(repo_root),
        pi_hook_finding(repo_root),
        live_server_finding(mode, &manifest, &resolved_remote),
        docs_finding(),
    ];

    // Keep output order deterministic even if future refactors build findings conditionally.
    findings.sort_by(|a, b| a.category.cmp(&b.category));

    DoctorReport {
        schema_version: DOCTOR_SCHEMA_VERSION.into(),
        command: "doctor".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        mode,
        overall_status: findings
            .iter()
            .fold(Severity::Ok, |acc, finding| acc.max(finding.status)),
        command_surface,
        findings,
        docs_links: DOCS_LINKS.to_vec(),
    }
}

fn write_human_report(out: &mut impl Write, report: &DoctorReport) -> io::Result<()> {
    writeln!(out, "scryrs doctor")?;
    writeln!(out, "Status: {}", report.overall_status.as_label())?;
    writeln!(out, "Binary version: {}", report.version)?;
    writeln!(out, "Resolved mode: {}", report.mode.as_str())?;
    writeln!(out)?;
    writeln!(out, "Findings")?;
    for finding in &report.findings {
        writeln!(
            out,
            "- [{}] {}: {}",
            finding.status.as_label(),
            finding.category,
            finding.summary
        )?;
    }
    writeln!(out)?;
    writeln!(out, "Command surface")?;
    writeln!(
        out,
        "- commands: {}",
        report.command_surface.commands.join(", ")
    )?;
    writeln!(
        out,
        "- features: {}",
        report.command_surface.features.join(", ")
    )?;
    writeln!(out)?;
    writeln!(out, "Docs links")?;
    for link in &report.docs_links {
        writeln!(out, "- {}: {}", link.label, link.path)?;
    }
    Ok(())
}

fn binary_version_finding() -> DoctorFinding {
    DoctorFinding::new(
        "binary_version",
        Severity::Ok,
        format!("scryrs {}", env!("CARGO_PKG_VERSION")),
        Some(json!({ "version": env!("CARGO_PKG_VERSION") })),
    )
}

fn command_surface_finding(command_surface: &CommandSurface) -> DoctorFinding {
    DoctorFinding::new(
        "command_surface",
        Severity::Ok,
        format!(
            "{} root commands and {} enabled features",
            command_surface.commands.len(),
            command_surface.features.len()
        ),
        Some(json!({
            "commands": command_surface.commands,
            "features": command_surface.features,
        })),
    )
}

fn config_finding(manifest: &ManifestInspection) -> DoctorFinding {
    match &manifest.state {
        ManifestState::Missing { env_overrides } => {
            if env_overrides.is_empty() {
                DoctorFinding::new(
                    "config",
                    Severity::Ok,
                    "no scryrs.json found; using implicit local defaults".into(),
                    None,
                )
            } else {
                DoctorFinding::new(
                    "config",
                    Severity::Ok,
                    "no scryrs.json found; using environment overrides".into(),
                    Some(json!({ "envOverrides": env_overrides })),
                )
            }
        }
        ManifestState::Ok {
            path,
            env_overrides,
        } => {
            let mut details = json!({ "path": path.display().to_string() });
            if let Some(remote) = &manifest.remote {
                details["remote"] = json!({
                    "ingestUrl": remote.ingest_url,
                    "repositoryId": remote.repository_id,
                    "workspaceId": remote.workspace_id,
                    "agentId": remote.agent_id,
                    "timeoutMs": remote.timeout_ms,
                });
            }
            if !env_overrides.is_empty() {
                details["envOverrides"] = json!(env_overrides);
            }
            DoctorFinding::new(
                "config",
                Severity::Ok,
                format!("loaded scryrs.json from {}", path.display()),
                Some(details),
            )
        }
        ManifestState::Error { path, message } => {
            let mut details = json!({ "message": message });
            if let Some(path) = path {
                details["path"] = json!(path.display().to_string());
            }
            DoctorFinding::new("config", Severity::Error, message.clone(), Some(details))
        }
    }
}

fn mode_finding(
    mode: ResolvedMode,
    resolved_remote: &Result<Option<remote_config::ResolvedRemote>, RemoteConfigError>,
) -> DoctorFinding {
    match (mode, resolved_remote) {
        (ResolvedMode::Local, Ok(None)) => DoctorFinding::new(
            "mode",
            Severity::Ok,
            "resolved mode: local".into(),
            Some(json!({ "mode": "local" })),
        ),
        (ResolvedMode::Live, Ok(Some(resolved))) => DoctorFinding::new(
            "mode",
            Severity::Ok,
            format!("resolved mode: live ({})", resolved.config.ingest_url),
            Some(json!({
                "mode": "live",
                "ingestUrl": resolved.config.ingest_url,
                "repositoryId": resolved.config.repository_id,
                "workspaceId": resolved.config.workspace_id,
                "agentId": resolved.config.agent_id,
                "timeoutMs": resolved.config.timeout_ms,
            })),
        ),
        (ResolvedMode::Live, Err(error)) => DoctorFinding::new(
            "mode",
            Severity::Error,
            format!("live mode is configured but unusable: {error}"),
            Some(json!({ "mode": "live", "error": error.to_string() })),
        ),
        (ResolvedMode::Local, Ok(Some(resolved))) => DoctorFinding::new(
            "mode",
            Severity::Ok,
            format!("resolved mode: live ({})", resolved.config.ingest_url),
            Some(json!({ "mode": "live" })),
        ),
        _ => DoctorFinding::new(
            "mode",
            Severity::Error,
            "mode resolution returned an unexpected state".into(),
            None,
        ),
    }
}

/// Diagnose whether the live default has resolvable configuration. Because live
/// is now the default for `init`, `record`, and `dashboard`, an unresolved remote
/// config is a warning with remediation rather than a silent local fallback.
fn live_default_config_finding(
    repo_root: &Path,
    resolved_remote: &Result<Option<remote_config::ResolvedRemote>, RemoteConfigError>,
) -> DoctorFinding {
    let env_present = repo_root.join(".scryrs").join(".env").is_file();
    match resolved_remote {
        Ok(Some(_)) => DoctorFinding::new(
            "liveConfig",
            Severity::Ok,
            "live config resolves (ingest URL and identity present)".into(),
            Some(json!({ "resolved": true, "envFilePresent": env_present })),
        ),
        Ok(None) => DoctorFinding::new(
            "liveConfig",
            Severity::Warn,
            "live mode is the default but no ingest URL resolves; populate .scryrs/.env (SCRYRS_REMOTE_INGEST_URL, SCRYRS_REPOSITORY_ID, SCRYRS_WORKSPACE_ID, SCRYRS_AGENT_ID) or run commands with --mode local".into(),
            Some(json!({ "resolved": false, "missing": "ingest_url", "envFilePresent": env_present })),
        ),
        Err(error) => DoctorFinding::new(
            "liveConfig",
            Severity::Warn,
            format!(
                "live mode is the default but {} is unresolved; populate .scryrs/.env or run commands with --mode local",
                error.missing_field()
            ),
            Some(json!({ "resolved": false, "missing": error.missing_field(), "envFilePresent": env_present })),
        ),
    }
}

fn local_store_finding(repo_root: &Path, mode: ResolvedMode) -> DoctorFinding {
    let store_path = repo_root.join(CANONICAL_STORE_PATH);
    match TraceQuery::open(repo_root) {
        Ok(query) => match query.iter_events_with_ids_ordered() {
            Ok(events) => {
                let severity = Severity::Ok;
                let summary = if mode == ResolvedMode::Live {
                    format!(
                        "local store is readable at {} ({} events); live mode does not use it",
                        store_path.display(),
                        events.len()
                    )
                } else {
                    format!(
                        "local store is readable at {} ({} events)",
                        store_path.display(),
                        events.len()
                    )
                };
                DoctorFinding::new(
                    "local_store",
                    severity,
                    summary,
                    Some(json!({
                        "path": store_path.display().to_string(),
                        "eventCount": events.len(),
                        "storeSchemaVersion": query.store_schema_version(),
                    })),
                )
            }
            Err(QueryError::EmptyStore) => {
                let (status, summary) = if mode == ResolvedMode::Live {
                    (
                        Severity::Ok,
                        format!(
                            "local store exists at {} but is empty; live mode does not depend on it",
                            store_path.display()
                        ),
                    )
                } else {
                    (
                        Severity::Warn,
                        format!(
                            "local store exists at {} but has no recorded events yet",
                            store_path.display()
                        ),
                    )
                };
                DoctorFinding::new(
                    "local_store",
                    status,
                    summary,
                    Some(json!({
                        "path": store_path.display().to_string(),
                        "eventCount": 0,
                        "storeSchemaVersion": query.store_schema_version(),
                    })),
                )
            }
            Err(QueryError::MissingStore) => missing_store_finding(&store_path, mode),
            Err(QueryError::UnsupportedStore(message)) => DoctorFinding::new(
                "local_store",
                Severity::Error,
                format!("local store is unsupported: {message}"),
                Some(json!({ "path": store_path.display().to_string(), "error": message })),
            ),
            Err(QueryError::StorageError(error)) => DoctorFinding::new(
                "local_store",
                Severity::Error,
                format!("local store is unreadable: {error}"),
                Some(json!({
                    "path": store_path.display().to_string(),
                    "error": error.to_string(),
                })),
            ),
        },
        Err(QueryError::MissingStore) => missing_store_finding(&store_path, mode),
        Err(QueryError::EmptyStore) => missing_store_finding(&store_path, mode),
        Err(QueryError::UnsupportedStore(message)) => DoctorFinding::new(
            "local_store",
            Severity::Error,
            format!("local store is unsupported: {message}"),
            Some(json!({ "path": store_path.display().to_string(), "error": message })),
        ),
        Err(QueryError::StorageError(error)) => DoctorFinding::new(
            "local_store",
            Severity::Error,
            format!("local store is unreadable: {error}"),
            Some(json!({
                "path": store_path.display().to_string(),
                "error": error.to_string(),
            })),
        ),
    }
}

fn missing_store_finding(store_path: &Path, mode: ResolvedMode) -> DoctorFinding {
    let (status, summary) = if mode == ResolvedMode::Live {
        (
            Severity::Ok,
            format!(
                "local store is absent at {} (expected when only live mode is used)",
                store_path.display()
            ),
        )
    } else {
        (
            Severity::Warn,
            format!(
                "local store is not initialized at {}; run `scryrs setup local` or capture events first",
                store_path.display()
            ),
        )
    };
    DoctorFinding::new(
        "local_store",
        status,
        summary,
        Some(json!({ "path": store_path.display().to_string() })),
    )
}

fn claude_hook_finding(repo_root: &Path) -> DoctorFinding {
    let settings_path = repo_root.join(".claude/settings.json");
    let settings = match std::fs::read_to_string(&settings_path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return DoctorFinding::new(
                "claude_code_hook",
                Severity::Warn,
                format!(
                    "Claude Code hook not detected at {}",
                    settings_path.display()
                ),
                Some(json!({ "path": settings_path.display().to_string(), "installed": false })),
            );
        }
        Err(error) => {
            return DoctorFinding::new(
                "claude_code_hook",
                Severity::Error,
                format!(
                    "Claude Code hook settings could not be read at {}: {error}",
                    settings_path.display()
                ),
                Some(
                    json!({ "path": settings_path.display().to_string(), "error": error.to_string() }),
                ),
            );
        }
    };

    let parsed: Value = match serde_json::from_str(&settings) {
        Ok(value) => value,
        Err(error) => {
            return DoctorFinding::new(
                "claude_code_hook",
                Severity::Error,
                format!(
                    "Claude Code hook settings are malformed at {}: {error}",
                    settings_path.display()
                ),
                Some(
                    json!({ "path": settings_path.display().to_string(), "error": error.to_string() }),
                ),
            );
        }
    };

    let installed = parsed
        .get("hooks")
        .and_then(Value::as_object)
        .and_then(|hooks| hooks.get("PreToolUse"))
        .and_then(Value::as_array)
        .map(|entries| {
            entries.iter().any(|entry| {
                entry
                    .get("hooks")
                    .and_then(Value::as_array)
                    .map(|hooks| {
                        hooks.iter().any(|hook| {
                            hook.get("type").and_then(Value::as_str) == Some("command")
                                && hook.get("command").and_then(Value::as_str)
                                    == Some(CLAUDE_HOOK_COMMAND)
                        })
                    })
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false);

    if installed {
        DoctorFinding::new(
            "claude_code_hook",
            Severity::Ok,
            format!("Claude Code hook detected at {}", settings_path.display()),
            Some(json!({ "path": settings_path.display().to_string(), "installed": true })),
        )
    } else {
        DoctorFinding::new(
            "claude_code_hook",
            Severity::Warn,
            format!(
                "Claude Code settings found at {} but the scryrs hook command is not installed",
                settings_path.display()
            ),
            Some(json!({ "path": settings_path.display().to_string(), "installed": false })),
        )
    }
}

fn pi_hook_finding(repo_root: &Path) -> DoctorFinding {
    let hook_path = repo_root.join(".pi/extensions/scryrs/index.ts");
    match std::fs::read_to_string(&hook_path) {
        Ok(_) => DoctorFinding::new(
            "pi_hook",
            Severity::Ok,
            format!("Pi hook detected at {}", hook_path.display()),
            Some(json!({ "path": hook_path.display().to_string(), "installed": true })),
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => DoctorFinding::new(
            "pi_hook",
            Severity::Warn,
            format!("Pi hook not detected at {}", hook_path.display()),
            Some(json!({ "path": hook_path.display().to_string(), "installed": false })),
        ),
        Err(error) => DoctorFinding::new(
            "pi_hook",
            Severity::Error,
            format!(
                "Pi hook could not be read at {}: {error}",
                hook_path.display()
            ),
            Some(json!({ "path": hook_path.display().to_string(), "error": error.to_string() })),
        ),
    }
}

fn live_server_finding(
    mode: ResolvedMode,
    manifest: &ManifestInspection,
    resolved_remote: &Result<Option<remote_config::ResolvedRemote>, RemoteConfigError>,
) -> DoctorFinding {
    if mode == ResolvedMode::Local {
        return DoctorFinding::new(
            "live_server",
            Severity::Ok,
            "live mode not configured; server reachability check skipped".into(),
            None,
        );
    }

    match resolved_remote {
        Ok(Some(resolved)) => probe_live_server(&resolved.config),
        Err(error) => DoctorFinding::new(
            "live_server",
            Severity::Error,
            format!("live server reachability could not be checked: {error}"),
            Some(json!({
                "ingestUrl": manifest.resolved_ingest_url(),
                "error": error.to_string(),
            })),
        ),
        _ => DoctorFinding::new(
            "live_server",
            Severity::Error,
            "live server reachability could not be checked because live mode is not fully resolved"
                .into(),
            None,
        ),
    }
}

fn probe_live_server(config: &remote_config::RemoteConfig) -> DoctorFinding {
    let url = live_probe_url(&config.ingest_url, &config.repository_id);
    let response = ureq::get(&url)
        .timeout(std::time::Duration::from_millis(config.timeout_ms))
        .call();

    match response {
        Ok(resp) if (200..300).contains(&resp.status()) => DoctorFinding::new(
            "live_server",
            Severity::Ok,
            format!("live server is reachable at {}", config.ingest_url),
            Some(json!({
                "ingestUrl": config.ingest_url,
                "probeUrl": url,
                "repositoryId": config.repository_id,
                "timeoutMs": config.timeout_ms,
            })),
        ),
        Ok(resp) => DoctorFinding::new(
            "live_server",
            Severity::Error,
            format!(
                "live server probe returned HTTP {} for {}",
                resp.status(),
                config.ingest_url
            ),
            Some(json!({
                "ingestUrl": config.ingest_url,
                "probeUrl": url,
                "status": resp.status(),
            })),
        ),
        Err(ureq::Error::Status(status, response)) => {
            let body = response.into_string().unwrap_or_default();
            DoctorFinding::new(
                "live_server",
                Severity::Error,
                format!(
                    "live server probe returned HTTP {} for {}",
                    status, config.ingest_url
                ),
                Some(json!({
                    "ingestUrl": config.ingest_url,
                    "probeUrl": url,
                    "status": status,
                    "body": body,
                })),
            )
        }
        Err(ureq::Error::Transport(error)) => DoctorFinding::new(
            "live_server",
            Severity::Error,
            format!(
                "live server is unreachable at {}: {error}",
                config.ingest_url
            ),
            Some(json!({
                "ingestUrl": config.ingest_url,
                "probeUrl": url,
                "timeoutMs": config.timeout_ms,
                "error": error.to_string(),
            })),
        ),
    }
}

fn live_probe_url(ingest_url: &str, repository_id: &str) -> String {
    let base = ingest_url.trim_end_matches('/');
    format!("{base}/v1/repositories/{repository_id}/hotspots?window=cumulative")
}

fn docs_finding() -> DoctorFinding {
    DoctorFinding::new(
        "docs",
        Severity::Ok,
        format!("{} operator docs links available", DOCS_LINKS.len()),
        Some(json!({ "links": DOCS_LINKS })),
    )
}

fn available_commands() -> Vec<String> {
    vec![
        "hotspots",
        "record",
        "hook",
        "init",
        "doctor",
        "graph",
        "route",
        "propose",
        "proposals",
        "dashboard",
        "server",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn available_features() -> Vec<String> {
    [
        cfg!(feature = "core").then_some("core"),
        cfg!(feature = "dashboard").then_some("dashboard"),
        cfg!(feature = "graph").then_some("graph"),
        cfg!(feature = "curator").then_some("curator"),
        cfg!(feature = "markdown").then_some("markdown"),
        cfg!(feature = "runtime").then_some("runtime"),
        cfg!(feature = "guardrails").then_some("guardrails"),
        cfg!(feature = "policy").then_some("policy"),
        cfg!(feature = "sandbox").then_some("sandbox"),
        cfg!(feature = "telemetry").then_some("telemetry"),
        cfg!(feature = "server").then_some("server"),
        cfg!(feature = "llm").then_some("llm"),
        cfg!(feature = "rspress").then_some("rspress"),
    ]
    .into_iter()
    .flatten()
    .map(str::to_string)
    .collect()
}

fn inspect_manifest(repo_root: &Path) -> ManifestInspection {
    let path = find_nearest_ancestor(repo_root, "scryrs.json");
    let env_overrides = active_remote_env_overrides();

    let Some(path) = path else {
        return ManifestInspection {
            state: ManifestState::Missing { env_overrides },
            remote: None,
        };
    };

    let contents = match std::fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) => {
            return ManifestInspection {
                state: ManifestState::Error {
                    path: Some(path),
                    message: format!("scryrs.json could not be read: {error}"),
                },
                remote: None,
            };
        }
    };

    let root: Value = match serde_json::from_str(&contents) {
        Ok(value) => value,
        Err(error) => {
            return ManifestInspection {
                state: ManifestState::Error {
                    path: Some(path),
                    message: format!("scryrs.json is malformed JSON: {error}"),
                },
                remote: None,
            };
        }
    };

    let Some(root_object) = root.as_object() else {
        return ManifestInspection {
            state: ManifestState::Error {
                path: Some(path),
                message: "scryrs.json must be a JSON object".into(),
            },
            remote: None,
        };
    };

    let remote = match root_object.get("remote") {
        None => ManifestRemote::default(),
        Some(Value::Object(remote)) => match build_manifest_remote(remote, &path) {
            Ok(remote) => remote,
            Err(error) => return error.into_inspection(),
        },
        Some(_) => {
            return ManifestInspection {
                state: ManifestState::Error {
                    path: Some(path),
                    message: "scryrs.json remote field must be a JSON object".into(),
                },
                remote: None,
            };
        }
    };

    ManifestInspection {
        state: ManifestState::Ok {
            path,
            env_overrides,
        },
        remote: Some(remote),
    }
}

fn build_manifest_remote(
    remote: &serde_json::Map<String, Value>,
    path: &Path,
) -> Result<ManifestRemote, ManifestParseError> {
    let ingest_url = read_optional_string(remote, "ingest_url", path)?;
    let repository_id = read_optional_string(remote, "repository_id", path)?;
    let workspace_id = read_optional_string(remote, "workspace_id", path)?;
    let agent_id = read_optional_string(remote, "agent_id", path)?;
    let timeout_ms =
        read_optional_u64(remote, "timeout_ms", path)?.unwrap_or(DEFAULT_LIVE_TIMEOUT_MS);

    Ok(ManifestRemote {
        ingest_url,
        repository_id,
        workspace_id,
        agent_id,
        timeout_ms,
    })
}

fn read_optional_string(
    object: &serde_json::Map<String, Value>,
    key: &str,
    path: &Path,
) -> Result<String, ManifestParseError> {
    match object.get(key) {
        None | Some(Value::Null) => Ok(String::new()),
        Some(Value::String(value)) => Ok(value.clone()),
        Some(_) => Err(ManifestParseError::new(
            path,
            format!("scryrs.json remote.{key} must be a string"),
        )),
    }
}

fn read_optional_u64(
    object: &serde_json::Map<String, Value>,
    key: &str,
    path: &Path,
) -> Result<Option<u64>, ManifestParseError> {
    match object.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(value)) => value.as_u64().map(Some).ok_or_else(|| {
            ManifestParseError::new(
                path,
                format!("scryrs.json remote.{key} must be an unsigned integer"),
            )
        }),
        Some(_) => Err(ManifestParseError::new(
            path,
            format!("scryrs.json remote.{key} must be an unsigned integer"),
        )),
    }
}

fn find_nearest_ancestor(start: &Path, filename: &str) -> Option<PathBuf> {
    let mut current = Some(start);
    while let Some(path) = current {
        let candidate = path.join(filename);
        if candidate.is_file() {
            return Some(candidate);
        }
        current = path.parent();
    }
    None
}

fn live_mode_requested(manifest: &ManifestInspection) -> bool {
    non_empty_env("SCRYRS_REMOTE_INGEST_URL").is_some()
        || manifest
            .remote
            .as_ref()
            .map(|remote| !remote.ingest_url.trim().is_empty())
            .unwrap_or(false)
}

fn active_remote_env_overrides() -> Vec<String> {
    [
        "SCRYRS_REMOTE_INGEST_URL",
        "SCRYRS_REPOSITORY_ID",
        "SCRYRS_WORKSPACE_ID",
        "SCRYRS_AGENT_ID",
        "SCRYRS_REMOTE_TIMEOUT_MS",
    ]
    .into_iter()
    .filter_map(non_empty_env)
    .collect()
}

fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(|_| key.to_string())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DoctorReport {
    schema_version: String,
    command: String,
    version: String,
    mode: ResolvedMode,
    overall_status: Severity,
    command_surface: CommandSurface,
    findings: Vec<DoctorFinding>,
    docs_links: Vec<DocLink>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CommandSurface {
    commands: Vec<String>,
    features: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DoctorFinding {
    category: String,
    status: Severity,
    summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<Value>,
}

impl DoctorFinding {
    fn new(category: &str, status: Severity, summary: String, details: Option<Value>) -> Self {
        Self {
            category: category.to_string(),
            status,
            summary,
            details,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
enum Severity {
    Ok,
    Warn,
    Error,
}

impl Severity {
    fn max(self, other: Self) -> Self {
        if self >= other { self } else { other }
    }

    fn as_label(self) -> &'static str {
        match self {
            Severity::Ok => "ok",
            Severity::Warn => "warn",
            Severity::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ResolvedMode {
    Local,
    Live,
}

impl ResolvedMode {
    fn as_str(self) -> &'static str {
        match self {
            ResolvedMode::Local => "local",
            ResolvedMode::Live => "live",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct DocLink {
    label: &'static str,
    path: &'static str,
}

#[derive(Debug, Clone)]
struct ManifestInspection {
    state: ManifestState,
    remote: Option<ManifestRemote>,
}

impl ManifestInspection {
    fn resolved_ingest_url(&self) -> Option<String> {
        std::env::var("SCRYRS_REMOTE_INGEST_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| self.remote.as_ref().map(|remote| remote.ingest_url.clone()))
            .filter(|value| !value.trim().is_empty())
    }
}

#[derive(Debug, Clone)]
enum ManifestState {
    Missing {
        env_overrides: Vec<String>,
    },
    Ok {
        path: PathBuf,
        env_overrides: Vec<String>,
    },
    Error {
        path: Option<PathBuf>,
        message: String,
    },
}

#[derive(Debug, Clone, Default)]
struct ManifestRemote {
    ingest_url: String,
    repository_id: String,
    workspace_id: String,
    agent_id: String,
    timeout_ms: u64,
}

#[derive(Debug)]
struct ManifestParseError {
    path: PathBuf,
    message: String,
}

impl ManifestParseError {
    fn new(path: &Path, message: String) -> Self {
        Self {
            path: path.to_path_buf(),
            message,
        }
    }

    fn into_inspection(self) -> ManifestInspection {
        ManifestInspection {
            state: ManifestState::Error {
                path: Some(self.path),
                message: self.message,
            },
            remote: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fs;

    use scryrs_core::EventStore;
    use scryrs_types::{
        DocRetrievedPayload, Outcome, SCHEMA_VERSION, TraceEvent, TraceEventPayload, TraceEventType,
    };

    use crate::run_with_writers;
    use crate::test_support::with_cwd;

    fn run_doctor(args: &[&str], repo_root: &Path) -> (i32, String, String) {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let exit = with_captured_cwd(repo_root, || {
            run_with_writers(args.iter().copied(), &mut out, &mut err)
        });
        (
            exit,
            String::from_utf8(out).unwrap_or_else(|error| panic!("stdout utf8: {error}")),
            String::from_utf8(err).unwrap_or_else(|error| panic!("stderr utf8: {error}")),
        )
    }

    fn with_captured_cwd<T>(repo_root: &Path, action: impl FnOnce() -> T) -> T {
        let mut result = None;
        with_cwd(repo_root, || {
            result = Some(action());
        });
        match result {
            Some(value) => value,
            None => panic!("with_captured_cwd must set a result"),
        }
    }

    fn write_trace_event(repo_root: &Path) {
        let store_path = repo_root.join(CANONICAL_STORE_PATH);
        let mut store =
            EventStore::open(&store_path).unwrap_or_else(|error| panic!("open store: {error}"));
        let event = TraceEvent {
            schema_version: SCHEMA_VERSION.into(),
            timestamp: "2026-06-29T12:00:00Z".into(),
            session_id: "session-1".into(),
            event_type: TraceEventType::DocRetrieved,
            tool_name: Some("read".into()),
            payload: TraceEventPayload::DocRetrieved(DocRetrievedPayload {
                doc_ref: "docs/guide.md".into(),
            }),
            outcome: Outcome::Success,
        };
        store
            .append(&event)
            .unwrap_or_else(|error| panic!("append event: {error}"));
    }

    fn install_claude_hook(repo_root: &Path) {
        let claude_dir = repo_root.join(".claude");
        fs::create_dir_all(&claude_dir).unwrap_or_else(|error| panic!("create .claude: {error}"));
        fs::write(
            claude_dir.join("settings.json"),
            serde_json::to_string_pretty(&json!({
                "hooks": {
                    "PreToolUse": [
                        {
                            "matcher": "",
                            "hooks": [
                                { "type": "command", "command": CLAUDE_HOOK_COMMAND }
                            ]
                        }
                    ]
                }
            }))
            .unwrap_or_else(|error| panic!("serialize settings: {error}")),
        )
        .unwrap_or_else(|error| panic!("write settings: {error}"));
    }

    fn install_pi_hook(repo_root: &Path) {
        let pi_dir = repo_root.join(".pi/extensions/scryrs");
        fs::create_dir_all(&pi_dir).unwrap_or_else(|error| panic!("create pi dir: {error}"));
        fs::write(pi_dir.join("index.ts"), "// installed")
            .unwrap_or_else(|error| panic!("write pi hook: {error}"));
    }

    fn read_json_report(stdout: &str) -> Value {
        serde_json::from_str(stdout)
            .unwrap_or_else(|error| panic!("parse doctor json: {error}\n{stdout}"))
    }

    #[test]
    fn default_output_is_human_readable() {
        let temp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
        let (exit, stdout, stderr) = run_doctor(&["doctor"], temp.path());

        assert_eq!(exit, 0);
        assert!(stderr.is_empty());
        assert!(stdout.contains("scryrs doctor"));
        assert!(stdout.contains("Resolved mode: local"));
        assert!(stdout.contains("Docs links"));
    }

    #[test]
    fn json_output_reports_no_config_local_empty_store() {
        let temp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
        let (exit, stdout, stderr) = run_doctor(&["doctor", "--json"], temp.path());

        assert_eq!(exit, 0);
        assert!(stderr.is_empty());
        let report = read_json_report(&stdout);
        assert_eq!(report["command"], "doctor");
        assert_eq!(report["mode"], "local");
        assert_eq!(report["overallStatus"], "warn");
        let local_store = report["findings"]
            .as_array()
            .and_then(|findings| {
                findings
                    .iter()
                    .find(|finding| finding["category"] == "local_store")
            })
            .unwrap_or_else(|| panic!("missing local_store finding: {report}"));
        assert_eq!(local_store["status"], "warn");
        assert!(
            local_store["summary"]
                .as_str()
                .unwrap_or_default()
                .contains("not initialized")
        );
    }

    #[test]
    fn initialized_local_workspace_with_hooks_is_healthy() {
        let temp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
        write_trace_event(temp.path());
        install_claude_hook(temp.path());
        install_pi_hook(temp.path());

        let (exit, stdout, stderr) = run_doctor(&["doctor", "--json"], temp.path());

        assert_eq!(exit, 0);
        assert!(stderr.is_empty());
        let report = read_json_report(&stdout);
        // Live is the default. A local workspace with no resolvable remote
        // config still warns on the liveConfig finding, so overall is "warn"
        // (not "ok"), while the local store and hook findings remain "ok".
        assert_eq!(report["overallStatus"], "warn");
        let live_config = report["findings"]
            .as_array()
            .and_then(|findings| {
                findings
                    .iter()
                    .find(|finding| finding["category"] == "liveConfig")
            })
            .unwrap_or_else(|| panic!("missing liveConfig finding: {report}"));
        assert_eq!(live_config["status"], "warn");
        assert!(
            live_config["summary"]
                .as_str()
                .unwrap_or_default()
                .contains("live mode is the default"),
            "liveConfig summary must explain the live default, got: {live_config}"
        );
        let local_store = report["findings"]
            .as_array()
            .and_then(|findings| {
                findings
                    .iter()
                    .find(|finding| finding["category"] == "local_store")
            })
            .unwrap_or_else(|| panic!("missing local_store finding: {report}"));
        assert_eq!(local_store["status"], "ok");
        let claude = report["findings"]
            .as_array()
            .and_then(|findings| {
                findings
                    .iter()
                    .find(|finding| finding["category"] == "claude_code_hook")
            })
            .unwrap_or_else(|| panic!("missing claude finding: {report}"));
        assert_eq!(claude["status"], "ok");
        let pi = report["findings"]
            .as_array()
            .and_then(|findings| {
                findings
                    .iter()
                    .find(|finding| finding["category"] == "pi_hook")
            })
            .unwrap_or_else(|| panic!("missing pi finding: {report}"));
        assert_eq!(pi["status"], "ok");
    }

    #[test]
    fn live_config_with_unreachable_server_is_structural_error() {
        let temp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
        fs::write(
            temp.path().join("scryrs.json"),
            serde_json::to_string_pretty(&json!({
                "remote": {
                    "ingest_url": "http://127.0.0.1:1",
                    "repository_id": "repo-a",
                    "workspace_id": "workspace-a",
                    "agent_id": "agent-a",
                    "timeout_ms": 50
                }
            }))
            .unwrap_or_else(|error| panic!("serialize manifest: {error}")),
        )
        .unwrap_or_else(|error| panic!("write manifest: {error}"));

        let (exit, stdout, stderr) = run_doctor(&["doctor", "--json"], temp.path());

        assert_eq!(exit, 2);
        assert!(stderr.is_empty());
        let report = read_json_report(&stdout);
        assert_eq!(report["mode"], "live");
        let live_server = report["findings"]
            .as_array()
            .and_then(|findings| {
                findings
                    .iter()
                    .find(|finding| finding["category"] == "live_server")
            })
            .unwrap_or_else(|| panic!("missing live_server finding: {report}"));
        assert_eq!(live_server["status"], "error");
        assert!(
            live_server["summary"]
                .as_str()
                .unwrap_or_default()
                .contains("unreachable")
        );
    }

    #[test]
    fn live_config_missing_remote_identity_is_structural_error() {
        let temp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
        fs::write(
            temp.path().join("scryrs.json"),
            serde_json::to_string_pretty(&json!({
                "remote": {
                    "ingest_url": "http://127.0.0.1:8081",
                    "repository_id": "repo-a"
                }
            }))
            .unwrap_or_else(|error| panic!("serialize manifest: {error}")),
        )
        .unwrap_or_else(|error| panic!("write manifest: {error}"));

        let (exit, stdout, stderr) = run_doctor(&["doctor", "--json"], temp.path());

        assert_eq!(exit, 2);
        assert!(stderr.is_empty());
        let report = read_json_report(&stdout);
        let mode = report["findings"]
            .as_array()
            .and_then(|findings| {
                findings
                    .iter()
                    .find(|finding| finding["category"] == "mode")
            })
            .unwrap_or_else(|| panic!("missing mode finding: {report}"));
        assert_eq!(mode["status"], "error");
        assert!(
            mode["summary"]
                .as_str()
                .unwrap_or_default()
                .contains("workspace_id")
        );
    }

    #[test]
    fn claude_hook_presence_is_reported() {
        let temp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
        install_claude_hook(temp.path());

        let (exit, stdout, stderr) = run_doctor(&["doctor", "--json"], temp.path());

        assert_eq!(exit, 0);
        assert!(stderr.is_empty());
        let report = read_json_report(&stdout);
        let claude = report["findings"]
            .as_array()
            .and_then(|findings| {
                findings
                    .iter()
                    .find(|finding| finding["category"] == "claude_code_hook")
            })
            .unwrap_or_else(|| panic!("missing claude finding: {report}"));
        assert_eq!(claude["status"], "ok");
    }

    #[test]
    fn pi_hook_presence_is_reported() {
        let temp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
        install_pi_hook(temp.path());

        let (exit, stdout, stderr) = run_doctor(&["doctor", "--json"], temp.path());

        assert_eq!(exit, 0);
        assert!(stderr.is_empty());
        let report = read_json_report(&stdout);
        let pi = report["findings"]
            .as_array()
            .and_then(|findings| {
                findings
                    .iter()
                    .find(|finding| finding["category"] == "pi_hook")
            })
            .unwrap_or_else(|| panic!("missing pi finding: {report}"));
        assert_eq!(pi["status"], "ok");
    }

    #[test]
    fn corrupt_config_is_reported_as_error() {
        let temp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
        fs::write(temp.path().join("scryrs.json"), "{not-json")
            .unwrap_or_else(|error| panic!("write manifest: {error}"));

        let (exit, stdout, stderr) = run_doctor(&["doctor", "--json"], temp.path());

        assert_eq!(exit, 2);
        assert!(stderr.is_empty());
        let report = read_json_report(&stdout);
        let config = report["findings"]
            .as_array()
            .and_then(|findings| {
                findings
                    .iter()
                    .find(|finding| finding["category"] == "config")
            })
            .unwrap_or_else(|| panic!("missing config finding: {report}"));
        assert_eq!(config["status"], "error");
        assert!(
            config["summary"]
                .as_str()
                .unwrap_or_default()
                .contains("malformed JSON")
        );
    }

    #[test]
    fn unsupported_remote_shape_is_reported_as_error() {
        let temp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
        fs::write(
            temp.path().join("scryrs.json"),
            serde_json::to_string_pretty(&json!({ "remote": [] }))
                .unwrap_or_else(|error| panic!("serialize manifest: {error}")),
        )
        .unwrap_or_else(|error| panic!("write manifest: {error}"));

        let (exit, stdout, stderr) = run_doctor(&["doctor", "--json"], temp.path());

        assert_eq!(exit, 2);
        assert!(stderr.is_empty());
        let report = read_json_report(&stdout);
        let config = report["findings"]
            .as_array()
            .and_then(|findings| {
                findings
                    .iter()
                    .find(|finding| finding["category"] == "config")
            })
            .unwrap_or_else(|| panic!("missing config finding: {report}"));
        assert_eq!(config["status"], "error");
        assert_eq!(
            config["summary"],
            "scryrs.json remote field must be a JSON object"
        );
    }
}
