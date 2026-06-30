//! Tests for `scryrs setup <mode>` — runtime trace-transport configuration.

use std::collections::VecDeque;

use crate::init_prompt::{InitPrompt, InitPromptError, PromptSpec, TerminalState};
use crate::run_with_writers;
use crate::setup::{self, SetupExecutionOptions, SetupMode};
use crate::test_support::{snapshot_dir_or_file, with_cwd};

// ---------------------------------------------------------------------------
// Fake prompt for wizard tests
// ---------------------------------------------------------------------------

enum FakeTextResponse {
    Value(&'static str),
    Cancelled,
}

enum FakeConfirmResponse {
    Value(bool),
    #[allow(dead_code)]
    Cancelled,
}

struct FakePrompt {
    text_responses: VecDeque<FakeTextResponse>,
    confirm_responses: VecDeque<FakeConfirmResponse>,
    seen_fields: Vec<&'static str>,
    seen_confirms: Vec<String>,
}

impl FakePrompt {
    fn new(
        text_responses: Vec<FakeTextResponse>,
        confirm_responses: Vec<FakeConfirmResponse>,
    ) -> Self {
        Self {
            text_responses: VecDeque::from(text_responses),
            confirm_responses: VecDeque::from(confirm_responses),
            seen_fields: Vec::new(),
            seen_confirms: Vec::new(),
        }
    }
}

impl InitPrompt for FakePrompt {
    fn prompt_text(&mut self, spec: &PromptSpec) -> Result<String, InitPromptError> {
        self.seen_fields.push(spec.field_name);
        match self.text_responses.pop_front() {
            Some(FakeTextResponse::Value(value)) => Ok(value.to_string()),
            Some(FakeTextResponse::Cancelled) => Err(InitPromptError::Cancelled),
            None => panic!("missing fake text response for {}", spec.field_name),
        }
    }

    fn confirm(&mut self, prompt: &str, _default: bool) -> Result<bool, InitPromptError> {
        self.seen_confirms.push(prompt.to_string());
        match self.confirm_responses.pop_front() {
            Some(FakeConfirmResponse::Value(value)) => Ok(value),
            Some(FakeConfirmResponse::Cancelled) => Err(InitPromptError::Cancelled),
            None => panic!("missing fake confirm response for {prompt}"),
        }
    }
}

fn tty_options(with_compose: bool) -> SetupExecutionOptions {
    SetupExecutionOptions {
        no_interactive: false,
        with_compose,
        terminal_state: TerminalState {
            stdin_is_terminal: true,
            stdout_is_terminal: true,
        },
    }
}

// ---------------------------------------------------------------------------
// mode argument handling
// ---------------------------------------------------------------------------

#[test]
fn setup_requires_a_mode() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(run_with_writers(["setup"], &mut out, &mut err), 2);
        let err_str = String::from_utf8_lossy(&err);
        assert!(
            err_str.contains("mode") && err_str.contains("required"),
            "must state a mode is required, got: {err_str}"
        );
        assert!(!dir.path().join(".scryrs").exists());
        assert!(!dir.path().join("scryrs.json").exists());
    });
}

#[test]
fn setup_unknown_mode_fails_loudly() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(run_with_writers(["setup", "remote"], &mut out, &mut err), 2);
        let err_str = String::from_utf8_lossy(&err);
        assert!(
            err_str.contains("'remote' is not a supported mode"),
            "must reject unknown mode, got: {err_str}"
        );
        // Supported modes listed in stable order: live, local.
        let live = err_str.find("live").unwrap_or(usize::MAX);
        let local = err_str.find("local").unwrap_or(0);
        assert!(
            live < local,
            "modes must be listed live then local: {err_str}"
        );
        assert!(!dir.path().join(".scryrs").exists());
        assert!(!dir.path().join("scryrs.json").exists());
    });
}

#[test]
fn setup_runs_without_a_prior_init() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        // No hook installed; setup local succeeds on its own.
        assert_eq!(run_with_writers(["setup", "local"], &mut out, &mut err), 0);
        assert!(err.is_empty(), "stderr: {}", String::from_utf8_lossy(&err));
        assert!(dir.path().join(".scryrs/scryrs.db").exists());
        // It installs no hook.
        assert!(!dir.path().join(".claude").exists());
        assert!(!dir.path().join(".pi").exists());
    });
}

