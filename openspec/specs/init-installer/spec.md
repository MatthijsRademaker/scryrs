# init-installer Specification

## Purpose
TBD - created by archiving change task-6fabface-1a09-4ce2-956c-ee1ab783d60a. Update Purpose after archive.
## Requirements
### Requirement: Init subcommand is discoverable via CLI dispatch, help, and help-json

The `init` subcommand SHALL remain accessible through CLI dispatch, help text, and help-json, and those discovery surfaces SHALL describe both local and live setup. The machine-readable `init` entry SHALL include `--agent`, `--mode`, and the live-mode remote configuration arguments.

#### Scenario: Help output documents the mode choice

- **WHEN** `scryrs --help` is invoked
- **THEN** the output includes the `init` command
- **AND** it documents `--mode local|live`
- **AND** it describes the live-mode remote configuration inputs

#### Scenario: Help-json exposes live init arguments

- **WHEN** `scryrs --help-json` is invoked
- **THEN** the `commands` array contains an `init` entry
- **AND** that entry includes `--agent` and `--mode`
- **AND** it includes the live-mode remote configuration arguments in the machine-readable surface

### Requirement: Hook source artifacts are embedded at compile time

The installer SHALL embed reference hook source file contents at compile time using `include_str!()` with paths relative to `crates/scryrs-cli/src/`. The binary SHALL be self-contained and SHALL NOT read hook source files from disk at runtime.

#### Scenario: Claude Code hook source is embedded

- **GIVEN** the file `hooks/claude-code/scryrs-hook.mjs` exists in the repository
- **WHEN** `crates/scryrs-cli` is compiled
- **THEN** the file contents are embedded in the binary via `include_str!("../../../hooks/claude-code/scryrs-hook.mjs")`
- **AND** the binary contains the full hook source as a static string

#### Scenario: Pi hook source is embedded

- **GIVEN** the file `hooks/pi/index.ts` exists in the repository
- **WHEN** `crates/scryrs-cli` is compiled
- **THEN** the file contents are embedded in the binary via `include_str!("../../../hooks/pi/index.ts")`
- **AND** the binary contains the full hook source as a static string

#### Scenario: Binary works without source tree

- **GIVEN** a compiled `scryrs` binary
- **WHEN** the binary is moved to a machine that does not have the scryrs source repository
- **THEN** `scryrs init --agent claude-code` installs the hook using embedded content
- **AND** no filesystem access to `hooks/` is attempted

### Requirement: Harness registry is typed and separate from lib.rs

The installer logic SHALL be implemented in a separate `crates/scryrs-cli/src/init.rs` module with a public `execute_init` function. A private typed harness registry SHALL declare supported agent names, embedded source assets, target directories, and deterministic next-step text.

#### Scenario: Installer module is separate from lib.rs

- **GIVEN** the `crates/scryrs-cli/src/` directory
- **WHEN** inspecting the file structure
- **THEN** file `init.rs` exists and contains all installer logic
- **AND** `lib.rs` imports and delegates to `init::execute_init` for the `init` subcommand

#### Scenario: Registry entries are complete

- **WHEN** inspecting the harness registry in `init.rs`
- **THEN** it declares entries for `claude-code` and `pi`
- **AND** each entry includes the agent name, embedded source string, target relative directory, target filename, and deterministic next-step text
- **AND** the entries are ordered deterministically (alphabetical by agent name)

#### Scenario: Supported harnesses are listed in stable order

- **WHEN** the installer emits an unsupported-harness error or lists supported harnesses
- **THEN** the harness names appear in alphabetical order (`claude-code`, `pi`)
- **AND** the order is deterministic across invocations

### Requirement: Installer writes to project-local consumer directories only

The installer SHALL write hook files to consumer-side project-local directories under a resolved target root. For ordinary consumer projects, resolved target root is the current working directory. For `scryrs init --agent pi` invoked inside the scryrs source checkout, resolved target root is the detected scryrs checkout root. For Claude Code, the target is `.claude/hooks/scryrs-hook.mjs`. For Pi, the target is `.pi/extensions/scryrs/index.ts`. The installer SHALL NOT write to user-global directories (e.g., `~/.pi/agent/extensions/`) in v1.

#### Scenario: Claude Code hook installed to .claude/hooks/

- **WHEN** `scryrs init --agent claude-code` is invoked from a project root outside the scryrs source checkout
- **THEN** the directory `.claude/hooks/` is created if it does not exist
- **AND** file `.claude/hooks/scryrs-hook.mjs` is written with the embedded reference hook content
- **AND** the file content is byte-identical to the embedded source

