//! Deterministic hook installer (`scryrs init --agent <NAME>`).
//!
//! Hook source files are embedded at compile time via `include_str!()` so the
//! binary is self-contained and works without a `hooks/` source tree.

use std::fs;
use std::io::Write;
use std::path::Path;

// ---------------------------------------------------------------------------
// Harness registry — typed, deterministic, stable alphabetical order
// ---------------------------------------------------------------------------

/// One entry in the supported-harness registry.
struct HarnessEntry {
    /// Canonical agent name (lowercase, e.g. "claude-code", "pi").
    agent_name: &'static str,
    /// Full hook source embedded at compile time.
    source_asset: &'static str,
    /// Target directory relative to CWD (created if missing).
    target_dir: &'static str,
    /// Target filename within `target_dir`.
    target_filename: &'static str,
    /// Deterministic post-install instructions printed to stdout on success.
    next_steps: &'static str,
}

/// Deterministic harness registry.
///
/// Entries are ordered alphabetically by `agent_name`. Adding a new harness
/// is a matter of adding one struct literal and a corresponding
/// `include_str!()` call.
const HARNESS_REGISTRY: &[HarnessEntry] = &[
    HarnessEntry {
        agent_name: "claude-code",
        // Path relative to this file: 3 levels up to repo root, then hooks/.
        source_asset: include_str!("../../../hooks/claude-code/scryrs-hook.mjs"),
        target_dir: ".claude/hooks",
        target_filename: "scryrs-hook.mjs",
        next_steps: concat!(
            "scryrs Claude Code hook installed to .claude/hooks/scryrs-hook.mjs\n",
            "\n",
            "Next steps:\n",
            "  1. Ensure scryrs is on your PATH.\n",
            "  2. Create .claude/settings.json (if it doesn't exist) and add the hook configuration:\n",
            "     {\n",
            "       \"hooks\": {\n",
            "         \"PreToolUse\": [\n",
            "           {\n",
            "             \"matcher\": \"\",\n",
            "             \"hook\": \"node .claude/hooks/scryrs-hook.mjs\"\n",
            "           }\n",
            "         ]\n",
            "       }\n",
            "     }\n",
            "  3. Restart your Claude Code session for the hook to take effect.\n",
        ),
    },
    HarnessEntry {
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
            "  3. Verify the hook assumes standard input fields: ast_grep_search (input.query),\n",
            "     lsp_navigation (input.symbol). Adjust if your Pi version differs.\n",
        ),
    },
];

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Execute the `scryrs init --agent <NAME>` subcommand.
///
/// Returns the process exit code:
/// - 0: successful installation
/// - 1: I/O error (cannot create directory, cannot write file)
/// - 2: usage error (unsupported harness, collision, self-install refusal)
pub fn execute_init(out: &mut impl Write, err: &mut impl Write, agent_name: &str) -> i32 {
    // --- self-install boundary guard ---

    let cwd = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => {
            let _ = writeln!(err, "scryrs init: cannot determine current directory: {e}");
            return 1;
        }
    };

    if is_scryrs_source_checkout(&cwd) {
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

    // --- harness lookup ---

    let entry = match find_harness(agent_name) {
        Some(e) => e,
        None => {
            let _ = writeln!(
                err,
                "scryrs init: '{agent_name}' is not a supported harness"
            );
            let _ = write!(err, "Supported harnesses: ");
            for (i, h) in HARNESS_REGISTRY.iter().enumerate() {
                if i > 0 {
                    let _ = write!(err, ", ");
                }
                let _ = write!(err, "{}", h.agent_name);
            }
            let _ = writeln!(err);
            return 2;
        }
    };

    // --- claude-code: settings.json collision check ---

    if entry.agent_name == "claude-code" {
        let settings_path = Path::new(".claude/settings.json");
        if settings_path.exists() {
            let _ = writeln!(err, "scryrs init: .claude/settings.json already exists");
            let _ = writeln!(
                err,
                "The hook source will not be installed — add the hook configuration manually:"
            );
            let _ = writeln!(err, r#"Insert into .claude/settings.json under "hooks": "#,);
            let _ = writeln!(
                err,
                r#"  "PreToolUse": [{{"matcher": "", "hook": "node .claude/hooks/scryrs-hook.mjs"}}]"#,
            );
            return 2;
        }
    }

    // --- create target directory ---

    if let Err(e) = fs::create_dir_all(entry.target_dir) {
        let _ = writeln!(err, "scryrs init: cannot create {}: {e}", entry.target_dir);
        return 1;
    }

    // --- collision check (target file already exists) ---

    let target_path = Path::new(entry.target_dir).join(entry.target_filename);
    if target_path.exists() {
        let _ = writeln!(err, "scryrs init: {} already exists", target_path.display());
        let _ = writeln!(err, "Remove the file manually and rerun scryrs init.");
        let _ = writeln!(err, "See `scryrs --help`");
        return 2;
    }

    // --- write embedded hook source ---

    if let Err(e) = fs::write(&target_path, entry.source_asset) {
        let _ = writeln!(
            err,
            "scryrs init: cannot write {}: {e}",
            target_path.display()
        );
        return 1;
    }

    // --- success: print next steps ---

    let _ = write!(out, "{}", entry.next_steps);

    0
}

// ---------------------------------------------------------------------------
// Self-install boundary detection
// ---------------------------------------------------------------------------

/// Walk parent directories from `cwd` looking for the scryrs source checkout.
///
/// Returns `true` when BOTH markers are found in the same ancestor directory:
/// 1. `Cargo.toml` exists and contains the string `scryrs-cli`, AND
/// 2. `hooks/claude-code/` exists as a subdirectory.
///
/// The dual-marker heuristic prevents false positives: a user project would
/// need to both include scryrs-cli as a workspace member AND clone the
/// hooks/claude-code/ directory structure.
fn is_scryrs_source_checkout(cwd: &Path) -> bool {
    let mut current = Some(cwd);

    while let Some(dir) = current {
        let cargo_toml = dir.join("Cargo.toml");
        let hooks_claude_code = dir.join("hooks").join("claude-code");

        if cargo_toml.exists() && hooks_claude_code.is_dir() {
            // Both structural markers present — check content.
            if let Ok(contents) = fs::read_to_string(&cargo_toml) {
                if contents.contains("scryrs-cli") {
                    return true;
                }
            }
        }

        current = dir.parent();
    }

    false
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Linear scan through the registry for a harness matching `agent_name`.
fn find_harness(agent_name: &str) -> Option<&'static HarnessEntry> {
    HARNESS_REGISTRY.iter().find(|e| e.agent_name == agent_name)
}
