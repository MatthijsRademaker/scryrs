//! Deterministic hook installer (`scryrs init --agent <NAME>`).
//!
//! Two integration shapes:
//!
//! - **claude-code**: no hook file. The installer create-or-merges
//!   `.claude/settings.json` with a native `PreToolUse` command hook invoking
//!   `scryrs hook claude-code`. No JavaScript / node runtime is involved.
//! - **pi**: an in-process extension. The slimmed `hooks/pi/index.ts` is
//!   embedded at compile time via `include_str!()` and written to
//!   `.pi/extensions/pi-trace/index.ts`.
//!
//! Two install modes:
//!
//! - **local** (default): scaffolds `.scryrs/scryrs.db` for local SQLite trace storage.
//! - **live**: creates or merges `scryrs.json` `remote` section for remote ingest,
//!   does not create a local database, and requires explicit remote identity inputs.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::remote_config::{self, RemoteConfig, RemoteOverrides};

/// The native Claude Code `PreToolUse` hook command.
const CLAUDE_HOOK_COMMAND: &str = "scryrs hook claude-code";

/// Deterministic post-install instructions for Claude Code (no `.mjs`).
const CLAUDE_NEXT_STEPS: &str = concat!(
    "scryrs Claude Code hook configured in .claude/settings.json\n",
    "The PreToolUse hook command is: scryrs hook claude-code\n",
    "\n",
    "Next steps:\n",
    "  1. Ensure scryrs is on your PATH.\n",
    "  2. Restart your Claude Code session for the hook to take effect.\n",
);

// ---------------------------------------------------------------------------
// File-based harness registry (Pi). Claude Code is handled separately.
// ---------------------------------------------------------------------------

/// One entry in the file-based harness registry.
struct HarnessEntry {
    /// Canonical agent name (lowercase, e.g. "pi").
    agent_name: &'static str,
    /// Full hook source embedded at compile time.
    source_asset: &'static str,
    /// Target directory relative to the base dir (created if missing).
    target_dir: &'static str,
    /// Target filename within `target_dir`.
    target_filename: &'static str,
    /// Deterministic post-install instructions printed to stdout on success.
    next_steps: &'static str,
}

/// File-based harness registry. Claude Code is NOT here — it merges JSON.
const FILE_HARNESS_REGISTRY: &[HarnessEntry] = &[HarnessEntry {
    agent_name: "pi",
    // Path relative to this file: 3 levels up to repo root, then hooks/.
    source_asset: include_str!("../../../hooks/pi/index.ts"),
    target_dir: ".pi/extensions/pi-trace",
    target_filename: "index.ts",
    next_steps: concat!(
        "scryrs Pi trace hook installed to .pi/extensions/pi-trace/index.ts\n",
        "\n",
        "Next steps:\n",
        "  1. Ensure scryrs is on your PATH.\n",
        "  2. Reload Pi (e.g., /reload) to activate the hook.\n",
        "  3. The thin shim forwards raw Pi tool_result/session_start events to\n",
        "     `scryrs hook pi`; all tool→event translation lives in scryrs.\n",
    ),
}];

/// All supported harness names, for the unsupported-harness error message.
/// Ordered deterministically (alphabetical).
const SUPPORTED_HARNESSES: &[&str] = &["claude-code", "pi"];

/// `.gitignore` written into `.scryrs/` so runtime trace data is never
/// committed. Ignores everything in the directory except the ignore file
/// itself (covers `scryrs.db`, `hotspots.json`, and `hooks/`).
const SCRYRS_GITIGNORE: &str = concat!(
    "# scryrs runtime data — do not commit\n",
    "*\n",
    "!.gitignore\n",
);

/// Deterministic confirmation printed after the `.scryrs/` store is scaffolded.
const SCRYRS_SCAFFOLD_NOTE: &str =
    "Initialized .scryrs/ trace store (.scryrs/scryrs.db, .scryrs/.gitignore)\n\n";

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Install mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InitMode {
    /// Local SQLite trace store (default).
    Local,
    /// Remote ingest via scryrs server (writes project scryrs.json).
    Live,
}

