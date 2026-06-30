use crate::run_with_writers;
use crate::test_support::{snapshot_dir_or_file, with_cwd};

// --- 7.1: init --agent claude-code creates settings.json with native hook ---

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

#[test]
fn init_agent_claude_code_creates_native_settings() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(
                ["init", "--agent", "claude-code", "--mode", "local"],
                &mut out,
                &mut err
            ),
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
            run_with_writers(
                ["init", "--agent", "claude-code", "--mode", "local"],
                &mut out,
                &mut err
            ),
            0
        );
        let mut out2 = Vec::new();
        let mut err2 = Vec::new();
        assert_eq!(
            run_with_writers(
                ["init", "--agent", "claude-code", "--mode", "local"],
                &mut out2,
                &mut err2
            ),
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

// --- 7.1b: init scaffolds the .scryrs/ runtime store ---

#[test]
fn init_claude_code_scaffolds_scryrs_store() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(
                ["init", "--agent", "claude-code", "--mode", "local"],
                &mut out,
                &mut err
            ),
            0
        );
        assert!(
            err.is_empty(),
            "stderr must be empty, got: {}",
            String::from_utf8_lossy(&err)
        );

        // The runtime store and its .gitignore exist.
        let db = dir.path().join(".scryrs/scryrs.db");
        let gitignore = dir.path().join(".scryrs/.gitignore");
        assert!(
            db.exists(),
            "scryrs.db must be scaffolded at {}",
            db.display()
        );
        assert!(
            gitignore.exists(),
            ".scryrs/.gitignore must be scaffolded at {}",
            gitignore.display()
        );

        // The .gitignore ignores trace data but keeps itself tracked.
        let ignore =
            std::fs::read_to_string(&gitignore).unwrap_or_else(|e| panic!("read gitignore: {e}"));
        assert!(ignore.contains("!.gitignore"), "got: {ignore}");

        // The db is a schema-initialized scryrs store (no rows yet).
        let conn = rusqlite::Connection::open(&db).unwrap_or_else(|e| panic!("open db: {e}"));
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM trace_events", [], |r| r.get(0))
            .unwrap_or_else(|e| panic!("query trace_events: {e}"));
        assert_eq!(count, 0, "fresh store must have no events");

        // stdout announces the scaffolded store.
        let stdout = String::from_utf8_lossy(&out);
        assert!(
            stdout.contains(".scryrs/"),
            "must mention .scryrs/, got: {stdout}"
        );
    });
}

#[test]
fn init_pi_scaffolds_scryrs_store() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(
                ["init", "--agent", "pi", "--mode", "local"],
                &mut out,
                &mut err
            ),
            0
        );
        assert!(dir.path().join(".scryrs/scryrs.db").exists());
        assert!(dir.path().join(".scryrs/.gitignore").exists());
    });
}

#[test]
fn init_unknown_harness_does_not_scaffold() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(
                ["init", "--agent", "unknown", "--mode", "local"],
                &mut out,
                &mut err
            ),
            2
        );
        // An unsupported harness must leave the filesystem untouched.
        assert!(
            !dir.path().join(".scryrs").exists(),
            ".scryrs must not be created for an unsupported harness"
        );
    });
}

// --- 7.2: init --agent pi writes hook file ---

#[test]
fn init_agent_pi_writes_hook_file() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(
                ["init", "--agent", "pi", "--mode", "local"],
                &mut out,
                &mut err
            ),
            0
        );
        assert!(
            err.is_empty(),
            "stderr must be empty, got: {}",
            String::from_utf8_lossy(&err)
        );

        let hook_path = dir.path().join(".pi/extensions/pi-trace/index.ts");
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

// --- 7.3: init --agent unknown exits 2 ---

#[test]
fn init_agent_unknown_exits_2() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(
                ["init", "--agent", "unknown", "--mode", "local"],
                &mut out,
                &mut err
            ),
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

// --- 7.4: existing settings.json is merged, not clobbered ---

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
            run_with_writers(
                ["init", "--agent", "claude-code", "--mode", "local"],
                &mut out,
                &mut err
            ),
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

// --- 7.6: pi/index.ts collision ---

