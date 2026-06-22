# init-installer Specification

## Purpose
TBD - created by archiving change task-6fabface-1a09-4ce2-956c-ee1ab783d60a. Update Purpose after archive.
## Requirements
### Requirement: Init subcommand is discoverable via CLI dispatch, help, and help-json

The `init` subcommand SHALL be accessible through the pre-clap command whitelist, documented in `write_help()`, and exposed in the `--help-json` machine-readable surface at `SURFACE_VERSION 0.3.0`.

#### Scenario: init reaches clap dispatch

- **GIVEN** the pre-clap unknown-command check in `lib.rs`
- **WHEN** `scryrs init --agent claude-code` is invoked
- **THEN** the first argument `"init"` is recognized as a known command
- **AND** arguments reach clap's `try_get_matches_from` for subcommand dispatch
- **AND** the exit code is 0 (on successful install)

#### Scenario: init appears in --help output

- **WHEN** `scryrs --help` is invoked
- **THEN** the output includes the `init --agent <name>` command with description and supported harnesses
- **AND** the output mentions `claude-code` and `pi` as supported harness names

#### Scenario: init appears in --help-json surface

- **WHEN** `scryrs --help-json` is invoked
- **THEN** the JSON output includes `"surfaceVersion": "0.3.0"`
- **AND** the `commands` array contains an entry with `"name": "init"`
- **AND** the entry includes `arguments` (with `--agent` as required string) and `output` contract metadata

#### Scenario: init without --agent exits 2 via clap usage error

- **WHEN** `scryrs init` is invoked without `--agent`
- **THEN** clap reports a `MissingRequiredArgument` error
- **AND** the exit code is 2
- **AND** stderr follows the contract three-line format with problem description, usage, and `See \`scryrs --help\``

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

The installer SHALL write hook files to consumer-side project-local directories under a resolved target root. For ordinary consumer projects, resolved target root is the current working directory. For `scryrs init --agent pi` invoked inside the scryrs source checkout, resolved target root is the detected scryrs checkout root. For Claude Code, the target is `.claude/hooks/scryrs-hook.mjs`. For Pi, the target is `.pi/extensions/pi-trace/index.ts`. The installer SHALL NOT write to user-global directories (e.g., `~/.pi/agent/extensions/`) in v1.

#### Scenario: Claude Code hook installed to .claude/hooks/

- **WHEN** `scryrs init --agent claude-code` is invoked from a project root outside the scryrs source checkout
- **THEN** the directory `.claude/hooks/` is created if it does not exist
- **AND** file `.claude/hooks/scryrs-hook.mjs` is written with the embedded reference hook content
- **AND** the file content is byte-identical to the embedded source

#### Scenario: Pi hook installed to .pi/extensions/pi-trace/

- **WHEN** `scryrs init --agent pi` is invoked from a project root outside the scryrs source checkout
- **THEN** the directory `.pi/extensions/pi-trace/` is created if it does not exist
- **AND** file `.pi/extensions/pi-trace/index.ts` is written with the embedded reference hook content
- **AND** the file content is byte-identical to the embedded source

#### Scenario: Source-repo Pi install writes to repository root

- **GIVEN** the current working directory is the scryrs source checkout root
- **WHEN** `scryrs init --agent pi` is invoked
- **THEN** file `.pi/extensions/pi-trace/index.ts` is written under the scryrs repository root
- **AND** the file content is byte-identical to the embedded source

#### Scenario: Source-repo Pi install from subdirectory resolves to repository root

- **GIVEN** the current working directory is a subdirectory of the scryrs source checkout
- **WHEN** `scryrs init --agent pi` is invoked
- **THEN** the installer resolves the scryrs checkout root from ancestry markers
- **AND** file `.pi/extensions/pi-trace/index.ts` is written under that checkout root
- **AND** no nested subdirectory-local `.pi/extensions/pi-trace/` tree is created beneath the caller CWD

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

If the target file or directory already exists at the installation path, the installer SHALL exit 2 with a deterministic collision error and remediation instructions. The installer SHALL NOT overwrite, merge, or partially mutate any existing file.

#### Scenario: Hook file already exists

- **GIVEN** `.claude/hooks/scryrs-hook.mjs` already exists from a prior installation
- **WHEN** `scryrs init --agent claude-code` is invoked
- **THEN** the exit code is 2
- **AND** stderr indicates the file already exists at the target path
- **AND** stderr provides remediation instructions (e.g., remove the file and rerun)
- **AND** the existing file content is not modified

#### Scenario: Target directory already exists but target file does not

- **GIVEN** `.claude/hooks/` directory already exists but contains no `scryrs-hook.mjs`
- **WHEN** `scryrs init --agent claude-code` is invoked
- **THEN** the installer creates `scryrs-hook.mjs` inside the existing directory
- **AND** the exit code is 0

#### Scenario: Pi target directory already exists but target file does not

- **GIVEN** `.pi/extensions/pi-trace/` directory already exists but contains no `index.ts`
- **WHEN** `scryrs init --agent pi` is invoked
- **THEN** the installer creates `index.ts` inside the existing directory
- **AND** the exit code is 0

### Requirement: Claude Code settings.json collision is refused with JSON insertion instructions

If `.claude/settings.json` already exists in the target directory (regardless of whether it contains a hooks section), the Claude Code installer SHALL exit 2 with an error message that includes the exact JSON block the user must insert into the settings file and the setting's purpose. The installer SHALL NOT attempt structured merge or partial rewrite of the existing file.

#### Scenario: settings.json exists with hooks block

- **GIVEN** `.claude/settings.json` already exists and contains a `hooks` key
- **WHEN** `scryrs init --agent claude-code` is invoked
- **THEN** the exit code is 2
- **AND** stderr states that settings.json already exists
- **AND** stderr includes the exact JSON block to insert (the hook configuration object)
- **AND** the existing `.claude/settings.json` file is not modified