/// Execute the `scryrs init --agent <NAME>` subcommand.
///
/// Returns the process exit code:
/// - 0: successful installation
/// - 1: I/O error (cannot create directory, cannot write file)
/// - 2: usage error (unsupported harness, collision, self-install refusal,
///        invalid mode, or missing/invalid live-mode configuration)
#[allow(clippy::too_many_arguments)]
pub fn execute_init(
    out: &mut impl Write,
    err: &mut impl Write,
    agent_name: &str,
    mode: InitMode,
    ingest_url: &str,
    workspace_id: &str,
    agent_id: &str,
    repository_id: Option<&str>,
) -> i32 {
    let cwd = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => {
            let _ = writeln!(err, "scryrs init: cannot determine current directory: {e}");
            return 1;
        }
    };

    let source_root = detect_scryrs_source_checkout(&cwd);

    // Determine the target base directory.
    // For Pi inside the scryrs source checkout: use the detected checkout root.
    // For all other cases (consumer projects, Claude Code, etc.): use CWD.
    let target_base: &Path = match (&source_root, agent_name) {
        (Some(root), "pi") => root,
        _ => &cwd,
    };

    // Harness-specific self-install policy:
    // - Live mode is always refused in the scryrs source checkout.
    // - In local mode, only Pi may install into the source checkout.
    if source_root.is_some() {
        if mode == InitMode::Live {
            let _ = writeln!(
                err,
                "scryrs init: live mode is not allowed in the scryrs source repository"
            );
            let _ = writeln!(
                err,
                "Live mode is the default and configures a consumer project for remote ingest."
            );
            if agent_name == "pi" {
                let _ = writeln!(
                    err,
                    "Use `scryrs init --agent pi --mode local` here for dogfooding, or run live init from a consumer project."
                );
            } else {
                let _ = writeln!(
                    err,
                    "Run live init from your target project directory instead."
                );
            }
            return 2;
        }
        if agent_name != "pi" {
            let _ = writeln!(
                err,
                "scryrs init: refusing to install into the scryrs source repository"
            );
            let _ = writeln!(
                err,
                "Consumer config must not be written into the scryrs source repo."
            );
            let _ = writeln!(
                err,
                "Run scryrs init from your target project directory instead."
            );
            return 2;
        }
    }

    // Live-mode resolution: merge CLI flags over the shared precedence chain
    // (flags > env > .scryrs/.env > scryrs.json remote) before any filesystem
    // writes. Unresolved required config fails fast with remediation guidance.
    let mut resolved_config: Option<RemoteConfig> = None;
    if mode == InitMode::Live {
        let overrides = RemoteOverrides {
            ingest_url: opt_nonempty(ingest_url),
            repository_id: repository_id.and_then(opt_nonempty),
            workspace_id: opt_nonempty(workspace_id),
            agent_id: opt_nonempty(agent_id),
            timeout_ms: None,
        };
        match remote_config::resolve_remote_config_with(Some(&cwd), &overrides) {
            Ok(Some(resolved)) => resolved_config = Some(resolved.config),
            Ok(None) => {
                remote_config::write_missing_config_guidance(err, "init", "ingest_url");
                return 2;
            }
            Err(e) => {
                remote_config::write_missing_config_guidance(err, "init", e.missing_field());
                return 2;
            }
        }
    }

    // Resolve the install action, rejecting an unsupported harness *before*
    // scaffolding so an unknown name leaves the filesystem untouched.
    enum Action {
        ClaudeCode,
        File(&'static HarnessEntry),
    }
    let action = match agent_name {
        "claude-code" => Action::ClaudeCode,
        other => match find_harness(other) {
            Some(entry) => Action::File(entry),
            None => {
                let _ = writeln!(
                    err,
                    "scryrs init: '{agent_name}' is not a supported harness"
                );
                let _ = write!(err, "Supported harnesses: ");
                for (i, name) in SUPPORTED_HARNESSES.iter().enumerate() {
                    if i > 0 {
                        let _ = write!(err, ", ");
                    }
                    let _ = write!(err, "{name}");
                }
                let _ = writeln!(err);
                return 2;
            }
        },
    };

    // Scaffold `.scryrs/` runtime directory. Local mode creates the full
    // trace store; live mode creates warning-log directories only.
    let code = scaffold_scryrs(out, err, target_base, mode);
    if code != 0 {
        return code;
    }

    // Live-mode: write or merge the project's `scryrs.json` `remote` section
    // and scaffold a gitignored `.scryrs/.env` template (without clobbering
    // any values the operator has already set).
    if mode == InitMode::Live {
        let config = match resolved_config.as_ref() {
            Some(c) => c,
            None => {
                let _ = writeln!(err, "scryrs init: internal error: live config not resolved");
                return 1;
            }
        };
        let code = write_live_manifest(err, target_base, config);
        if code != 0 {
            return code;
        }
        let code = scaffold_env_template(err, &target_base.join(".scryrs"), config);
        if code != 0 {
            return code;
        }
    }

    match action {
        Action::ClaudeCode => {
            let code = install_claude_code(out, err, target_base);
            if code != 0 {
                return code;
            }
        }
        Action::File(entry) => {
            let code = install_file_harness(out, err, target_base, entry);
            if code != 0 {
                return code;
            }
        }
    }

    // Print mode-specific next-step text.
    match mode {
        InitMode::Local => {
            // next-step text is already printed by install functions
        }
        InitMode::Live => {
            let _ = write!(out, "{}", LIVE_NEXT_STEPS);
        }
    }

    0
}