// ---------------------------------------------------------------------------
// setup local
// ---------------------------------------------------------------------------

#[test]
fn setup_local_creates_store_without_manifest() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(run_with_writers(["setup", "local"], &mut out, &mut err), 0);
        assert!(err.is_empty(), "stderr: {}", String::from_utf8_lossy(&err));

        assert!(dir.path().join(".scryrs/scryrs.db").exists());
        assert!(dir.path().join(".scryrs/.gitignore").exists());
        assert!(
            !dir.path().join("scryrs.json").exists(),
            "local setup must not create scryrs.json"
        );

        let stdout = String::from_utf8_lossy(&out);
        assert!(!stdout.contains("Docker"));
        assert!(!stdout.contains("ingest"));
    });
}

#[test]
fn setup_local_is_idempotent() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();
        assert_eq!(run_with_writers(["setup", "local"], &mut out, &mut err), 0);

        let db = dir.path().join(".scryrs/scryrs.db");
        let before = snapshot_dir_or_file(&db);

        out.clear();
        err.clear();
        assert_eq!(run_with_writers(["setup", "local"], &mut out, &mut err), 0);
        assert_eq!(before, snapshot_dir_or_file(&db), "store must be preserved");
    });
}

// ---------------------------------------------------------------------------
// setup live (core)
// ---------------------------------------------------------------------------

fn read_manifest(dir: &std::path::Path) -> serde_json::Value {
    let manifest = std::fs::read_to_string(dir.join("scryrs.json"))
        .unwrap_or_else(|e| panic!("read manifest: {e}"));
    serde_json::from_str(&manifest).unwrap_or_else(|e| panic!("parse manifest: {e}"))
}

#[test]
fn setup_live_writes_only_committed_constants() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(
                [
                    "setup",
                    "live",
                    "--ingest-url",
                    "http://scryrs:8081",
                    "--workspace-id",
                    "myproj",
                ],
                &mut out,
                &mut err,
            ),
            0,
            "stderr: {}",
            String::from_utf8_lossy(&err)
        );
        assert!(err.is_empty(), "stderr: {}", String::from_utf8_lossy(&err));

        let parsed = read_manifest(dir.path());
        assert_eq!(parsed["remote"]["ingest_url"], "http://scryrs:8081");
        assert_eq!(parsed["remote"]["workspace_id"], "myproj");
        assert!(parsed["remote"]["repository_id"].is_null());
        assert!(parsed["remote"]["agent_id"].is_null());
        assert!(
            parsed["remote"]["docker_network"].is_null(),
            "core setup live must not write docker_network"
        );

        // No compose scaffold; no managed .env.
        assert!(!dir.path().join(".scryrs/compose.yml").exists());
        assert!(!dir.path().join(".scryrs/scryrs.db").exists());
    });
}

#[test]
fn setup_live_does_not_require_docker_network() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(
                [
                    "setup",
                    "live",
                    "--ingest-url",
                    "http://scryrs:8081",
                    "--workspace-id",
                    "myproj",
                ],
                &mut out,
                &mut err,
            ),
            0,
            "stderr: {}",
            String::from_utf8_lossy(&err)
        );
    });
}

#[test]
fn setup_live_missing_ingest_url_exits_2_without_writes() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        // Non-TTY in tests → fail-fast, no wizard.
        assert_eq!(
            run_with_writers(
                ["setup", "live", "--workspace-id", "ws1"],
                &mut out,
                &mut err,
            ),
            2
        );
        let err_str = String::from_utf8_lossy(&err);
        assert!(
            err_str.contains("ingest_url is required"),
            "must name missing ingest_url, got: {err_str}"
        );
        assert!(!dir.path().join(".scryrs").exists());
        assert!(!dir.path().join("scryrs.json").exists());
    });
}