#### Scenario: Pi hook installed to .pi/extensions/scryrs/

- **WHEN** `scryrs init --agent pi` is invoked from a project root outside the scryrs source checkout
- **THEN** the directory `.pi/extensions/scryrs/` is created if it does not exist
- **AND** file `.pi/extensions/scryrs/index.ts` is written with the embedded reference hook content
- **AND** the file content is byte-identical to the embedded source

#### Scenario: Source-repo Pi install writes to repository root

- **GIVEN** the current working directory is the scryrs source checkout root
- **WHEN** `scryrs init --agent pi` is invoked
- **THEN** file `.pi/extensions/scryrs/index.ts` is written under the scryrs repository root
- **AND** the file content is byte-identical to the embedded source

#### Scenario: Source-repo Pi install from subdirectory resolves to repository root

- **GIVEN** the current working directory is a subdirectory of the scryrs source checkout
- **WHEN** `scryrs init --agent pi` is invoked
- **THEN** the installer resolves the scryrs checkout root from ancestry markers
- **AND** file `.pi/extensions/scryrs/index.ts` is written under that checkout root
- **AND** no nested subdirectory-local `.pi/extensions/scryrs/` tree is created beneath the caller CWD

#### Scenario: No files written outside resolved target root

- **WHEN** `scryrs init --agent <name>` is invoked
- **THEN** all files are written under the resolved target root for that invocation
- **AND** no files are written under `$HOME`, `/tmp`, or any absolute path outside that root

### Requirement: Unsupported harnesses fail loudly with deterministic guidance

When an unrecognized agent name is provided, the installer SHALL exit with code 2, print a deterministic error message listing the supported harness names in stable alphabetical order, and SHALL NOT attempt any filesystem operations.

#### Scenario: Unknown harness exits 2

- **WHEN** `scryrs init --agent unknown` is invoked
- **THEN** the exit code is 2
- **AND** stderr contains an error message indicating `unknown` is not a supported harness
- **AND** stderr lists the supported harnesses (`claude-code`, `pi`)
- **AND** no files or directories are created

#### Scenario: Case-sensitive matching

- **WHEN** `scryrs init --agent CLAUDE-CODE` is invoked
- **THEN** the exit code is 2 (case-sensitive match fails)
- **AND** stderr lists the supported harnesses in their canonical casing

### Requirement: Pre-existing target files trigger loud refusal

The installer SHALL preserve existing managed files deterministically. For file-based harness installs, identical existing managed content SHALL be treated as a successful no-op, while divergent existing content SHALL still fail loudly instead of being overwritten.

#### Scenario: Identical Pi runtime copy is accepted as no-op

- **GIVEN** `.pi/extensions/scryrs/index.ts` already exists and is byte-identical to the embedded Pi hook source
- **WHEN** `scryrs init --agent pi` is invoked again
- **THEN** the exit code is 0
- **AND** the existing file content is preserved unchanged
- **AND** the installer continues with the rest of the requested init work

#### Scenario: Divergent Pi runtime copy is refused

- **GIVEN** `.pi/extensions/scryrs/index.ts` already exists and differs from the embedded Pi hook source
- **WHEN** `scryrs init --agent pi` is invoked
- **THEN** the exit code is 2
- **AND** stderr reports the existing path conflict with remediation guidance
- **AND** the divergent file is not overwritten

#### Scenario: Pi target directory already exists but target file does not

- **GIVEN** `.pi/extensions/scryrs/` already exists but contains no `index.ts`
- **WHEN** `scryrs init --agent pi` is invoked
- **THEN** the installer creates `index.ts` inside the existing directory
- **AND** the exit code is 0

### Requirement: Claude Code installs via create-or-merge of .claude/settings.json

The Claude Code installer SHALL create-or-merge `.claude/settings.json` with a native `PreToolUse` command hook invoking `scryrs hook claude-code`. It SHALL preserve unrelated top-level keys, SHALL be idempotent across re-runs (the hook SHALL NOT be duplicated), and SHALL refuse to overwrite a non-object `settings.json`. The installer SHALL NOT write a `.mjs` file.

#### Scenario: settings.json is created when absent

- **GIVEN** `.claude/settings.json` does not exist
- **WHEN** `scryrs init --agent claude-code` is invoked
- **THEN** `.claude/settings.json` is created
- **AND** it contains a `hooks.PreToolUse` array with a single entry invoking `scryrs hook claude-code`
- **AND** the exit code is 0

#### Scenario: settings.json merge preserves unrelated keys