#[test]
fn init_pi_hook_file_collision_exits_2() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    // Create the target directory and file before running init
    std::fs::create_dir_all(dir.path().join(".pi/extensions/pi-trace"))
        .unwrap_or_else(|e| panic!("create_dir: {e}"));
    std::fs::write(
        dir.path().join(".pi/extensions/pi-trace/index.ts"),
        "existing",
    )
    .unwrap_or_else(|e| panic!("write: {e}"));

    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(
                ["init", "--agent", "pi", "--mode", "local"],
                &mut out,
                &mut err
            ),
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

// --- 7.7: self-install detection (Claude Code refused) ---

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
            run_with_writers(
                ["init", "--agent", "claude-code", "--mode", "local"],
                &mut out,
                &mut err
            ),
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

// --- 7.7b: self-install detection (Pi allowed at source root) ---

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
            run_with_writers(
                ["init", "--agent", "pi", "--mode", "local"],
                &mut out,
                &mut err
            ),
            0
        );
        assert!(
            err.is_empty(),
            "stderr must be empty for allowed Pi install, got: {}",
            String::from_utf8_lossy(&err)
        );

        // File must be written at source root.
        let hook_path = dir.path().join(".pi/extensions/pi-trace/index.ts");
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

// --- 7.7c: Pi self-install from subdirectory resolves to source root ---

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
            run_with_writers(
                ["init", "--agent", "pi", "--mode", "local"],
                &mut out,
                &mut err
            ),
            0
        );
        assert!(
            err.is_empty(),
            "stderr must be empty, got: {}",
            String::from_utf8_lossy(&err)
        );

        // File must be at checkout root, not at CWD.
        let root_hook_path = dir.path().join(".pi/extensions/pi-trace/index.ts");
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

// --- 7.7d: Pi collision inside source checkout still exits 2 ---

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
    std::fs::create_dir_all(dir.path().join(".pi/extensions/pi-trace"))
        .unwrap_or_else(|e| panic!("create_dir: {e}"));
    std::fs::write(
        dir.path().join(".pi/extensions/pi-trace/index.ts"),
        "existing",
    )
    .unwrap_or_else(|e| panic!("write: {e}"));

    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(
                ["init", "--agent", "pi", "--mode", "local"],
                &mut out,
                &mut err
            ),
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

// --- 7.8: unrelated project passes self-install check ---

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

        // Should succeed — this is a normal project (local mode needs no remote config)
        assert_eq!(
            run_with_writers(
                ["init", "--agent", "claude-code", "--mode", "local"],
                &mut out,
                &mut err
            ),
            0
        );
        assert!(
            err.is_empty(),
            "stderr must be empty, got: {}",
            String::from_utf8_lossy(&err)
        );
    });
}

// --- 7.9: init without --agent exits 2 ---

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

// --- 7.10: init with empty --agent value exits 2 ---

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

// --- 7.11: init help text appears in --help output ---

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
    assert!(help.contains("--docker-network <NAME>"));
    assert!(help.contains("scryrs up"));
}

// --- 7.12: init entry appears in --help-json output ---

#[test]
fn init_appears_in_help_json() {
    let mut out = Vec::new();
    let mut err = Vec::new();

    assert_eq!(run_with_writers(["--help-json"], &mut out, &mut err), 0);
    assert!(err.is_empty());
    let json_str = String::from_utf8_lossy(&out);
    let doc: serde_json::Value =
        serde_json::from_str(&json_str).unwrap_or_else(|e| panic!("parse help-json: {e}"));

    assert_eq!(doc["surfaceVersion"], "0.12.1");

    let commands = doc["commands"]
        .as_array()
        .unwrap_or_else(|| panic!("commands must be array"));
    let init_cmd = commands
        .iter()
        .find(|c| c["name"] == "init")
        .unwrap_or_else(|| panic!("init must be in commands array"));

    assert_eq!(
        init_cmd["description"],
        "Install scryrs trace hook for a supported agent harness and scaffold workspace-local live bootstrap in default live mode"
    );

    let args = init_cmd["arguments"]
        .as_array()
        .unwrap_or_else(|| panic!("arguments must be array"));
    let agent_arg = args
        .iter()
        .find(|a| a["name"] == "agent")
        .unwrap_or_else(|| panic!("--agent must be in arguments"));
    assert_eq!(agent_arg["required"], true);
    assert_eq!(agent_arg["type"], "string");

    let docker_network_arg = args
        .iter()
        .find(|a| a["name"] == "docker-network")
        .unwrap_or_else(|| panic!("--docker-network must be in arguments"));
    assert_eq!(docker_network_arg["flag"], "--docker-network");

    let up_cmd = commands
        .iter()
        .find(|c| c["name"] == "up")
        .unwrap_or_else(|| panic!("up must be in commands array"));
    assert_eq!(
        up_cmd["description"],
        "Start the workspace-managed live-server Compose stack from .scryrs/compose.yml, resolving the external network from scryrs.json remote.docker_network (override via SCRYRS_DOCKER_NETWORK or .scryrs/.env)"
    );
}

