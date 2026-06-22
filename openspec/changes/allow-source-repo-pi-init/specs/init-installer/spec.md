## ADDED Requirements

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

## MODIFIED Requirements

### Requirement: Installer writes to project-local consumer directories only

The installer SHALL write hook files to consumer-side project-local directories under a resolved target root. For ordinary consumer projects, resolved target root is the current working directory. For `scryrs init --agent pi` invoked inside the scryrs source checkout, resolved target root is the detected scryrs checkout root. For Claude Code, the target remains `.claude/hooks/scryrs-hook.mjs`. For Pi, the target is `.pi/extensions/pi-trace/index.ts`. The installer SHALL NOT write to user-global directories (e.g., `~/.pi/agent/extensions/`) in v1.

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
