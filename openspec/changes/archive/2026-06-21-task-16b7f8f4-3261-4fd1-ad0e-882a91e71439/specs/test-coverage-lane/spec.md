# test-coverage-lane Specification

## Purpose

Defines requirements for wiring hook verification into the CI and developer-local test lanes, adding installed-hook end-to-end validation, and establishing targeted unit test coverage for the highest-priority scaffold crates. Covers only the gaps identified in the testing inventory — does not require blanket coverage or framework refactoring.

## ADDED Requirements

### Requirement: Fast hook verification runs in CI on hook-related changes

The CI pipeline SHALL run `scripts/hook-test` (Node-only Claude Code hook contract verification) on any pull request that touches files under `hooks/` or `scripts/verification/` paths. The check SHALL fail if the hook produces malformed JSON, alters stdout/stderr, or fails to forward tool inputs to `scryrs record --stdin`.

#### Scenario: Hook PR triggers CI verification lane
- **GIVEN** a pull request that modifies `hooks/claude-code/scryrs-hook.mjs`
- **WHEN** CI runs the new hook verification check
- **THEN** `scripts/hook-test` executes successfully (exit code 0)
- **AND** the check reports a green status on the PR

#### Scenario: Broken hook contract fails CI
- **GIVEN** a pull request that introduces a JSON-shaping regression in the Claude Code hook
- **WHEN** CI runs `scripts/hook-test`
- **THEN** the check exits with a non-zero status
- **AND** the PR is blocked from merging

#### Scenario: Non-hook PR does not trigger hook verification
- **GIVEN** a pull request that only modifies Rust source files under `crates/`
- **WHEN** CI runs
- **THEN** the hook verification check is skipped or passes trivially without executing `scripts/hook-test`

### Requirement: Full cross-harness verification is gated by measured runtime

Before `scripts/verify-trace-capture` (Docker-backed, builds release binary, runs both Claude Code and Pi e2e fixtures) is assigned to a CI lane, its wall-clock runtime SHALL be measured under CI-like conditions. Based on the measurement: if runtime is <3 minutes it MAY be a PR gate candidate; if runtime is 3–10 minutes it SHALL be assigned to a nightly lane; if runtime is >10 minutes, optimization SHALL be investigated before CI gating.

#### Scenario: Runtime measurement is documented
- **GIVEN** a CI-like environment with Docker available
- **WHEN** `scripts/verify-trace-capture` is executed with both fixtures
- **THEN** the wall-clock runtime is recorded and documented as a comment or artifact
- **AND** a lane assignment decision is made based on the measured threshold (<3 min, 3–10 min, >10 min)

#### Scenario: Nightly lane captures verification regressions
- **GIVEN** `scripts/verify-trace-capture` is assigned to a nightly CI lane
- **WHEN** a nightly run encounters a cross-harness regression (e.g., scryrs binary fails to accept stdin input)
- **THEN** the nightly run reports failure
- **AND** the failure is surfaced through the CI notification channel

### Requirement: Installed-hook end-to-end validation proves init output works

The system SHALL include an end-to-end test that runs `scryrs init --agent claude-code` and `scryrs init --agent pi` in a temporary consumer-style project directory, loads the installed hook artifacts from their consumer locations, and exercises tool-capture forwarding against the real `scryrs` binary. The test SHALL prove that installed artifacts are loadable and functional, not merely that files were created.

#### Scenario: Claude Code installed hook is loadable and functional
- **GIVEN** a temporary consumer project directory
- **WHEN** `scryrs init --agent claude-code` is executed in that directory
- **THEN** the installed `scryrs-hook.mjs` file exists at the expected consumer-relative path
- **AND** the installed file can be loaded as a Node.js module without import errors
- **AND** when exercised with a valid tool input (e.g., Bash tool), the hook forwards the event to `scryrs record --stdin` and produces a valid event in `.scryrs/scryrs.db`
- **AND** the hook writes zero bytes to stdout and stderr during tool forwarding

#### Scenario: Pi installed hook is loadable and functional
- **GIVEN** a temporary consumer project directory
- **WHEN** `scryrs init --agent pi` is executed in that directory
- **THEN** the installed Pi hook artifact exists at the expected consumer-relative path
- **AND** the installed artifact can be loaded (TypeScript transpiled or imported) without errors
- **AND** when exercised with a simulated `tool_result` event for Bash, the hook forwards the event to `scryrs record --stdin` and produces a valid event in `.scryrs/scryrs.db`
- **AND** the hook returns `undefined` (non-interfering)