// --- 7.14: claude-code stdout contains next-step text ---

#[test]
fn init_claude_code_stdout_has_next_steps() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(
                ["init", "--agent", "claude-code", "--mode", "local"],
                &mut out,
                &mut err
            ),
            0
        );
        assert!(err.is_empty());
        let stdout = String::from_utf8_lossy(&out);
        assert!(stdout.contains("Next steps:"));
        assert!(stdout.contains("scryrs is on your PATH"));
        assert!(stdout.contains("settings.json"));
        assert!(stdout.contains("Restart your Claude Code session"));
    });
}

// --- 7.15: pi stdout contains next-step text ---

#[test]
fn init_pi_stdout_has_next_steps() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(
                ["init", "--agent", "pi", "--mode", "local"],
                &mut out,
                &mut err
            ),
            0
        );
        assert!(err.is_empty());
        let stdout = String::from_utf8_lossy(&out);
        assert!(stdout.contains("Next steps:"));
        assert!(stdout.contains("scryrs is on your PATH"));
        assert!(stdout.contains("Reload Pi"));
    });
}

// --- 7.17: default mode is live; bare init without resolvable config fails fast ---

#[test]
fn init_default_mode_is_live_and_fails_fast_without_config() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        // Default (no --mode) resolves live. With no remote config in the temp
        // dir (no flags, no .scryrs/.env, no scryrs.json remote), the installer
        // must fail fast with exit 2 and deterministic guidance — never silently
        // fall back to local mode.
        assert_eq!(
            run_with_writers(["init", "--agent", "claude-code"], &mut out, &mut err),
            2
        );

        let err_str = String::from_utf8_lossy(&err);
        // Guidance from write_missing_config_guidance.
        assert!(
            err_str.contains("live mode is the default"),
            "must explain live is the default, got: {err_str}"
        );
        assert!(
            err_str.contains(".scryrs/.env"),
            "must describe populating .scryrs/.env, got: {err_str}"
        );
        assert!(
            err_str.contains("--mode local"),
            "must describe selecting local mode, got: {err_str}"
        );

        // No filesystem writes occurred — fail-fast happens before any scaffolding.
        assert!(
            !dir.path().join(".scryrs").exists(),
            ".scryrs must not be created on fail-fast"
        );
        assert!(!dir.path().join("scryrs.json").exists());
        assert!(!dir.path().join(".claude").exists());
    });
}

#[test]
fn init_explicit_local_mode_creates_db_and_not_manifest() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(
                ["init", "--agent", "pi", "--mode", "local"],
                &mut out,
                &mut err
            ),
            0
        );
        assert!(err.is_empty());

        assert!(dir.path().join(".scryrs/scryrs.db").exists());
        assert!(!dir.path().join("scryrs.json").exists());

        let stdout = String::from_utf8_lossy(&out);
        assert!(!stdout.contains("Docker"));
        assert!(!stdout.contains("live ingest"));
    });
}

// --- 7.18: live-mode validation before writes ---

#[test]
fn init_live_missing_ingest_url_exits_2_without_writes() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(
                [
                    "init",
                    "--agent",
                    "pi",
                    "--mode",
                    "live",
                    "--workspace-id",
                    "ws1",
                    "--agent-id",
                    "a1"
                ],
                &mut out,
                &mut err,
            ),
            2
        );
        let err_str = String::from_utf8_lossy(&err);
        // Missing ingest_url → no ingest URL resolves → Ok(None) → guidance for ingest_url.
        assert!(
            err_str.contains("ingest_url is not configured"),
            "must name missing ingest_url, got: {err_str}"
        );
        assert!(
            err_str.contains(".scryrs/.env"),
            "must describe populating .scryrs/.env, got: {err_str}"
        );
        assert!(
            err_str.contains("--mode local"),
            "must describe selecting local mode, got: {err_str}"
        );

        // No filesystem writes occurred.
        assert!(!dir.path().join(".scryrs").exists());
        assert!(!dir.path().join(".pi").exists());
        assert!(!dir.path().join("scryrs.json").exists());
    });
}

