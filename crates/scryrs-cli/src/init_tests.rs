//! Tests for `scryrs init` — hook installation only.
//!
//! After the init/setup split, `init` installs the harness hook and nothing
//! else: it never writes `scryrs.json` or the `.scryrs/` scaffold, never
//! prompts, and cannot fail on missing ingest config. Transport configuration
//! is covered by `setup_tests`.

use crate::run_with_writers;
use crate::test_support::{snapshot_dir_or_file, with_cwd};

/// Parse `.claude/settings.json` under `base` into a JSON value.
fn read_settings(base: &std::path::Path) -> serde_json::Value {
    let path = base.join(".claude/settings.json");
    let contents =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&contents).unwrap_or_else(|e| panic!("parse settings.json: {e}"))
}

/// Count occurrences of the native `scryrs hook claude-code` command across all
/// PreToolUse hook entries.
fn count_native_hook(settings: &serde_json::Value) -> usize {
    settings["hooks"]["PreToolUse"]
        .as_array()
        .map(|entries| {
            entries
                .iter()
                .filter_map(|e| e["hooks"].as_array())
                .flatten()
                .filter(|h| h["type"] == "command" && h["command"] == "scryrs hook claude-code")
                .count()
        })
        .unwrap_or(0)
}

// --- init --agent claude-code creates settings.json with native hook ---

#[test]
fn init_agent_claude_code_creates_native_settings() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(["init", "--agent", "claude-code"], &mut out, &mut err),
            0
        );
        assert!(
            err.is_empty(),
            "stderr must be empty, got: {}",
            String::from_utf8_lossy(&err)
        );

        // The native command hook block is present exactly once.
        let settings = read_settings(dir.path());
        assert_eq!(count_native_hook(&settings), 1);

        // No .mjs file is ever written.
        assert!(
            !dir.path().join(".claude/hooks/scryrs-hook.mjs").exists(),
            "no .mjs file must be written"
        );
        assert!(
            !dir.path().join(".claude/hooks").exists(),
            "no .claude/hooks dir must be created"
        );
    });
}

#[test]
fn init_agent_claude_code_is_idempotent() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();
        assert_eq!(
            run_with_writers(["init", "--agent", "claude-code"], &mut out, &mut err),
            0
        );
        let mut out2 = Vec::new();
        let mut err2 = Vec::new();
        assert_eq!(
            run_with_writers(["init", "--agent", "claude-code"], &mut out2, &mut err2),
            0
        );
        // Re-running does not duplicate the hook.
        let settings = read_settings(dir.path());
        assert_eq!(
            count_native_hook(&settings),
            1,
            "re-run must not duplicate the hook"
        );
        // Next-step text is byte-identical across runs.
        assert_eq!(out, out2, "next-step text must be deterministic");
    });
}

// --- init is manifest-agnostic: it never writes scryrs.json or .scryrs/ ---

#[test]
fn init_claude_code_does_not_create_manifest_or_scaffold() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(["init", "--agent", "claude-code"], &mut out, &mut err),
            0
        );
        assert!(err.is_empty(), "stderr: {}", String::from_utf8_lossy(&err));

        // The hook is installed, but no config side effects exist.
        assert!(dir.path().join(".claude/settings.json").exists());
        assert!(
            !dir.path().join("scryrs.json").exists(),
            "init must not create scryrs.json"
        );
        assert!(
            !dir.path().join(".scryrs").exists(),
            "init must not create the .scryrs scaffold"
        );
    });
}

#[test]
fn init_pi_does_not_create_manifest_or_scaffold() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(["init", "--agent", "pi"], &mut out, &mut err),
            0
        );
        assert!(dir.path().join(".pi/extensions/scryrs/index.ts").exists());
        assert!(!dir.path().join("scryrs.json").exists());
        assert!(!dir.path().join(".scryrs").exists());
    });
}

#[test]
fn init_unknown_harness_does_not_write() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(["init", "--agent", "unknown"], &mut out, &mut err),
            2
        );
        // An unsupported harness must leave the filesystem untouched.
        assert!(!dir.path().join(".scryrs").exists());
        assert!(!dir.path().join(".claude").exists());
        assert!(!dir.path().join(".pi").exists());
    });
}