#### Scenario: Installed hook e2e test does not depend on repository source
- **GIVEN** the installed-hook end-to-end test is executing
- **WHEN** the test loads hook artifacts
- **THEN** artifacts are loaded from the temporary consumer project directory, NOT from `hooks/claude-code/` or `hooks/pi/` in the repository source
- **AND** the test works independently of the repository source tree after `scryrs init` completes

#### Scenario: Failed init is detected by the e2e test
- **GIVEN** `scryrs init --agent claude-code` fails (e.g., due to a bug that produces a corrupt hook file)
- **WHEN** the installed-hook e2e test attempts to load the installed artifact
- **THEN** the test fails with a diagnostic message indicating which artifact could not be loaded
- **AND** the test does not silently skip the load step

### Requirement: scryrs-curator gains unit test coverage for propose_from_hotspot

The `scryrs-curator` crate SHALL include unit tests for the `propose_from_hotspot` function covering: proposal title contains the hotspot subject, rationale references subject and score, and edge-case handling of empty counts and evidence.

#### Scenario: Proposal title includes hotspot subject
- **GIVEN** a `HotspotEntry` with `subject = "routing"` and `score = 3`
- **WHEN** `propose_from_hotspot` is called
- **THEN** the returned `KnowledgeProposal.title` contains the string `"routing"`

#### Scenario: Proposal rationale includes subject and score
- **GIVEN** a `HotspotEntry` with `subject = "routing"` and `score = 5`
- **WHEN** `propose_from_hotspot` is called
- **THEN** the returned `KnowledgeProposal.rationale` contains both `"routing"` and `"5"`

#### Scenario: Empty counts and evidence are handled without panic
- **GIVEN** a `HotspotEntry` with empty `HotspotCounts` (all maps empty) and empty `HotspotEvidence` (rowIds empty)
- **WHEN** `propose_from_hotspot` is called
- **THEN** the function returns a valid `KnowledgeProposal` without panicking

### Requirement: scryrs-sandbox gains unit test coverage for ToolPolicy

The `scryrs-sandbox` crate SHALL include unit tests for `ToolPolicy` covering: `read_only` constructor correctness, `can_write` path-matching for allowed and disallowed paths, and `can_write` rejection when the allowlist is empty.

#### Scenario: read_only constructor sets correct defaults
- **GIVEN** `ToolPolicy::read_only` is called with paths `["/repo"]`
- **WHEN** the constructed policy is inspected
- **THEN** `allow_read_fs` contains `"/repo"`
- **AND** `allow_write_fs` is empty
- **AND** `allow_exec` is empty
- **AND** `confirm_before_write` is `true`
- **AND** `confirm_before_exec` is `true`

#### Scenario: can_write permits paths under allowed prefix
- **GIVEN** a `ToolPolicy` with `allow_write_fs = [PathBuf::from("/tmp/project")]`
- **WHEN** `can_write(Path::new("/tmp/project/src/main.rs"))` is called
- **THEN** the result is `true`

#### Scenario: can_write rejects paths outside allowed prefixes
- **GIVEN** a `ToolPolicy` with `allow_write_fs = [PathBuf::from("/tmp/project")]`
- **WHEN** `can_write(Path::new("/etc/passwd"))` is called
- **THEN** the result is `false`

#### Scenario: can_write rejects all paths when allowlist is empty
- **GIVEN** a `ToolPolicy` with an empty `allow_write_fs`
- **WHEN** `can_write(Path::new("/any/path"))` is called
- **THEN** the result is `false`

### Requirement: Developer-local test lane includes hook verification

The default developer-local test lane SHALL provide a documented option to run hook verification alongside Rust tests so that developers can run the full suite from a single entrypoint without discovering separate scripts.

#### Scenario: Full lane runs hook verification after Rust tests
- **GIVEN** a developer invokes the full test lane (e.g., `scripts/test --full` or `scripts/test-all`)
- **WHEN** the lane executes
- **THEN** `cargo test --workspace --all-targets --all-features --locked` runs first
- **AND** `scripts/hook-test` runs after Rust tests complete
- **AND** the lane exits with a non-zero status if either phase fails

#### Scenario: Default lane remains hook-verification-free
- **GIVEN** a developer invokes `scripts/test` without flags
- **WHEN** the lane executes
- **THEN** only `cargo test --workspace` runs (existing behavior preserved)
- **AND** hook verification is not executed

#### Scenario: Full lane is documented in test script help
- **GIVEN** a developer runs `scripts/test --help`
- **WHEN** help output is displayed
- **THEN** the output documents the `--full` flag or alternative entrypoint for running hook verification