#[test]
fn init_live_missing_workspace_id_exits_2_without_writes() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(
                [
                    "init",
                    "--agent",
                    "pi",
                    "--mode",
                    "live",
                    "--ingest-url",
                    "http://localhost:8081",
                    "--repository-id",
                    "repo1",
                    "--agent-id",
                    "a1"
                ],
                &mut out,
                &mut err,
            ),
            2
        );
        let err_str = String::from_utf8_lossy(&err);
        // ingest_url + repository_id resolve; workspace_id is unresolved → guidance.
        assert!(
            err_str.contains("workspace_id is not configured"),
            "must name missing workspace_id, got: {err_str}"
        );
        assert!(
            err_str.contains(".scryrs/.env"),
            "must describe populating .scryrs/.env, got: {err_str}"
        );
        assert!(
            err_str.contains("--mode local"),
            "must describe selecting local mode, got: {err_str}"
        );

        // No filesystem writes occurred.
        assert!(!dir.path().join(".scryrs").exists());
        assert!(!dir.path().join("scryrs.json").exists());
    });
}

#[test]
fn init_live_missing_agent_id_succeeds_and_is_not_committed() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        // agent_id is no longer a required live-mode input; it is autogenerated
        // at runtime and never written to committed config.
        assert_eq!(
            run_with_writers(
                [
                    "init",
                    "--agent",
                    "pi",
                    "--mode",
                    "live",
                    "--ingest-url",
                    "http://localhost:8081",
                    "--repository-id",
                    "repo1",
                    "--workspace-id",
                    "ws1",
                    "--docker-network",
                    "agent-net"
                ],
                &mut out,
                &mut err,
            ),
            0,
            "stderr: {}",
            String::from_utf8_lossy(&err)
        );
        assert!(err.is_empty(), "stderr: {}", String::from_utf8_lossy(&err));

        let manifest = std::fs::read_to_string(dir.path().join("scryrs.json"))
            .unwrap_or_else(|e| panic!("read manifest: {e}"));
        let parsed: serde_json::Value =
            serde_json::from_str(&manifest).unwrap_or_else(|e| panic!("parse manifest: {e}"));
        // No agent_id is written into committed config.
        assert!(
            parsed["remote"]["agent_id"].is_null(),
            "agent_id must not be committed, got: {}",
            parsed["remote"]["agent_id"]
        );
    });
}

#[test]
fn init_live_missing_docker_network_exits_2_without_writes() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(
                [
                    "init",
                    "--agent",
                    "pi",
                    "--mode",
                    "live",
                    "--ingest-url",
                    "http://scryrs:8081",
                    "--repository-id",
                    "repo1",
                    "--workspace-id",
                    "ws1",
                    "--agent-id",
                    "a1"
                ],
                &mut out,
                &mut err,
            ),
            2
        );
        let err_str = String::from_utf8_lossy(&err);
        assert!(err_str.contains("docker_network is not configured"));
        assert!(err_str.contains("SCRYRS_DOCKER_NETWORK"));
        assert!(!dir.path().join(".scryrs").exists());
        assert!(!dir.path().join("scryrs.json").exists());
        assert!(!dir.path().join(".pi").exists());
    });
}

// --- 7.19: live-mode scryrs.json creation and .scryrs/ scaffolding ---