// --- init --agent pi writes hook file ---

#[test]
fn init_agent_pi_writes_hook_file() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(["init", "--agent", "pi"], &mut out, &mut err),
            0
        );
        assert!(
            err.is_empty(),
            "stderr must be empty, got: {}",
            String::from_utf8_lossy(&err)
        );

        let hook_path = dir.path().join(".pi/extensions/scryrs/index.ts");
        assert!(
            hook_path.exists(),
            "hook file must exist at {}",
            hook_path.display()
        );
        let content =
            std::fs::read_to_string(&hook_path).unwrap_or_else(|e| panic!("read hook: {e}"));
        assert!(!content.is_empty(), "hook file must not be empty");
        assert!(
            content.contains("ExtensionAPI"),
            "hook must reference ExtensionAPI"
        );
    });
}

// --- init --agent unknown exits 2 ---

#[test]
fn init_agent_unknown_exits_2() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(["init", "--agent", "unknown"], &mut out, &mut err),
            2
        );
        assert!(out.is_empty(), "stdout must be empty");
        let err_str = String::from_utf8_lossy(&err);
        assert!(
            err_str.contains("'unknown' is not a supported harness"),
            "must report unsupported harness, got: {err_str}"
        );
        assert!(
            err_str.contains("Supported harnesses:"),
            "must list supported harnesses, got: {err_str}"
        );
        assert!(
            err_str.contains("claude-code"),
            "must mention claude-code, got: {err_str}"
        );
        assert!(err_str.contains("pi"), "must mention pi, got: {err_str}");
    });
}

// --- existing settings.json is merged, not clobbered ---

#[test]
fn init_claude_code_merges_existing_settings() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    // Pre-existing settings.json with an unrelated key and an unrelated hook.
    std::fs::create_dir_all(dir.path().join(".claude"))
        .unwrap_or_else(|e| panic!("create_dir: {e}"));
    std::fs::write(
        dir.path().join(".claude/settings.json"),
        r#"{
          "model": "claude-opus",
          "hooks": {
            "PreToolUse": [
              { "matcher": "Bash", "hooks": [ { "type": "command", "command": "other-tool" } ] }
            ]
          }
        }"#,
    )
    .unwrap_or_else(|e| panic!("write: {e}"));

    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(["init", "--agent", "claude-code"], &mut out, &mut err),
            0,
            "merge must succeed (not refuse), stderr: {}",
            String::from_utf8_lossy(&err)
        );
    });

    let settings = read_settings(dir.path());
    // Unrelated key preserved.
    assert_eq!(settings["model"], "claude-opus");
    // Unrelated hook preserved.
    let pre = settings["hooks"]["PreToolUse"]
        .as_array()
        .unwrap_or_else(|| panic!("PreToolUse must be array"));
    assert!(
        pre.iter().any(|e| e["hooks"][0]["command"] == "other-tool"),
        "existing unrelated hook must be preserved"
    );
    // Native hook added exactly once.
    assert_eq!(count_native_hook(&settings), 1);
}

// --- pi/index.ts collision ---

#[test]
fn init_pi_hook_file_collision_exits_2() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    // Create the target directory and file before running init
    std::fs::create_dir_all(dir.path().join(".pi/extensions/scryrs"))
        .unwrap_or_else(|e| panic!("create_dir: {e}"));
    std::fs::write(
        dir.path().join(".pi/extensions/scryrs/index.ts"),
        "existing",
    )
    .unwrap_or_else(|e| panic!("write: {e}"));

    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(["init", "--agent", "pi"], &mut out, &mut err),
            2
        );
        let err_str = String::from_utf8_lossy(&err);
        assert!(
            err_str.contains("already exists"),
            "must report file collision, got: {err_str}"
        );
        assert!(
            err_str.contains("Remove the file manually"),
            "must include remediation, got: {err_str}"
        );
    });
}

// --- self-install detection (Claude Code refused) ---

