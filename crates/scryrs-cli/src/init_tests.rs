use crate::run_with_writers;
use crate::test_support::with_cwd;

// --- 7.1: init --agent claude-code writes hook file ---

#[test]
fn init_agent_claude_code_writes_hook_file() {
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

        let hook_path = dir.path().join(".claude/hooks/scryrs-hook.mjs");
        assert!(
            hook_path.exists(),
            "hook file must exist at {}",
            hook_path.display()
        );
        let content =
            std::fs::read_to_string(&hook_path).unwrap_or_else(|e| panic!("read hook: {e}"));
        assert!(!content.is_empty(), "hook file must not be empty");
        assert!(
            content.contains("PreToolUse"),
            "hook must contain PreToolUse"
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
            run_with_writers(["init", "--agent", "pi"], &mut out, &mut err),
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

// --- 7.4: settings.json collision ---

#[test]
fn init_claude_code_settings_json_collision_exits_2() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    // Create .claude/settings.json before running init
    std::fs::create_dir_all(dir.path().join(".claude"))
        .unwrap_or_else(|e| panic!("create_dir: {e}"));
    std::fs::write(dir.path().join(".claude/settings.json"), "{}")
        .unwrap_or_else(|e| panic!("write: {e}"));

    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(["init", "--agent", "claude-code"], &mut out, &mut err),
            2
        );
        let err_str = String::from_utf8_lossy(&err);
        assert!(
            err_str.contains("settings.json already exists"),
            "must report settings.json collision, got: {err_str}"
        );
        assert!(
            err_str.contains("not be installed"),
            "must not claim hook source was installed, got: {err_str}"
        );
        assert!(
            !err_str.contains("has been installed"),
            "must not claim hook source was already installed, got: {err_str}"
        );
        assert!(
            err_str.contains("PreToolUse"),
            "must include JSON block instructions, got: {err_str}"
        );

        // Verify no mutation: hook file must NOT exist
        let hook_path = dir.path().join(".claude/hooks/scryrs-hook.mjs");
        assert!(
            !hook_path.exists(),
            "hook file must not be written on settings.json collision"
        );
    });
}

// --- 7.5: scryrs-hook.mjs collision ---

#[test]
fn init_claude_code_hook_file_collision_exits_2() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    // Create the target directory and file before running init
    std::fs::create_dir_all(dir.path().join(".claude/hooks"))
        .unwrap_or_else(|e| panic!("create_dir: {e}"));
    std::fs::write(dir.path().join(".claude/hooks/scryrs-hook.mjs"), "existing")
        .unwrap_or_else(|e| panic!("write: {e}"));

    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(["init", "--agent", "claude-code"], &mut out, &mut err),
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

// --- 7.7: self-install detection ---

#[test]
fn init_self_install_detection_refuses() {
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

        // Should succeed — this is a normal project
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

    assert_eq!(doc["surfaceVersion"], "0.3.0");

    let commands = doc["commands"]
        .as_array()
        .unwrap_or_else(|| panic!("commands must be array"));
    let init_cmd = commands
        .iter()
        .find(|c| c["name"] == "init")
        .unwrap_or_else(|| panic!("init must be in commands array"));

    assert_eq!(
        init_cmd["description"],
        "Install scryrs trace hook for a supported agent harness"
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
}

// --- 7.14: claude-code stdout contains next-step text ---

#[test]
fn init_claude_code_stdout_has_next_steps() {
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
            run_with_writers(["init", "--agent", "pi"], &mut out, &mut err),
            0
        );
        assert!(err.is_empty());
        let stdout = String::from_utf8_lossy(&out);
        assert!(stdout.contains("Next steps:"));
        assert!(stdout.contains("scryrs is on your PATH"));
        assert!(stdout.contains("Reload Pi"));
    });
}

// --- 7.16: all existing tests remain unchanged ---

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
