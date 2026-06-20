## 1. Create installer module with harness registry

- [x] 1.1 Create `crates/scryrs-cli/src/init.rs` with a `pub fn execute_init(out: &mut impl Write, err: &mut impl Write, agent_name: &str) -> i32` entry point
- [x] 1.2 Define a private `HarnessEntry` struct with fields: `agent_name: &'static str`, `source_asset: &'static str` (embedded via `include_str!`), `target_dir: &'static str` (relative to CWD), `target_filename: &'static str`, and `next_steps: &'static str` (deterministic post-install instructions)
- [x] 1.3 Define the `HARNESS_REGISTRY` array with two entries: `claude-code` (source → `include_str!("../../../hooks/claude-code/scryrs-hook.mjs")`, target → `.claude/hooks/scryrs-hook.mjs`) and `pi` (source → `include_str!("../../../hooks/pi/index.ts")`, target → `.pi/extensions/pi-trace/index.ts`)
- [x] 1.4 Implement deterministic supported-harness listing with stable ordering (alphabetical)

## 2. Implement self-install boundary detection

- [x] 2.1 Implement `fn is_scryrs_source_checkout(cwd: &Path) -> bool` that walks parent directories looking for `Cargo.toml` containing `scryrs-cli` in `[workspace.members]` AND a sibling `hooks/claude-code/` directory
- [x] 2.2 If both markers found, refuse installation with a deterministic error on stderr explaining the reference-only boundary and exit 2

## 3. Implement harness-specific install logic

- [x] 3.1 Implement Claude Code install: create `.claude/hooks/` directory, check for pre-existing `.claude/settings.json` (refuse with insertion instructions and exit 2 if exists), write embedded `scryrs-hook.mjs` content
- [x] 3.2 Implement Pi install: create `.pi/extensions/pi-trace/` directory, write embedded `index.ts` content
- [x] 3.3 For both harnesses: if target file already exists, exit 2 with deterministic collision error and remediation instructions (remove file manually and rerun)
- [x] 3.4 On success, print harness-specific next-step text to stdout

## 4. Wire `init` into CLI dispatch

- [x] 4.1 Add `"init"` to the pre-clap known-command condition in `lib.rs` (alongside `"hotspots"` and `"record"`)
- [x] 4.2 Add `init` as a clap subcommand in the `Command::new("scryrs")` builder with a required `--agent <NAME>` argument
- [x] 4.3 Add dispatch arm for `Some(("init", m))` in the clap subcommand match that extracts `--agent` value and calls `init::execute_init`
- [x] 4.4 Handle missing `--agent` argument via clap's built-in `MissingRequiredArgument` error (consistent with existing error formatting)
- [x] 4.5 Import `init` module in `lib.rs`

## 5. Update CLI surfaces

- [x] 5.1 Bump `SURFACE_VERSION` from `"0.2.0"` to `"0.3.0"`
- [x] 5.2 Add `init` entry to `cli_surface_doc()` commands array with name, description, arguments (`--agent`: required, string), output contract (stdout format, exit codes)
- [x] 5.3 Update `write_help()` to include the `init --agent <name>` command with description, supported harnesses, and deterministic behavior notes
- [x] 5.4 Run snapshot tests with `INSTA_UPDATE=1` to accept new help text snapshots

## 6. Update project documentation

- [x] 6.1 Update `README.md` — change "The CLI ships two commands" to reflect three commands; add `scryrs init --agent claude-code` and `scryrs init --agent pi` examples to the quickstart
- [x] 6.2 Update `.devagent/docs/docs/cli-v0-contract.md` — add `init` to the unknown commands section (as a now-known command); add `init` contract section documenting `--agent <NAME>`, supported harnesses, exit codes, and deterministic behavior
- [x] 6.3 Update `.devagent/docs/docs/trace-hook-contract.md` — remove "Hook installation is currently a manual process pending `scryrs init --agent` installer" language; replace with "Hook installation is automated via `scryrs init --agent <name>`"; update Reference Hooks section to remove "forthcoming Phase 1 deliverable" for Pi hook and state it exists at `hooks/pi/`

## 7. Add tests

- [x] 7.1 Add test: `init --agent claude-code` writes `scryrs-hook.mjs` to `.claude/hooks/` within a tempdir with correct content
- [x] 7.2 Add test: `init --agent pi` writes `index.ts` to `.pi/extensions/pi-trace/` within a tempdir with correct content
- [x] 7.3 Add test: `init --agent unknown` exits 2 with stderr listing supported harnesses in stable order
- [x] 7.4 Add test: `init --agent claude-code` when `.claude/settings.json` exists exits 2 with remediation instructions
- [x] 7.5 Add test: `init --agent claude-code` when `.claude/hooks/scryrs-hook.mjs` already exists exits 2 with collision error
- [x] 7.6 Add test: `init --agent pi` when `.pi/extensions/pi-trace/index.ts` already exists exits 2 with collision error
- [x] 7.7 Add test: self-install detection refuses installation when both Cargo.toml scryrs-cli marker and hooks/claude-code/ directory exist
- [x] 7.8 Add test: self-install detection permits installation into unrelated project (no false positive)
- [x] 7.9 Add test: `init` without `--agent` exits 2 via clap usage error, not unknown-command path
- [x] 7.10 Add test: `init` with empty `--agent` value exits 2 via clap usage error
- [x] 7.11 Add test: `init` help text appears in `--help` output
- [x] 7.12 Add test: `init` entry appears in `--help-json` output with correct metadata
- [x] 7.13 Add test: `previously_stubbed_commands_exit_2` does NOT include `init` (init is a real command)
- [x] 7.14 Add test: `init --agent claude-code` stdout contains deterministic next-step text with reload/PATH instructions
- [x] 7.15 Add test: `init --agent pi` stdout contains deterministic next-step text with reload instructions
- [x] 7.16 Add test: all existing tests pass unchanged (`--help`, `--version`, `hotspots`, `record`, `--help-json`, bare invocation, unknown commands)

## 8. Validation

- [x] 8.1 Run `cargo test -p scryrs-cli` to confirm all tests pass
- [x] 8.2 Run `cargo check -p scryrs-cli` to confirm no warnings
- [x] 8.3 Run `cargo build -p scryrs-cli` to confirm `include_str!` paths resolve at compile time
- [x] 8.4 Manually verify `scryrs init --agent claude-code` in a temp directory creates correct files
- [x] 8.5 Manually verify `scryrs init --agent pi` in a temp directory creates correct files
- [x] 8.6 Manually verify `scryrs init --agent unknown` exits 2 and lists supported harnesses
- [x] 8.7 Manually verify `scryrs --help-json` includes the init command entry with correct structure