#[test]
fn setup_live_missing_workspace_id_exits_2_without_writes() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(
                ["setup", "live", "--ingest-url", "http://scryrs:8081"],
                &mut out,
                &mut err,
            ),
            2
        );
        let err_str = String::from_utf8_lossy(&err);
        assert!(
            err_str.contains("workspace_id is required"),
            "must name missing workspace_id, got: {err_str}"
        );
        assert!(!dir.path().join(".scryrs").exists());
        assert!(!dir.path().join("scryrs.json").exists());
    });
}

#[test]
fn setup_live_merges_into_existing_manifest() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
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
                    "setup",
                    "live",
                    "--ingest-url",
                    "http://scryrs:8081",
                    "--workspace-id",
                    "ws1",
                ],
                &mut out,
                &mut err,
            ),
            0,
            "stderr: {}",
            String::from_utf8_lossy(&err)
        );

        let parsed = read_manifest(dir.path());
        // Unrelated keys preserved.
        assert_eq!(parsed["project_name"], "my-project");
        assert_eq!(parsed["version"], "1.0");
        // Only the remote section is updated.
        assert_eq!(parsed["remote"]["ingest_url"], "http://scryrs:8081");
        assert_eq!(parsed["remote"]["workspace_id"], "ws1");
    });
}

#[test]
fn setup_live_conflicting_manifest_value_fails_without_partial_writes() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    std::fs::write(
        dir.path().join("scryrs.json"),
        r#"{"remote": {"ingest_url": "http://other:8081", "workspace_id": "ws1"}}"#,
    )
    .unwrap_or_else(|e| panic!("write: {e}"));
    let manifest_before = snapshot_dir_or_file(&dir.path().join("scryrs.json"));

    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(
                [
                    "setup",
                    "live",
                    "--ingest-url",
                    "http://scryrs:8081",
                    "--workspace-id",
                    "ws1",
                ],
                &mut out,
                &mut err,
            ),
            2
        );
        let err_str = String::from_utf8_lossy(&err);
        assert!(
            err_str.contains("remote.ingest_url") && err_str.contains("conflicts"),
            "must report the conflicting field, got: {err_str}"
        );
        // No partial writes.
        assert_eq!(
            manifest_before,
            snapshot_dir_or_file(&dir.path().join("scryrs.json"))
        );
        assert!(!dir.path().join(".scryrs").exists());
    });
}

// ---------------------------------------------------------------------------
// compose opt-in
// ---------------------------------------------------------------------------

#[test]
fn setup_live_without_compose_does_not_scaffold_compose() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(
                [
                    "setup",
                    "live",
                    "--ingest-url",
                    "http://scryrs:8081",
                    "--workspace-id",
                    "ws1",
                ],
                &mut out,
                &mut err,
            ),
            0
        );
        assert!(!dir.path().join(".scryrs/compose.yml").exists());
        assert!(!dir.path().join(".scryrs/.env").exists());
    });
}

#[test]
fn setup_live_with_compose_scaffolds_stack_and_records_network() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(
                [
                    "setup",
                    "live",
                    "--with-compose",
                    "--ingest-url",
                    "http://scryrs:8081",
                    "--workspace-id",
                    "ws1",
                    "--docker-network",
                    "agent-net",
                ],
                &mut out,
                &mut err,
            ),
            0,
            "stderr: {}",
            String::from_utf8_lossy(&err)
        );

        let parsed = read_manifest(dir.path());
        assert_eq!(parsed["remote"]["ingest_url"], "http://scryrs:8081");
        assert_eq!(parsed["remote"]["workspace_id"], "ws1");
        assert_eq!(parsed["remote"]["docker_network"], "agent-net");

        let compose_path = dir.path().join(".scryrs/compose.yml");
        assert!(compose_path.exists());
        let compose =
            std::fs::read_to_string(&compose_path).unwrap_or_else(|e| panic!("read compose: {e}"));
        assert!(compose.contains("name: ${SCRYRS_DOCKER_NETWORK}"));
        assert!(!compose.contains("build:"));

        // Overrides-only .env: managed values are not pre-populated.
        let env_path = dir.path().join(".scryrs/.env");
        assert!(env_path.exists());
        let env_contents =
            std::fs::read_to_string(&env_path).unwrap_or_else(|e| panic!("read .env: {e}"));
        assert!(!env_contents.contains("SCRYRS_DOCKER_NETWORK=agent-net"));
        assert!(env_contents.contains("local overrides"));

        let stdout = String::from_utf8_lossy(&out);
        assert!(stdout.contains("scryrs up"));
    });
}