#[allow(clippy::disallowed_methods)]
#[test]
fn init_live_creates_scryrs_json_and_skips_scryrs_db() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        // Set up a Git repo so repository_id can be derived.
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(dir.path())
            .output()
            .unwrap_or_else(|e| panic!("git init: {e}"));
        std::process::Command::new("git")
            .args([
                "remote",
                "add",
                "origin",
                "https://github.com/example/repo.git",
            ])
            .current_dir(dir.path())
            .output()
            .unwrap_or_else(|e| panic!("git remote: {e}"));

        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(
                [
                    "init",
                    "--agent",
                    "claude-code",
                    "--mode",
                    "live",
                    "--ingest-url",
                    "http://localhost:8081",
                    "--workspace-id",
                    "ws1",
                    "--agent-id",
                    "a1",
                    "--docker-network",
                    "agent-net",
                ],
                &mut out,
                &mut err,
            ),
            0
        );
        assert!(err.is_empty(), "stderr: {}", String::from_utf8_lossy(&err));

        // scryrs.json is created.
        let manifest_path = dir.path().join("scryrs.json");
        assert!(manifest_path.exists(), "scryrs.json must be created");
        let manifest = std::fs::read_to_string(&manifest_path)
            .unwrap_or_else(|e| panic!("read manifest: {e}"));
        let parsed: serde_json::Value =
            serde_json::from_str(&manifest).unwrap_or_else(|e| panic!("parse manifest: {e}"));
        assert_eq!(parsed["remote"]["ingest_url"], "http://localhost:8081");
        assert_eq!(parsed["remote"]["workspace_id"], "ws1");
        assert_eq!(parsed["remote"]["docker_network"], "agent-net");
        // repository_id (Git-derived) and agent_id (autogenerated) are NOT committed.
        assert!(
            parsed["remote"]["repository_id"].is_null(),
            "repository_id must not be committed"
        );
        assert!(
            parsed["remote"]["agent_id"].is_null(),
            "agent_id must not be committed"
        );

        // .scryrs/ is created but .scryrs/scryrs.db is NOT.
        assert!(dir.path().join(".scryrs").exists());
        assert!(dir.path().join(".scryrs/.gitignore").exists());
        assert!(dir.path().join(".scryrs/hooks").exists());
        assert!(
            !dir.path().join(".scryrs/scryrs.db").exists(),
            "scryrs.db must NOT be created in live mode"
        );

        let compose_path = dir.path().join(".scryrs/compose.yml");
        assert!(
            compose_path.exists(),
            ".scryrs/compose.yml must be created in live mode"
        );
        let compose =
            std::fs::read_to_string(&compose_path).unwrap_or_else(|e| panic!("read compose: {e}"));
        assert!(compose.contains("image: ghcr.io/matthijsrademaker/scryrs-server:latest"));
        // The consumer scaffold pulls the published image; it must not require a
        // local source build via a `build:` context.
        assert!(!compose.contains("build:"));
        assert!(compose.contains("container_name: scryrs"));
        assert!(compose.contains("name: ${SCRYRS_DOCKER_NETWORK}"));
        assert!(compose.contains("- scryrs-data:/data/scryrs"));

        // Live mode scaffolds a gitignored overrides-only .scryrs/.env stub —
        // no managed identity or network values are pre-populated.
        let env_path = dir.path().join(".scryrs/.env");
        assert!(
            env_path.exists(),
            ".scryrs/.env must be created in live mode"
        );
        let env_contents =
            std::fs::read_to_string(&env_path).unwrap_or_else(|e| panic!("read .env: {e}"));
        assert!(
            !env_contents.contains("SCRYRS_REMOTE_INGEST_URL=http://localhost:8081"),
            "managed ingest URL must not be written to .scryrs/.env, got: {env_contents}"
        );
        assert!(
            !env_contents.contains("SCRYRS_DOCKER_NETWORK=agent-net"),
            "managed docker network must not be written to .scryrs/.env, got: {env_contents}"
        );
        assert!(
            env_contents.contains("local overrides"),
            ".scryrs/.env must be an overrides-only stub, got: {env_contents}"
        );

        let stdout = String::from_utf8_lossy(&out);
        assert!(stdout.contains("live-mode remote ingest"));
        assert!(stdout.contains("Remote server endpoint: http://localhost:8081"));
        assert!(stdout.contains("External Docker network: agent-net"));
        assert!(stdout.contains("scryrs up"));
        assert!(!stdout.contains("docker-compose.yml"));
    });
}