// ---------------------------------------------------------------------------
// .scryrs/ runtime store scaffolding (harness-agnostic)
// ---------------------------------------------------------------------------

/// Scaffold the `.scryrs/` runtime directory at `base`.
///
/// Local mode: creates `.scryrs/scryrs.db`, `.scryrs/.gitignore`.
/// Live mode: creates `.scryrs/`, `.scryrs/.gitignore`, `.scryrs/hooks/` only
/// (no local database).
///
/// Idempotent: existing files are preserved (the store is opened, never clobbered).
///
/// Returns 0 on success, or 1 on an I/O error.
fn scaffold_scryrs(out: &mut impl Write, err: &mut impl Write, base: &Path, mode: InitMode) -> i32 {
    let scryrs_dir = base.join(".scryrs");
    if let Err(e) = fs::create_dir_all(&scryrs_dir) {
        let _ = writeln!(
            err,
            "scryrs init: cannot create {}: {e}",
            scryrs_dir.display()
        );
        return 1;
    }

    // Write `.gitignore` only when absent, so a user's edits are never clobbered.
    let gitignore_path = scryrs_dir.join(".gitignore");
    if !gitignore_path.exists() {
        if let Err(e) = fs::write(&gitignore_path, SCRYRS_GITIGNORE) {
            let _ = writeln!(
                err,
                "scryrs init: cannot write {}: {e}",
                gitignore_path.display()
            );
            return 1;
        }
    }

    match mode {
        InitMode::Local => {
            // Initialize the trace store schema (idempotent; preserves existing data).
            if let Err(reason) = init_trace_store(&scryrs_dir.join("scryrs.db")) {
                let _ = writeln!(err, "scryrs init: cannot initialize trace store: {reason}");
                return 1;
            }
            let _ = write!(out, "{SCRYRS_SCAFFOLD_NOTE}");
        }
        InitMode::Live => {
            // Create `.scryrs/hooks/` for warning logs; no local database.
            let hooks_dir = scryrs_dir.join("hooks");
            if let Err(e) = fs::create_dir_all(&hooks_dir) {
                let _ = writeln!(
                    err,
                    "scryrs init: cannot create {}: {e}",
                    hooks_dir.display()
                );
                return 1;
            }
            let _ = write!(
                out,
                "Initialized .scryrs/ for live-mode warning logs (.scryrs/.gitignore, .scryrs/hooks/)\n\n"
            );
        }
    }
    0
}

/// Open (creating + initializing the schema) the canonical trace store.
#[cfg(feature = "core")]
fn init_trace_store(db_path: &Path) -> Result<(), String> {
    scryrs_core::EventStore::open(db_path)
        .map(|_store| ())
        .map_err(|e| e.to_string())
}

/// Without the `core` feature the schema cannot be created here; the hook
/// initializes the store lazily on the first recorded event.
#[cfg(not(feature = "core"))]
fn init_trace_store(_db_path: &Path) -> Result<(), String> {
    Ok(())
}

// ---------------------------------------------------------------------------
// Claude Code: create-or-merge .claude/settings.json
// ---------------------------------------------------------------------------