#[test]
fn init_self_install_detection_refuses_claude_code() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    // Create a fake scryrs source checkout: Cargo.toml with scryrs-cli + hooks/claude-code/
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/scryrs-cli\", \"crates/scryrs-types\"]\n",
    )
    .unwrap_or_else(|e| panic!("write Cargo.toml: {e}"));
    std::fs::create_dir_all(dir.path().join("hooks/claude-code"))
        .unwrap_or_else(|e| panic!("create_dir: {e}"));

    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(["init", "--agent", "claude-code"], &mut out, &mut err),
            2
        );
        let err_str = String::from_utf8_lossy(&err);
        assert!(
            err_str.contains("refusing to install"),
            "must refuse self-install, got: {err_str}"
        );
        assert!(
            err_str.contains("source repo"),
            "must mention source repo, got: {err_str}"
        );
    });
}

// --- self-install detection (Pi allowed at source root) ---

#[test]
fn init_self_install_pi_allowed_at_source_root() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    // Create a fake scryrs source checkout.
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/scryrs-cli\"]\n",
    )
    .unwrap_or_else(|e| panic!("write Cargo.toml: {e}"));
    std::fs::create_dir_all(dir.path().join("hooks/claude-code"))
        .unwrap_or_else(|e| panic!("create_dir: {e}"));

    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(["init", "--agent", "pi"], &mut out, &mut err),
            0
        );
        assert!(
            err.is_empty(),
            "stderr must be empty for allowed Pi install, got: {}",
            String::from_utf8_lossy(&err)
        );

        // File must be written at source root.
        let hook_path = dir.path().join(".pi/extensions/scryrs/index.ts");
        assert!(
            hook_path.exists(),
            "hook file must exist at {}",
            hook_path.display()
        );
        let content =
            std::fs::read_to_string(&hook_path).unwrap_or_else(|e| panic!("read hook: {e}"));
        assert!(!content.is_empty(), "hook file must not be empty");
        assert!(
            content.contains("ExtensionAPI"),
            "hook must reference ExtensionAPI"
        );
    });
}

// --- Pi self-install from subdirectory resolves to source root ---

#[test]
fn init_self_install_pi_from_subdirectory_resolves_to_root() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    // Create a fake scryrs source checkout.
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/scryrs-cli\"]\n",
    )
    .unwrap_or_else(|e| panic!("write Cargo.toml: {e}"));
    std::fs::create_dir_all(dir.path().join("hooks/claude-code"))
        .unwrap_or_else(|e| panic!("create_dir: {e}"));

    // Create a deeply nested subdirectory.
    let subdir = dir.path().join("crates/scryrs-cli/src");
    std::fs::create_dir_all(&subdir).unwrap_or_else(|e| panic!("create subdir: {e}"));

    with_cwd(&subdir, || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(["init", "--agent", "pi"], &mut out, &mut err),
            0
        );
        assert!(
            err.is_empty(),
            "stderr must be empty, got: {}",
            String::from_utf8_lossy(&err)
        );

        // File must be at checkout root, not at CWD.
        let root_hook_path = dir.path().join(".pi/extensions/scryrs/index.ts");
        assert!(
            root_hook_path.exists(),
            "hook file must exist at checkout root {}",
            root_hook_path.display()
        );

        // Verify NO nested .pi/ tree exists under the subdirectory CWD.
        let nested_pi = subdir.join(".pi");
        assert!(
            !nested_pi.exists(),
            "no .pi/ tree must exist under subdirectory CWD {}",
            nested_pi.display()
        );
    });
}

// --- Pi collision inside source checkout still exits 2 ---

#[test]
fn init_self_install_pi_collision_in_source_repo_exits_2() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    // Create a fake scryrs source checkout.
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/scryrs-cli\"]\n",
    )
    .unwrap_or_else(|e| panic!("write Cargo.toml: {e}"));
    std::fs::create_dir_all(dir.path().join("hooks/claude-code"))
        .unwrap_or_else(|e| panic!("create_dir: {e}"));

    // Pre-create the target file to trigger collision.
    std::fs::create_dir_all(dir.path().join(".pi/extensions/scryrs"))
        .unwrap_or_else(|e| panic!("create_dir: {e}"));
    std::fs::write(
        dir.path().join(".pi/extensions/scryrs/index.ts"),
        "existing",
    )
    .unwrap_or_else(|e| panic!("write: {e}"));

    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(["init", "--agent", "pi"], &mut out, &mut err),
            2
        );
        let err_str = String::from_utf8_lossy(&err);
        assert!(
            err_str.contains("already exists"),
            "must report file collision, got: {err_str}"
        );
        assert!(
            err_str.contains("Remove the file manually"),
            "must include remediation, got: {err_str}"
        );
    });
}