/// Bare `scryrs init` (live default, no flags) succeeds when complete remote
/// configuration resolves from `.scryrs/.env`. Exercises the default-is-live
/// path plus the `.scryrs/.env` precedence layer without mutating process env.
#[test]
fn init_default_live_resolves_from_scryrs_env() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    // Seed a complete .scryrs/.env so live resolution has every required field.
    let scryrs_dir = dir.path().join(".scryrs");
    std::fs::create_dir_all(&scryrs_dir).unwrap_or_else(|e| panic!("create .scryrs: {e}"));
    std::fs::write(
        scryrs_dir.join(".env"),
        "SCRYRS_REMOTE_INGEST_URL=http://localhost:8081\n\
         SCRYRS_REPOSITORY_ID=repo-from-env\n\
         SCRYRS_WORKSPACE_ID=ws-from-env\n\
         SCRYRS_AGENT_ID=agent-from-env\n\
         SCRYRS_DOCKER_NETWORK=agent-net\n",
    )
    .unwrap_or_else(|e| panic!("write .env: {e}"));

    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        // No --mode (live default), no flags — config comes entirely from .env.
        assert_eq!(
            run_with_writers(["init", "--agent", "claude-code"], &mut out, &mut err),
            0,
            "stderr: {}",
            String::from_utf8_lossy(&err)
        );
        assert!(err.is_empty(), "stderr: {}", String::from_utf8_lossy(&err));

        // Live manifest written from resolved .env values.
        let manifest = std::fs::read_to_string(dir.path().join("scryrs.json"))
            .unwrap_or_else(|e| panic!("read manifest: {e}"));
        let parsed: serde_json::Value =
            serde_json::from_str(&manifest).unwrap_or_else(|e| panic!("parse manifest: {e}"));
        assert_eq!(parsed["remote"]["ingest_url"], "http://localhost:8081");
        assert_eq!(parsed["remote"]["workspace_id"], "ws-from-env");
        assert_eq!(parsed["remote"]["docker_network"], "agent-net");
        // repository_id/agent_id resolve from .env at runtime but are never committed.
        assert!(parsed["remote"]["repository_id"].is_null());
        assert!(parsed["remote"]["agent_id"].is_null());

        // Live mode: no local db.
        assert!(!dir.path().join(".scryrs/scryrs.db").exists());
    });
}

#[test]
fn init_live_with_explicit_repository_id_is_not_committed() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        // No git repo — repository_id is explicit, satisfying resolution without
        // a Git derivation, but it is still not written to committed config.
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(
                [
                    "init",
                    "--agent",
                    "pi",
                    "--mode",
                    "live",
                    "--ingest-url",
                    "http://localhost:8081",
                    "--workspace-id",
                    "ws1",
                    "--agent-id",
                    "a1",
                    "--repository-id",
                    "repo-explicit",
                    "--docker-network",
                    "agent-net",
                ],
                &mut out,
                &mut err,
            ),
            0
        );
        assert!(err.is_empty());

        let manifest_path = dir.path().join("scryrs.json");
        let manifest = std::fs::read_to_string(&manifest_path)
            .unwrap_or_else(|e| panic!("read manifest: {e}"));
        let parsed: serde_json::Value =
            serde_json::from_str(&manifest).unwrap_or_else(|e| panic!("parse manifest: {e}"));
        assert!(
            parsed["remote"]["repository_id"].is_null(),
            "explicit repository_id must not be committed to scryrs.json"
        );
        assert_eq!(parsed["remote"]["docker_network"], "agent-net");
    });
}

#[test]
fn init_live_merges_existing_manifest() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    // Pre-existing scryrs.json with unrelated keys.
    std::fs::write(
        dir.path().join("scryrs.json"),
        r#"{"project_name": "my-project", "version": "1.0"}"#,
    )
    .unwrap_or_else(|e| panic!("write: {e}"));

    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(
                [
                    "init",
                    "--agent",
                    "claude-code",
                    "--mode",
                    "live",
                    "--ingest-url",
                    "http://localhost:8081",
                    "--workspace-id",
                    "ws1",
                    "--agent-id",
                    "a1",
                    "--repository-id",
                    "repo1",
                    "--docker-network",
                    "agent-net",
                ],
                &mut out,
                &mut err,
            ),
            0
        );
        assert!(err.is_empty());

        let manifest = std::fs::read_to_string(dir.path().join("scryrs.json"))
            .unwrap_or_else(|e| panic!("read: {e}"));
        let parsed: serde_json::Value =
            serde_json::from_str(&manifest).unwrap_or_else(|e| panic!("parse: {e}"));

        // Unrelated keys preserved.
        assert_eq!(parsed["project_name"], "my-project");
        assert_eq!(parsed["version"], "1.0");
        // Remote section added with only the committed shared constants.
        assert_eq!(parsed["remote"]["ingest_url"], "http://localhost:8081");
        assert_eq!(parsed["remote"]["workspace_id"], "ws1");
        assert_eq!(parsed["remote"]["docker_network"], "agent-net");
        assert!(parsed["remote"]["agent_id"].is_null());
        assert!(parsed["remote"]["repository_id"].is_null());
    });
}