/// Create-or-merge `.claude/settings.json` with the native command hook.
/// Preserves unrelated keys; idempotent across re-runs.
fn install_claude_code(out: &mut impl Write, err: &mut impl Write, target_base: &Path) -> i32 {
    use serde_json::{Map, Value, json};

    let claude_dir = target_base.join(".claude");
    if let Err(e) = fs::create_dir_all(&claude_dir) {
        let _ = writeln!(
            err,
            "scryrs init: cannot create {}: {e}",
            claude_dir.display()
        );
        return 1;
    }

    let settings_path = claude_dir.join("settings.json");

    // Load existing settings (or start from an empty object).
    let mut root: Value = if settings_path.exists() {
        match fs::read_to_string(&settings_path) {
            Ok(contents) => match serde_json::from_str(&contents) {
                Ok(Value::Object(map)) => Value::Object(map),
                Ok(_other) => {
                    let _ = writeln!(
                        err,
                        "scryrs init: {} is not a JSON object; refusing to overwrite",
                        settings_path.display()
                    );
                    return 2;
                }
                Err(e) => {
                    let _ = writeln!(
                        err,
                        "scryrs init: cannot parse {}: {e}",
                        settings_path.display()
                    );
                    return 2;
                }
            },
            Err(e) => {
                let _ = writeln!(
                    err,
                    "scryrs init: cannot read {}: {e}",
                    settings_path.display()
                );
                return 1;
            }
        }
    } else {
        Value::Object(Map::new())
    };

    // root must be an object at this point.
    let obj = match root.as_object_mut() {
        Some(o) => o,
        None => {
            let _ = writeln!(err, "scryrs init: internal error building settings object");
            return 1;
        }
    };

    // Ensure `hooks` is an object.
    let hooks = obj
        .entry("hooks")
        .or_insert_with(|| Value::Object(Map::new()));
    let hooks = match hooks.as_object_mut() {
        Some(h) => h,
        None => {
            let _ = writeln!(
                err,
                "scryrs init: existing \"hooks\" is not an object; refusing to overwrite"
            );
            return 2;
        }
    };

    // Ensure `PreToolUse` is an array.
    let pre = hooks
        .entry("PreToolUse")
        .or_insert_with(|| Value::Array(Vec::new()));
    let pre = match pre.as_array_mut() {
        Some(a) => a,
        None => {
            let _ = writeln!(
                err,
                "scryrs init: existing \"PreToolUse\" is not an array; refusing to overwrite"
            );
            return 2;
        }
    };

    // Idempotency: bail out unchanged if our command is already registered.
    let already = pre.iter().any(|entry| {
        entry
            .get("hooks")
            .and_then(Value::as_array)
            .map(|hs| {
                hs.iter().any(|h| {
                    h.get("type").and_then(Value::as_str) == Some("command")
                        && h.get("command").and_then(Value::as_str) == Some(CLAUDE_HOOK_COMMAND)
                })
            })
            .unwrap_or(false)
    });

    if !already {
        pre.push(json!({
            "matcher": "",
            "hooks": [ { "type": "command", "command": CLAUDE_HOOK_COMMAND } ]
        }));
    }

    // Serialize with a trailing newline for stable on-disk form.
    let serialized = match serde_json::to_string_pretty(&root) {
        Ok(s) => s,
        Err(e) => {
            let _ = writeln!(err, "scryrs init: cannot serialize settings: {e}");
            return 1;
        }
    };
    if let Err(e) = fs::write(&settings_path, format!("{serialized}\n")) {
        let _ = writeln!(
            err,
            "scryrs init: cannot write {}: {e}",
            settings_path.display()
        );
        return 1;
    }

    let _ = write!(out, "{CLAUDE_NEXT_STEPS}");
    0
}

// ---------------------------------------------------------------------------
// File-based harnesses (Pi)
// ---------------------------------------------------------------------------