- **GIVEN** `.claude/settings.json` already exists with top-level keys unrelated to hooks
- **WHEN** `scryrs init --agent claude-code` is invoked
- **THEN** the installer inserts the `PreToolUse` hook entry
- **AND** the unrelated top-level keys are preserved unchanged
- **AND** the exit code is 0

#### Scenario: settings.json re-run is idempotent

- **GIVEN** a prior `scryrs init --agent claude-code` has written the PreToolUse hook
- **WHEN** `scryrs init --agent claude-code` is invoked again
- **THEN** the hook command appears exactly once in the PreToolUse array
- **AND** the exit code is 0

#### Scenario: non-object settings.json is refused

- **GIVEN** `.claude/settings.json` exists and is a JSON array or scalar (not an object)
- **WHEN** `scryrs init --agent claude-code` is invoked
- **THEN** the exit code is 2
- **AND** the existing file is not modified

### Requirement: Self-install into scryrs source checkout is refused

The installer SHALL continue to detect the scryrs source checkout by ancestry markers. `pi` local-mode installation inside the source checkout remains permitted for dogfooding. Live mode SHALL be refused inside the scryrs source checkout because live init configures consumer-project remote state.

#### Scenario: Local-mode Pi dogfooding remains allowed

- **GIVEN** the current working directory is the scryrs source checkout
- **WHEN** `scryrs init --agent pi --mode local` is invoked
- **THEN** installation proceeds against `.pi/extensions/scryrs/index.ts` at the checkout root

#### Scenario: Live mode is refused in the source checkout

- **GIVEN** the current working directory is the scryrs source checkout
- **WHEN** `scryrs init --agent pi --mode live` is invoked
- **THEN** the exit code is 2
- **AND** stderr explains that live mode must target a consumer project rather than the scryrs source repository
- **AND** no `scryrs.json` changes are written in the source checkout

### Requirement: Successful install prints deterministic next-step text

On successful installation, the installer SHALL print deterministic, mode-specific next-step instructions to stdout. Local mode SHALL keep the current local guidance. Live mode SHALL explain that the project is configured for remote ingest, identify the configured live-server endpoint, and direct the operator to launch the managed workspace compose stack with `scryrs up`.

#### Scenario: Local mode keeps current next steps

- **WHEN** `scryrs init --agent claude-code --mode local` completes successfully
- **THEN** stdout contains the existing local next-step guidance
- **AND** it does not mention Docker or a remote ingest URL

#### Scenario: Live mode prints workspace-bootstrap next steps

- **WHEN** `scryrs init --agent claude-code --mode live` completes successfully
- **THEN** stdout states that live remote ingest was configured
- **AND** stdout includes the remote server endpoint for the project
- **AND** stdout tells the operator to start the managed live server with `scryrs up`
- **AND** stdout does not tell the operator to check out the scryrs source repository and run the root `docker-compose.yml`

### Requirement: Installer does not create or depend on scryrs.json

The installer SHALL remain manifest-agnostic in local mode. In live mode only, it MAY create or merge the target project's `scryrs.json` `remote` section as part of remote setup. It SHALL NOT read or depend on `scryrs.json` for ordinary local installation behavior.

#### Scenario: Local mode still does not create a manifest

- **WHEN** `scryrs init --agent <name> --mode local` completes successfully
- **THEN** no `scryrs.json` file is created or modified

#### Scenario: Live mode may create the remote manifest section

- **WHEN** `scryrs init --agent <name> --mode live` completes successfully in a project without `scryrs.json`
- **THEN** the installer creates `scryrs.json` at the project root
- **AND** the file contains the configured `remote` section needed for live ingest
- **AND** no unrelated manifest structure is required for success

### Requirement: Source-repo Pi install remains non-canonical runtime state

When `scryrs init --agent pi` is used inside the scryrs source checkout, `hooks/pi/index.ts` SHALL remain the only canonical hook source in the repository. The installed file at `.pi/extensions/scryrs/index.ts` SHALL be treated as runtime copy only. Repository maintainer guidance SHALL explicitly state that LLMs/agents MUST NOT edit the installed copy directly or treat it as leading source.

#### Scenario: AGENTS guidance defines canonical Pi hook source
- **WHEN** a maintainer or agent reads `AGENTS.md`
- **THEN** the file states that `hooks/pi/index.ts` is canonical source for the Pi hook
- **AND** the file states that `.pi/extensions/scryrs/index.ts` is installed runtime copy only
- **AND** the file states that LLMs/agents MUST NOT edit the installed copy directly