// --- 7.20: live mode refused in source checkout ---

#[test]
fn init_live_reuses_existing_workspace_bootstrap_for_second_harness() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(
                [
                    "init",
                    "--agent",
                    "claude-code",
                    "--mode",
                    "live",
                    "--ingest-url",
                    "http://scryrs:8081",
                    "--workspace-id",
                    "ws1",
                    "--agent-id",
                    "a1",
                    "--repository-id",
                    "repo1",
                    "--docker-network",
                    "agent-net",
                ],
                &mut out,
                &mut err,
            ),
            0
        );
        assert!(err.is_empty(), "stderr: {}", String::from_utf8_lossy(&err));

        let env_before = snapshot_dir_or_file(&dir.path().join(".scryrs/.env"));
        let compose_before = snapshot_dir_or_file(&dir.path().join(".scryrs/compose.yml"));
        let manifest_before = snapshot_dir_or_file(&dir.path().join("scryrs.json"));

        out.clear();
        err.clear();
        assert_eq!(
            run_with_writers(
                [
                    "init",
                    "--agent",
                    "pi",
                    "--mode",
                    "live",
                    "--ingest-url",
                    "http://scryrs:8081",
                    "--workspace-id",
                    "ws1",
                    "--agent-id",
                    "a1",
                    "--repository-id",
                    "repo1",
                    "--docker-network",
                    "agent-net",
                ],
                &mut out,
                &mut err,
            ),
            0
        );
        assert!(err.is_empty(), "stderr: {}", String::from_utf8_lossy(&err));

        assert_eq!(
            env_before,
            snapshot_dir_or_file(&dir.path().join(".scryrs/.env"))
        );
        assert_eq!(
            compose_before,
            snapshot_dir_or_file(&dir.path().join(".scryrs/compose.yml"))
        );
        assert_eq!(
            manifest_before,
            snapshot_dir_or_file(&dir.path().join("scryrs.json"))
        );
        assert!(dir.path().join(".pi/extensions/pi-trace/index.ts").exists());
    });
}

#[test]
fn init_live_conflicting_managed_bootstrap_values_fail_without_partial_writes() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(
                [
                    "init",
                    "--agent",
                    "claude-code",
                    "--mode",
                    "live",
                    "--ingest-url",
                    "http://scryrs:8081",
                    "--workspace-id",
                    "ws1",
                    "--agent-id",
                    "a1",
                    "--repository-id",
                    "repo1",
                    "--docker-network",
                    "agent-net",
                ],
                &mut out,
                &mut err,
            ),
            0
        );
        assert!(err.is_empty(), "stderr: {}", String::from_utf8_lossy(&err));

        let env_before = snapshot_dir_or_file(&dir.path().join(".scryrs/.env"));
        let compose_before = snapshot_dir_or_file(&dir.path().join(".scryrs/compose.yml"));
        let manifest_before = snapshot_dir_or_file(&dir.path().join("scryrs.json"));

        out.clear();
        err.clear();
        assert_eq!(
            run_with_writers(
                [
                    "init",
                    "--agent",
                    "pi",
                    "--mode",
                    "live",
                    "--ingest-url",
                    "http://scryrs:8081",
                    "--workspace-id",
                    "ws1",
                    "--agent-id",
                    "a1",
                    "--repository-id",
                    "repo1",
                    "--docker-network",
                    "other-net",
                ],
                &mut out,
                &mut err,
            ),
            2
        );
        let stderr = String::from_utf8_lossy(&err);
        assert!(stderr.contains("conflicts with managed live bootstrap"));
        assert_eq!(
            env_before,
            snapshot_dir_or_file(&dir.path().join(".scryrs/.env"))
        );
        assert_eq!(
            compose_before,
            snapshot_dir_or_file(&dir.path().join(".scryrs/compose.yml"))
        );
        assert_eq!(
            manifest_before,
            snapshot_dir_or_file(&dir.path().join("scryrs.json"))
        );
        assert!(!dir.path().join(".pi").exists());
    });
}