#[test]
fn setup_live_with_compose_without_network_fails_loudly() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(
                [
                    "setup",
                    "live",
                    "--with-compose",
                    "--ingest-url",
                    "http://scryrs:8081",
                    "--workspace-id",
                    "ws1",
                ],
                &mut out,
                &mut err,
            ),
            2
        );
        let err_str = String::from_utf8_lossy(&err);
        assert!(
            err_str.contains("docker_network") && err_str.contains("Docker network"),
            "must explain compose needs a docker network, got: {err_str}"
        );
        assert!(!dir.path().join(".scryrs/compose.yml").exists());
        assert!(!dir.path().join("scryrs.json").exists());
    });
}

// ---------------------------------------------------------------------------
// source-checkout refusal
// ---------------------------------------------------------------------------

fn make_source_checkout(dir: &std::path::Path) {
    std::fs::write(
        dir.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/scryrs-cli\"]\n",
    )
    .unwrap_or_else(|e| panic!("write Cargo.toml: {e}"));
    std::fs::create_dir_all(dir.join("hooks/claude-code"))
        .unwrap_or_else(|e| panic!("create_dir: {e}"));
}

#[test]
fn setup_live_refused_in_source_checkout() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    make_source_checkout(dir.path());

    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(
                [
                    "setup",
                    "live",
                    "--ingest-url",
                    "http://scryrs:8081",
                    "--workspace-id",
                    "ws1",
                ],
                &mut out,
                &mut err,
            ),
            2
        );
        let err_str = String::from_utf8_lossy(&err);
        assert!(
            err_str.contains("source repository") || err_str.contains("consumer project"),
            "must refuse live setup in source checkout, got: {err_str}"
        );
        assert!(!dir.path().join("scryrs.json").exists());
    });
}

#[test]
fn setup_local_allowed_in_source_checkout() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    make_source_checkout(dir.path());

    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(run_with_writers(["setup", "local"], &mut out, &mut err), 0);
        assert!(err.is_empty(), "stderr: {}", String::from_utf8_lossy(&err));
        assert!(dir.path().join(".scryrs/scryrs.db").exists());
    });
}

// ---------------------------------------------------------------------------
// wizard (migrated to setup live)
// ---------------------------------------------------------------------------

#[test]
fn setup_live_interactive_wizard_collects_missing_config() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut prompt = FakePrompt::new(
            vec![
                FakeTextResponse::Value("http://scryrs:8081"),
                FakeTextResponse::Value("ws-interactive"),
            ],
            vec![FakeConfirmResponse::Value(true)],
        );

        assert_eq!(
            setup::execute_setup_with_prompt(
                &mut out,
                &mut err,
                SetupMode::Live,
                "",
                "",
                None,
                "",
                "",
                tty_options(false),
                &mut prompt,
            ),
            0,
            "stderr: {}",
            String::from_utf8_lossy(&err)
        );
        assert!(err.is_empty(), "stderr: {}", String::from_utf8_lossy(&err));
        // Without compose, the wizard prompts only ingest_url + workspace_id.
        assert_eq!(prompt.seen_fields, vec!["ingest_url", "workspace_id"]);
        assert_eq!(prompt.seen_confirms.len(), 1);

        let parsed = read_manifest(dir.path());
        assert_eq!(parsed["remote"]["ingest_url"], "http://scryrs:8081");
        assert_eq!(parsed["remote"]["workspace_id"], "ws-interactive");

        let stdout = String::from_utf8_lossy(&out);
        assert!(stdout.contains("scryrs setup live wizard"));
        assert!(stdout.contains("scryrs.json `remote`"));
    });
}