#### Scenario: Installed Pi copy is excluded from normal git noise
- **WHEN** `scryrs init --agent pi` is run inside the scryrs source checkout
- **THEN** the created `.pi/extensions/scryrs/` artifact path is ignored by repository ignore rules
- **AND** local dogfooding does not require committing installed Pi hook copy

#### Scenario: Claude Code consumer config remains blocked in source repo
- **GIVEN** CWD is the scryrs source checkout
- **WHEN** `scryrs init --agent claude-code` is invoked
- **THEN** the exit code is 2
- **AND** no `.claude/` consumer config files are created or modified inside the scryrs source checkout

### Requirement: Existing CLI behavior is not regressed

All existing commands (`hotspots`, `record`), flags (`--help`, `--version`, `--help-json`, `-h`, `-V`, `-hj`), bare invocation, error messages, exit codes, and JSON output formats SHALL remain unchanged.

#### Scenario: hotspots behavior unchanged

- **WHEN** `scryrs hotspots /tmp` is invoked
- **THEN** the output is the same versioned JSON envelope as before
- **AND** the exit code is 0

#### Scenario: record behavior unchanged

- **WHEN** `scryrs record --stdin` is invoked with valid JSONL on stdin
- **THEN** events are ingested as before
- **AND** the stdout summary envelope format is unchanged

#### Scenario: --help-json still includes existing commands

- **WHEN** `scryrs --help-json` is invoked
- **THEN** the `commands` array still contains `hotspots` and `record` entries with all existing metadata
- **AND** the `init` entry is added alongside them

#### Scenario: previously_stubbed_commands_exit_2 test still passes

- **WHEN** the test `previously_stubbed_commands_exit_2` runs
- **THEN** commands `trace`, `propose`, `graph`, `route`, `adapters`, `report`, `suggest-docs` all exit 2
- **AND** the `init` command is not in this list (it is now a real command)

### Requirement: Exit codes follow 0/1/2 contract

The `init` subcommand SHALL use the same exit code semantics as the rest of the CLI: 0 for success, 1 for I/O errors (write failure), 2 for usage errors (unsupported harness, collision, self-install, missing `--agent`).

#### Scenario: Successful install exits 0

- **WHEN** `scryrs init --agent claude-code` succeeds
- **THEN** the exit code is 0

#### Scenario: Write failure exits 1

- **GIVEN** the target directory is on a read-only filesystem
- **WHEN** `scryrs init --agent claude-code` attempts to write
- **THEN** the exit code is 1

#### Scenario: Usage errors exit 2

- **WHEN** `scryrs init --agent unknown` is invoked
- **THEN** the exit code is 2

### Requirement: Scope is limited to scryrs-cli crate and project docs

This change SHALL NOT modify any Rust crate outside `scryrs-cli`. It SHALL NOT modify hook business logic, TraceEvent schema, `scryrs record` behavior, or any existing OpenSpec capability specs for `trace-hook-contract`, `claude-code-reference-hook`, or `pi-reference-hook`.

#### Scenario: No changes to scryrs-types

- **WHEN** this change is implemented
- **THEN** no files in `crates/scryrs-types/` are modified

#### Scenario: No changes to scryrs-core

- **WHEN** this change is implemented
- **THEN** no files in `crates/scryrs-core/` are modified

#### Scenario: No changes to hook source files

- **WHEN** this change is implemented
- **THEN** `hooks/claude-code/scryrs-hook.mjs` is unchanged
- **AND** `hooks/pi/index.ts` is unchanged
- **AND** the installer embeds their content as-is without transformation

#### Scenario: No changes to existing capability specs

- **WHEN** this change is implemented
- **THEN** `openspec/specs/trace-hook-contract/spec.md` is unchanged
- **AND** `openspec/specs/claude-code-reference-hook/spec.md` is unchanged
- **AND** `openspec/specs/pi-reference-hook/spec.md` is unchanged
- **AND** `openspec/specs/cli-clap-migration/spec.md` is unchanged
- **AND** `openspec/specs/cli-machine-surface/spec.md` is unchanged

### Requirement: Init supports explicit local and live setup modes

The `init` subcommand SHALL accept `--mode local|live` with `live` as the default. Local mode SHALL preserve the existing local-install flow. Live mode SHALL use deterministic CLI inputs for `ingest_url`, `workspace_id`, and required `docker_network`, with optional `repository_id` and optional `agent_id`. When `repository_id` is omitted, the installer SHALL derive it from Git remote origin using the normalized repository identity contract required by live ingest. When `agent_id` is omitted, the installer SHALL NOT require it and SHALL leave it to runtime autogeneration; `agent_id` is not a required live-mode input. Missing or invalid live-mode configuration (a missing `ingest_url`, `workspace_id`, or `docker_network`, or an underivable `repository_id`) SHALL exit 2 before the installer writes hook files, `.scryrs/` artifacts, or `scryrs.json`.