// --- unrelated project passes self-install check ---

#[test]
fn init_unrelated_project_passes_self_install_check() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    // Create a non-scryrs project: Cargo.toml without scryrs-cli, no hooks/
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"my-project\"\nversion = \"0.1.0\"\n",
    )
    .unwrap_or_else(|e| panic!("write Cargo.toml: {e}"));

    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(["init", "--agent", "claude-code"], &mut out, &mut err),
            0
        );
        assert!(
            err.is_empty(),
            "stderr must be empty, got: {}",
            String::from_utf8_lossy(&err)
        );
    });
}

// --- init without --agent exits 2 ---

#[test]
fn init_without_agent_exits_2() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(run_with_writers(["init"], &mut out, &mut err), 2);
        assert!(out.is_empty(), "stdout must be empty");
        let err_str = String::from_utf8_lossy(&err);
        assert!(
            err_str.contains("scryrs init:"),
            "must name init, got: {err_str}"
        );
        assert!(
            err_str.contains("--agent"),
            "must mention --agent, got: {err_str}"
        );
        assert!(
            err_str.contains("See `scryrs --help`"),
            "must escalate to --help, got: {err_str}"
        );
    });
}

// --- init with empty --agent value exits 2 ---

#[test]
fn init_empty_agent_exits_2() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(["init", "--agent", ""], &mut out, &mut err),
            2
        );
        assert!(out.is_empty());
        let err_str = String::from_utf8_lossy(&err);
        assert!(
            err_str.contains("--agent requires a non-empty value"),
            "must reject empty --agent, got: {err_str}"
        );
        assert!(
            err_str.contains("See `scryrs --help`"),
            "must escalate to --help, got: {err_str}"
        );
    });
}

// --- init never prompts (it collects no config) ---

#[test]
fn init_never_prompts_and_installs_hook_without_config() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        // No live config present anywhere; init must still succeed (hook only)
        // and never prompt or fail-fast on missing config.
        assert_eq!(
            run_with_writers(["init", "--agent", "claude-code"], &mut out, &mut err),
            0,
            "init must succeed without any config; stderr: {}",
            String::from_utf8_lossy(&err)
        );
        assert!(err.is_empty(), "stderr: {}", String::from_utf8_lossy(&err));

        let stdout = String::from_utf8_lossy(&out);
        // No wizard / prompting output.
        assert!(!stdout.contains("wizard"), "init must not run a wizard");
        // No live-config guidance — that belongs to setup.
        assert!(
            !stdout.contains("ingest"),
            "init must not mention ingest config"
        );

        assert!(dir.path().join(".claude/settings.json").exists());
        assert!(!dir.path().join("scryrs.json").exists());
        assert!(!dir.path().join(".scryrs").exists());
    });
}

// --- next-step text points to setup, not live config or scryrs up ---

#[test]
fn init_claude_code_stdout_has_hook_focused_next_steps() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(["init", "--agent", "claude-code"], &mut out, &mut err),
            0
        );
        assert!(err.is_empty());
        let stdout = String::from_utf8_lossy(&out);
        assert!(stdout.contains("Next steps:"));
        assert!(stdout.contains("scryrs is on your PATH"));
        assert!(stdout.contains("settings.json"));
        assert!(stdout.contains("Restart your Claude Code session"));
        // Directs to setup; does not include a remote URL or `scryrs up`.
        assert!(stdout.contains("scryrs setup"));
        assert!(!stdout.contains("scryrs up"));
        assert!(!stdout.contains("ingest_url"));
    });
}

#[test]
fn init_pi_stdout_has_hook_focused_next_steps() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(["init", "--agent", "pi"], &mut out, &mut err),
            0
        );
        assert!(err.is_empty());
        let stdout = String::from_utf8_lossy(&out);
        assert!(stdout.contains("Next steps:"));
        assert!(stdout.contains("scryrs is on your PATH"));
        assert!(stdout.contains("Reload Pi"));
        assert!(stdout.contains("scryrs setup"));
        assert!(!stdout.contains("scryrs up"));
    });
}