#[test]
fn setup_live_wizard_preserves_resolved_values() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();
        // ingest_url provided as a flag; only workspace_id needs prompting.
        let mut prompt = FakePrompt::new(
            vec![FakeTextResponse::Value("ws-prompted")],
            vec![FakeConfirmResponse::Value(true)],
        );

        assert_eq!(
            setup::execute_setup_with_prompt(
                &mut out,
                &mut err,
                SetupMode::Live,
                "http://scryrs:8081",
                "",
                None,
                "",
                "",
                tty_options(false),
                &mut prompt,
            ),
            0,
            "stderr: {}",
            String::from_utf8_lossy(&err)
        );
        assert_eq!(prompt.seen_fields, vec!["workspace_id"]);
        let stdout = String::from_utf8_lossy(&out);
        assert!(stdout.contains("scryrs.json remote.ingest_url = http://scryrs:8081"));
    });
}

#[test]
fn setup_live_wizard_with_compose_prompts_docker_network() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut prompt = FakePrompt::new(
            vec![
                FakeTextResponse::Value("http://scryrs:8081"),
                FakeTextResponse::Value("ws1"),
                FakeTextResponse::Value("agent-net"),
            ],
            vec![FakeConfirmResponse::Value(true)],
        );

        assert_eq!(
            setup::execute_setup_with_prompt(
                &mut out,
                &mut err,
                SetupMode::Live,
                "",
                "",
                None,
                "",
                "",
                tty_options(true),
                &mut prompt,
            ),
            0,
            "stderr: {}",
            String::from_utf8_lossy(&err)
        );
        assert_eq!(
            prompt.seen_fields,
            vec!["ingest_url", "workspace_id", "docker_network"]
        );
        let parsed = read_manifest(dir.path());
        assert_eq!(parsed["remote"]["docker_network"], "agent-net");
        assert!(dir.path().join(".scryrs/compose.yml").exists());
    });
}

#[test]
fn setup_live_non_interactive_fails_fast_without_prompting() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut prompt = FakePrompt::new(vec![], vec![]);

        assert_eq!(
            setup::execute_setup_with_prompt(
                &mut out,
                &mut err,
                SetupMode::Live,
                "",
                "ws1",
                None,
                "",
                "",
                SetupExecutionOptions {
                    no_interactive: true,
                    with_compose: false,
                    terminal_state: TerminalState {
                        stdin_is_terminal: true,
                        stdout_is_terminal: true,
                    },
                },
                &mut prompt,
            ),
            2
        );
        assert!(out.is_empty(), "no prompt output");
        assert!(prompt.seen_fields.is_empty(), "wizard must not start");
        assert!(String::from_utf8_lossy(&err).contains("ingest_url is required"));
        assert!(!dir.path().join(".scryrs").exists());
        assert!(!dir.path().join("scryrs.json").exists());
    });
}

#[test]
fn setup_live_non_terminal_fails_fast_without_prompting() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut prompt = FakePrompt::new(vec![], vec![]);

        assert_eq!(
            setup::execute_setup_with_prompt(
                &mut out,
                &mut err,
                SetupMode::Live,
                "",
                "ws1",
                None,
                "",
                "",
                SetupExecutionOptions {
                    no_interactive: false,
                    with_compose: false,
                    terminal_state: TerminalState {
                        stdin_is_terminal: false,
                        stdout_is_terminal: true,
                    },
                },
                &mut prompt,
            ),
            2
        );
        assert!(prompt.seen_fields.is_empty());
        assert!(!dir.path().join("scryrs.json").exists());
    });
}

#[test]
fn setup_live_wizard_cancellation_leaves_no_partial_files() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e}"));
    with_cwd(dir.path(), || {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut prompt = FakePrompt::new(vec![FakeTextResponse::Cancelled], vec![]);

        assert_eq!(
            setup::execute_setup_with_prompt(
                &mut out,
                &mut err,
                SetupMode::Live,
                "",
                "",
                None,
                "",
                "",
                tty_options(false),
                &mut prompt,
            ),
            2
        );
        assert!(prompt.seen_confirms.is_empty());
        assert!(!dir.path().join(".scryrs").exists());
        assert!(!dir.path().join("scryrs.json").exists());
    });
}
