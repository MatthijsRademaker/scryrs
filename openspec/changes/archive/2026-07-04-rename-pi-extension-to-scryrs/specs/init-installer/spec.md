## MODIFIED Requirements

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