// --- init re-run is a no-op on identical hook content ---

#[test]
fn init_pi_identical_runtime_copy_is_noop() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(["init", "--agent", "pi"], &mut out, &mut err),
            0
        );
        assert!(err.is_empty(), "stderr: {}", String::from_utf8_lossy(&err));

        let hook_path = dir.path().join(".pi/extensions/scryrs/index.ts");
        let before = snapshot_dir_or_file(&hook_path);

        out.clear();
        err.clear();
        assert_eq!(
            run_with_writers(["init", "--agent", "pi"], &mut out, &mut err),
            0
        );
        assert!(err.is_empty(), "stderr: {}", String::from_utf8_lossy(&err));
        assert_eq!(before, snapshot_dir_or_file(&hook_path));
    });
}

// --- init help surfaces (hook only, point to setup) ---

#[test]
fn init_appears_in_help_output() {
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(run_with_writers(["--help"], &mut out, &mut err), 0);
    assert!(err.is_empty());
    let help = String::from_utf8_lossy(&out);
    assert!(help.contains("scryrs init --agent <NAME>"));
    assert!(help.contains("Install the scryrs trace hook"));
    assert!(help.contains("claude-code"));
    assert!(help.contains("pi"));
    assert!(help.contains("scryrs init --agent claude-code"));
    assert!(help.contains("scryrs init --agent pi"));
    // init no longer documents a --mode option.
    assert!(
        !help.contains("scryrs init --agent <NAME> [--mode"),
        "init must not document --mode"
    );
    // Points to setup for transport configuration.
    assert!(help.contains("scryrs setup"));
}

#[test]
fn init_appears_in_help_json() {
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(run_with_writers(["--help-json"], &mut out, &mut err), 0);
    assert!(err.is_empty());
    let json_str = String::from_utf8_lossy(&out);
    let doc: serde_json::Value =
        serde_json::from_str(&json_str).unwrap_or_else(|e| panic!("parse help-json: {e}"));

    assert_eq!(doc["surfaceVersion"], "0.14.0");

    let commands = doc["commands"]
        .as_array()
        .unwrap_or_else(|| panic!("commands must be array"));
    let init_cmd = commands
        .iter()
        .find(|c| c["name"] == "init")
        .unwrap_or_else(|| panic!("init must be in commands array"));

    let args = init_cmd["arguments"]
        .as_array()
        .unwrap_or_else(|| panic!("arguments must be array"));
    let agent_arg = args
        .iter()
        .find(|a| a["name"] == "agent")
        .unwrap_or_else(|| panic!("--agent must be in arguments"));
    assert_eq!(agent_arg["required"], true);
    assert_eq!(agent_arg["type"], "string");

    // init must not document --mode or any live-mode remote-configuration args.
    for forbidden in [
        "mode",
        "ingest-url",
        "workspace-id",
        "docker-network",
        "no-interactive",
    ] {
        assert!(
            !args.iter().any(|a| a["name"] == forbidden),
            "init must not include argument {forbidden}"
        );
    }
}

// --- regressions: other surfaces unaffected ---

#[test]
fn init_does_not_regress_help() {
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(run_with_writers(["--help"], &mut out, &mut err), 0);
    assert!(err.is_empty());
    let help = String::from_utf8_lossy(&out);
    assert!(help.contains("hotspots"));
    assert!(help.contains("record"));
    assert!(help.contains("OPTIONS"));
}

#[test]
fn init_does_not_regress_hotspots() {
    // Hotspot command requires a valid store. Without one, exits 2.
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(
        run_with_writers(["hotspots", "/tmp"], &mut out, &mut err),
        2
    );
    assert!(out.is_empty());
    let stderr = String::from_utf8_lossy(&err);
    assert!(stderr.contains("datastore not found"));
}

#[test]
fn init_does_not_regress_version() {
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(run_with_writers(["--version"], &mut out, &mut err), 0);
    assert!(err.is_empty());
    assert!(String::from_utf8_lossy(&out).contains("scryrs "));
}

#[test]
fn init_does_not_regress_unknown_command() {
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(run_with_writers(["nonexistent"], &mut out, &mut err), 2);
    assert!(String::from_utf8_lossy(&err).contains("unknown command"));
}