#[test]
fn init_pi_identical_runtime_copy_is_noop() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(
                ["init", "--agent", "pi", "--mode", "local"],
                &mut out,
                &mut err,
            ),
            0
        );
        assert!(err.is_empty(), "stderr: {}", String::from_utf8_lossy(&err));

        let hook_path = dir.path().join(".pi/extensions/pi-trace/index.ts");
        let before = snapshot_dir_or_file(&hook_path);

        out.clear();
        err.clear();
        assert_eq!(
            run_with_writers(
                ["init", "--agent", "pi", "--mode", "local"],
                &mut out,
                &mut err,
            ),
            0
        );
        assert!(err.is_empty(), "stderr: {}", String::from_utf8_lossy(&err));
        assert_eq!(before, snapshot_dir_or_file(&hook_path));
    });
}

#[test]
fn init_live_refused_in_source_checkout() {
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
            run_with_writers(
                [
                    "init",
                    "--agent",
                    "claude-code",
                    "--mode",
                    "live",
                    "--ingest-url",
                    "http://localhost:8081",
                    "--workspace-id",
                    "ws1",
                    "--agent-id",
                    "a1",
                ],
                &mut out,
                &mut err,
            ),
            2
        );
        let err_str = String::from_utf8_lossy(&err);
        assert!(
            err_str.contains("live mode is not allowed in the scryrs source repository"),
            "must refuse live mode in source checkout, got: {err_str}"
        );
        assert!(
            err_str.contains("configures a consumer project"),
            "must explain live mode purpose, got: {err_str}"
        );

        // No scryrs.json written.
        assert!(!dir.path().join("scryrs.json").exists());
    });
}

#[test]
fn init_pi_live_refused_in_source_checkout() {
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
            run_with_writers(
                [
                    "init",
                    "--agent",
                    "pi",
                    "--mode",
                    "live",
                    "--ingest-url",
                    "http://localhost:8081",
                    "--workspace-id",
                    "ws1",
                    "--agent-id",
                    "a1",
                ],
                &mut out,
                &mut err,
            ),
            2
        );
        let err_str = String::from_utf8_lossy(&err);
        assert!(
            err_str.contains("live mode is not allowed in the scryrs source repository"),
            "must refuse live mode in source checkout for pi agent, got: {err_str}"
        );
        // For pi, the refusal points to the local-mode dogfooding path.
        assert!(
            err_str.contains("scryrs init --agent pi --mode local"),
            "must suggest --mode local for pi dogfooding, got: {err_str}"
        );

        // No scryrs.json written.
        assert!(!dir.path().join("scryrs.json").exists());
        // No Pi hook written.
        assert!(!dir.path().join(".pi").exists());
    });
}

// --- 7.21: invalid mode exits 2 ---

#[test]
fn init_invalid_mode_exits_2() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(
                ["init", "--agent", "pi", "--mode", "invalid"],
                &mut out,
                &mut err
            ),
            2
        );
        let err_str = String::from_utf8_lossy(&err);
        assert!(err_str.contains("--mode must be local or live"));
    });
}

// --- 7.22: live-mode init writes .scryrs/hooks/ but not .scryrs/scryrs.db in Pi ---

#[test]
fn init_pi_live_creates_dirs_not_db() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(
                [
                    "init",
                    "--agent",
                    "pi",
                    "--mode",
                    "live",
                    "--ingest-url",
                    "http://localhost:8081",
                    "--workspace-id",
                    "ws1",
                    "--agent-id",
                    "a1",
                    "--repository-id",
                    "repo1",
                    "--docker-network",
                    "agent-net",
                ],
                &mut out,
                &mut err,
            ),
            0
        );

        assert!(dir.path().join(".scryrs").exists());
        assert!(dir.path().join(".scryrs/.gitignore").exists());
        assert!(dir.path().join(".scryrs/hooks").exists());
        assert!(!dir.path().join(".scryrs/scryrs.db").exists());
        assert!(dir.path().join(".pi/extensions/pi-trace/index.ts").exists());
    });
}

#[test]
fn init_does_not_regress_help() {
    // --help still works as before (init is additive)
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