#### Scenario: Default init validates complete live bootstrap inputs before writing

- **WHEN** `scryrs init --agent pi` or `scryrs init --agent claude-code` is invoked without `--mode`
- **THEN** the installer resolves live-mode bootstrap inputs
- **AND** the command succeeds only after `ingest_url`, `workspace_id`, and `docker_network` are all valid and `repository_id` is resolvable
- **AND** no partial files are written before that validation completes

#### Scenario: agent_id is not a required live-mode input

- **WHEN** `scryrs init --agent claude-code --mode live` is invoked without an `agent_id` value
- **THEN** the command does not fail for a missing `agent_id`
- **AND** no `agent_id` is written into committed config
- **AND** `agent_id` is left to runtime autogeneration

#### Scenario: Invalid live mode fails without partial writes

- **WHEN** `scryrs init --agent claude-code --mode live` is invoked with a missing or empty required live-mode field (`ingest_url`, `workspace_id`, or `docker_network`)
- **THEN** the exit code is 2
- **AND** stderr contains deterministic validation guidance
- **AND** no hook files, `.scryrs/` artifacts, or `scryrs.json` changes are written

#### Scenario: Local mode remains an explicit opt-in

- **WHEN** `scryrs init --agent pi --mode local` or `scryrs init --agent claude-code --mode local` is invoked
- **THEN** the installer behaves as the existing local-mode path
- **AND** no live bootstrap scaffold is required
- **AND** no network-specific next-step text is emitted

### Requirement: Live-mode init writes project remote config without local-store scaffolding

In live mode, the installer SHALL install the same harness hook transport, create `.scryrs/`, `.scryrs/.gitignore`, `.scryrs/hooks/`, `.scryrs/.env`, and `.scryrs/compose.yml` under the target project, and create-or-merge the target project's `scryrs.json` so that `scryrs.json` is the single committed source of truth for live config. It SHALL write only the committed shared constants — `ingest_url`, `workspace_id`, and `docker_network` — into the `remote` object, preserving unrelated manifest keys. It SHALL NOT write `repository_id` or `agent_id` into committed config (these are derived/autogenerated at resolution time). The scaffolded `.scryrs/.env` SHALL be created as an overrides-only stub: the installer SHALL NOT pre-populate it with managed ingest identity or the Docker network value. It SHALL NOT create `.scryrs/scryrs.db`, SHALL NOT dual-write managed values into both `scryrs.json` and `.scryrs/.env`, and SHALL NOT add direct HTTP logic to hook artifacts.

#### Scenario: Live mode scaffolds compose and overrides-only env

- **WHEN** `scryrs init --agent pi --mode live` succeeds
- **THEN** `.scryrs/`, `.scryrs/.gitignore`, `.scryrs/hooks/`, `.scryrs/.env`, and `.scryrs/compose.yml` exist under the target project
- **AND** `.scryrs/scryrs.db` does not exist
- **AND** the scaffolded `.scryrs/.env` does not contain managed ingest identity or `SCRYRS_DOCKER_NETWORK` values

#### Scenario: Live mode writes only shared constants into the manifest

- **WHEN** `scryrs init --agent claude-code --mode live` succeeds
- **THEN** `scryrs.json` `remote` contains `ingest_url`, `workspace_id`, and `docker_network`
- **AND** `scryrs.json` `remote` does not contain `repository_id` or `agent_id`
- **AND** the same managed values are not duplicated into `.scryrs/.env`

#### Scenario: Live mode merges into an existing manifest

- **GIVEN** a target project already has a `scryrs.json` file with unrelated top-level keys
- **WHEN** `scryrs init --agent claude-code --mode live` succeeds
- **THEN** the installer updates only the `remote` section
- **AND** unrelated manifest keys remain unchanged
- **AND** the workspace-local `.scryrs/compose.yml` is created or preserved as a managed bootstrap file

#### Scenario: Conflicting committed manifest value fails loudly

- **GIVEN** a target project `scryrs.json` already contains a `remote.ingest_url`, `remote.workspace_id`, or `remote.docker_network` value that conflicts with the resolved live bootstrap inputs
- **WHEN** `scryrs init --mode live` is invoked
- **THEN** the exit code is 2
- **AND** stderr reports the conflicting manifest field with remediation guidance
- **AND** no partial files are written