fn install_file_harness(
    out: &mut impl Write,
    err: &mut impl Write,
    target_base: &Path,
    entry: &HarnessEntry,
) -> i32 {
    let target_dir = target_base.join(entry.target_dir);
    if let Err(e) = fs::create_dir_all(&target_dir) {
        let _ = writeln!(
            err,
            "scryrs init: cannot create {}: {e}",
            target_dir.display()
        );
        return 1;
    }

    let target_path = target_dir.join(entry.target_filename);
    if target_path.exists() {
        let _ = writeln!(err, "scryrs init: {} already exists", target_path.display());
        let _ = writeln!(err, "Remove the file manually and rerun scryrs init.");
        let _ = writeln!(err, "See `scryrs --help`");
        return 2;
    }

    if let Err(e) = fs::write(&target_path, entry.source_asset) {
        let _ = writeln!(
            err,
            "scryrs init: cannot write {}: {e}",
            target_path.display()
        );
        return 1;
    }

    let _ = write!(out, "{}", entry.next_steps);
    0
}

// ---------------------------------------------------------------------------
// Self-install boundary detection
// ---------------------------------------------------------------------------

/// Walk parent directories from `cwd` looking for the scryrs source checkout.
///
/// Returns `Some(root)` when BOTH markers are found in the same ancestor directory:
/// 1. `Cargo.toml` exists and contains the string `scryrs-cli`, AND
/// 2. `hooks/claude-code/` exists as a subdirectory.
///
/// The returned path is the directory containing both markers (the checkout root).
/// Returns `None` when no matching ancestor directory is found.
///
/// The dual-marker heuristic prevents false positives: a user project would
/// need to both include scryrs-cli as a workspace member AND clone the
/// hooks/claude-code/ directory structure.
fn detect_scryrs_source_checkout(cwd: &Path) -> Option<PathBuf> {
    let mut current = Some(cwd);

    while let Some(dir) = current {
        let cargo_toml = dir.join("Cargo.toml");
        let hooks_claude_code = dir.join("hooks").join("claude-code");

        if cargo_toml.exists() && hooks_claude_code.is_dir() {
            // Both structural markers present — check content.
            if let Ok(contents) = fs::read_to_string(&cargo_toml) {
                if contents.contains("scryrs-cli") {
                    return Some(dir.to_path_buf());
                }
            }
        }

        current = dir.parent();
    }

    None
}

// ---------------------------------------------------------------------------
// Live-mode: validation and project manifest
// ---------------------------------------------------------------------------

/// Deterministic post-install instructions for live mode.
const LIVE_NEXT_STEPS: &str = concat!(
    "scryrs live-mode remote ingest configured (scryrs.json remote + .scryrs/.env)\n",
    "\n",
    "Live mode is the default — every `scryrs hook` invocation will submit events\n",
    "to the configured remote server. No local .scryrs/scryrs.db is created.\n",
    "Remote identity is read from .scryrs/.env (gitignored); edit it to adjust.\n",
    "\n",
    "Next steps:\n",
    "  1. Ensure the live ingest server is running (start with `scryrs server`).\n",
    "  2. Verify connectivity: check that the server is reachable at the configured URL.\n",
    "  3. For Docker-based deployment, use the provided docker-compose.yml to run\n",
    "     `scryrs-server` as a shared network service. Run live-mode init in each\n",
    "     agent container pointing at http://scryrs-server:8081.\n",
    "  4. Restart or reload your agent harness for the hook to take effect.\n",
);

/// Coerce a CLI flag value to `Some(trimmed)` when non-empty, else `None`.
fn opt_nonempty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Header written at the top of a freshly scaffolded `.scryrs/.env`.
const SCRYRS_ENV_HEADER: &str = concat!(
    "# scryrs remote ingest configuration (gitignored)\n",
    "# Populate these for live mode, or run commands with --mode local.\n",
);