#### Scenario: settings.json exists without hooks block

- **GIVEN** `.claude/settings.json` exists but does not contain a `hooks` key
- **WHEN** `scryrs init --agent claude-code` is invoked
- **THEN** the exit code is 2
- **AND** stderr states that settings.json already exists
- **AND** stderr includes the exact JSON block to insert
- **AND** the existing file is not modified

#### Scenario: settings.json does not exist

- **GIVEN** `.claude/hooks/scryrs-hook.mjs` is written successfully
- **WHEN** `.claude/settings.json` does not exist
- **THEN** the installer does not attempt to create it
- **AND** the next-step text on stdout instructs the user to create `.claude/settings.json` with the required hook configuration

### Requirement: Self-install into scryrs source checkout is refused

The installer SHALL detect when the resolved invocation context is the scryrs source checkout by checking for the presence of both a `Cargo.toml` referencing `scryrs-cli` in `[workspace.members]` AND a `hooks/claude-code/` directory in the CWD ancestry. When both markers are found, the installer SHALL apply harness-specific behavior: `claude-code` SHALL still exit 2 with a deterministic error explaining the reference-only boundary for Claude consumer config, while `pi` SHALL be permitted and SHALL install into the scryrs checkout root.

#### Scenario: Claude Code self-install detected and refused

- **GIVEN** CWD is the scryrs source checkout (contains `Cargo.toml` with `scryrs-cli` in workspace members AND `hooks/claude-code/` directory)
- **WHEN** `scryrs init --agent claude-code` is invoked
- **THEN** the exit code is 2
- **AND** stderr explains that Claude consumer config must not be written into the scryrs source repo
- **AND** no files are created or modified

#### Scenario: Pi self-install at source root is allowed

- **GIVEN** CWD is the scryrs source checkout root
- **WHEN** `scryrs init --agent pi` is invoked
- **THEN** the command does not fail with self-install refusal
- **AND** installation proceeds against `.pi/extensions/pi-trace/index.ts` at repository root

#### Scenario: Pi self-install check walks parent directories and still installs at checkout root

- **GIVEN** CWD is a subdirectory of the scryrs source checkout and the parent directory contains both markers
- **WHEN** `scryrs init --agent pi` is invoked
- **THEN** the installer walks the parent directory chain
- **AND** the scryrs checkout root is detected
- **AND** installation proceeds against `.pi/extensions/pi-trace/index.ts` at that checkout root

#### Scenario: Unrelated project passes self-install check

- **GIVEN** CWD is a user project that does not contain both scryrs markers
- **WHEN** `scryrs init --agent claude-code` is invoked
- **THEN** the self-install check passes
- **AND** the installer proceeds with normal installation

### Requirement: Successful install prints deterministic next-step text

On successful installation, the installer SHALL print harness-specific next-step instructions to stdout. The text SHALL be deterministic (static strings from the harness registry) and suitable for scripted callers. The output SHALL include any reload, PATH, or follow-up config guidance specific to the harness.

#### Scenario: Claude Code next steps

- **WHEN** `scryrs init --agent claude-code` completes successfully
- **THEN** stdout includes instructions for creating `.claude/settings.json` with the hook configuration
- **AND** stdout notes that scryrs must be on PATH
- **AND** stdout mentions reload or restart of the Claude Code session

#### Scenario: Pi next steps

- **WHEN** `scryrs init --agent pi` completes successfully
- **THEN** stdout includes instructions for reloading Pi (e.g., `/reload`)
- **AND** stdout notes that scryrs must be on PATH
- **AND** stdout may mention verifying input field assumptions for `ast_grep_search` and `lsp_navigation`

#### Scenario: Next-step text is deterministic

- **WHEN** `scryrs init --agent claude-code` is invoked twice in separate temp directories
- **THEN** the stdout output is byte-identical for both invocations

### Requirement: Installer does not create or depend on scryrs.json

The installer SHALL NOT create, read, or depend on the `scryrs.json` provisional manifest file. The next-step text may mention `scryrs.json` as an optional future configuration file but SHALL NOT instruct the user to create it.

#### Scenario: No scryrs.json is created

- **WHEN** `scryrs init --agent <name>` completes successfully
- **THEN** no `scryrs.json` file is created at the project root or anywhere else

#### Scenario: Installer does not read scryrs.json

- **GIVEN** a `scryrs.json` file exists at the project root
- **WHEN** `scryrs init --agent claude-code` is invoked
- **THEN** the installer does not read or parse the file
- **AND** installation behavior is identical regardless of the file's presence

### Requirement: Source-repo Pi install remains non-canonical runtime state

When `scryrs init --agent pi` is used inside the scryrs source checkout, `hooks/pi/index.ts` SHALL remain the only canonical hook source in the repository. The installed file at `.pi/extensions/pi-trace/index.ts` SHALL be treated as runtime copy only. Repository maintainer guidance SHALL explicitly state that LLMs/agents MUST NOT edit the installed copy directly or treat it as leading source.

#### Scenario: AGENTS guidance defines canonical Pi hook source
- **WHEN** a maintainer or agent reads `AGENTS.md`
- **THEN** the file states that `hooks/pi/index.ts` is canonical source for the Pi hook
- **AND** the file states that `.pi/extensions/pi-trace/index.ts` is installed runtime copy only
- **AND** the file states that LLMs/agents MUST NOT edit the installed copy directly

#### Scenario: Installed Pi copy is excluded from normal git noise
- **WHEN** `scryrs init --agent pi` is run inside the scryrs source checkout
- **THEN** the created `.pi/extensions/pi-trace/` artifact path is ignored by repository ignore rules
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