/// Create-or-merge a `.scryrs/.env` template with the resolved `SCRYRS_REMOTE_*`
/// values. Existing keys are preserved verbatim — only missing keys are added,
/// so operator edits are never clobbered. `.scryrs/.gitignore` already ignores
/// everything except itself, so `.env` is covered without extra handling.
///
/// Returns 0 on success, or 1 on an I/O error.
fn scaffold_env_template(err: &mut impl Write, scryrs_dir: &Path, config: &RemoteConfig) -> i32 {
    use std::collections::HashSet;

    let env_path = scryrs_dir.join(".env");
    let existing = fs::read_to_string(&env_path).unwrap_or_default();

    let mut present: HashSet<String> = HashSet::new();
    for line in existing.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, _)) = trimmed.split_once('=') {
            present.insert(key.trim().to_string());
        }
    }

    let entries = [
        ("SCRYRS_REMOTE_INGEST_URL", config.ingest_url.as_str()),
        ("SCRYRS_REPOSITORY_ID", config.repository_id.as_str()),
        ("SCRYRS_WORKSPACE_ID", config.workspace_id.as_str()),
        ("SCRYRS_AGENT_ID", config.agent_id.as_str()),
    ];

    let mut contents = existing.clone();
    if contents.is_empty() {
        contents.push_str(SCRYRS_ENV_HEADER);
    } else if !contents.ends_with('\n') {
        contents.push('\n');
    }

    let mut appended = false;
    for (key, value) in entries {
        if !present.contains(key) {
            contents.push_str(&format!("{key}={value}\n"));
            appended = true;
        }
    }

    // Only write when we created the file or added a missing key.
    if existing.is_empty() || appended {
        if let Err(e) = fs::write(&env_path, contents) {
            let _ = writeln!(err, "scryrs init: cannot write {}: {e}", env_path.display());
            return 1;
        }
    }

    0
}

/// Write or merge the `remote` section of the target project's `scryrs.json`.
///
/// Creates the file if missing; merges into existing manifest, preserving
/// unrelated top-level keys. Refuses to overwrite a non-object `scryrs.json`.
///
/// Returns 0 on success, 1 on I/O error, 2 on parse/format conflict.
fn write_live_manifest(err: &mut impl Write, target_base: &Path, config: &RemoteConfig) -> i32 {
    use serde_json::{Map, Value};

    let manifest_path = target_base.join("scryrs.json");

    let ingest_url = config.ingest_url.as_str();
    let workspace_id = config.workspace_id.as_str();
    let agent_id = config.agent_id.as_str();
    let repo_id = config.repository_id.clone();

    // Load existing manifest (or start from empty object).
    let mut root: Value = if manifest_path.exists() {
        match std::fs::read_to_string(&manifest_path) {
            Ok(contents) => match serde_json::from_str(&contents) {
                Ok(Value::Object(map)) => Value::Object(map),
                Ok(_other) => {
                    let _ = writeln!(
                        err,
                        "scryrs init: {} is not a JSON object; refusing to overwrite",
                        manifest_path.display()
                    );
                    return 2;
                }
                Err(e) => {
                    let _ = writeln!(
                        err,
                        "scryrs init: cannot parse {}: {e}",
                        manifest_path.display()
                    );
                    return 2;
                }
            },
            Err(e) => {
                let _ = writeln!(
                    err,
                    "scryrs init: cannot read {}: {e}",
                    manifest_path.display()
                );
                return 1;
            }
        }
    } else {
        Value::Object(Map::new())
    };

    // root must be an object.
    let obj = match root.as_object_mut() {
        Some(o) => o,
        None => {
            let _ = writeln!(
                err,
                "scryrs init: internal error building scryrs.json object"
            );
            return 1;
        }
    };

    // Build the remote section.
    let mut remote = Map::new();
    remote.insert(
        "ingest_url".to_string(),
        Value::String(ingest_url.to_string()),
    );
    remote.insert(
        "workspace_id".to_string(),
        Value::String(workspace_id.to_string()),
    );
    remote.insert("agent_id".to_string(), Value::String(agent_id.to_string()));
    remote.insert("repository_id".to_string(), Value::String(repo_id));

    obj.insert("remote".to_string(), Value::Object(remote));

    // Serialize with a trailing newline for stable on-disk form.
    let serialized = match serde_json::to_string_pretty(&root) {
        Ok(s) => s,
        Err(e) => {
            let _ = writeln!(err, "scryrs init: cannot serialize scryrs.json: {e}");
            return 1;
        }
    };
    if let Err(e) = std::fs::write(&manifest_path, format!("{serialized}\n")) {
        let _ = writeln!(
            err,
            "scryrs init: cannot write {}: {e}",
            manifest_path.display()
        );
        return 1;
    }

    0
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Linear scan through the file-based registry for a harness matching `agent_name`.
fn find_harness(agent_name: &str) -> Option<&'static HarnessEntry> {
    FILE_HARNESS_REGISTRY
        .iter()
        .find(|e| e.agent_name == agent_name)
}
