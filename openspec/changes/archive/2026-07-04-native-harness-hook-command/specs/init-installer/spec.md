## REMOVED Requirements

### Requirement: Claude Code settings.json collision is refused with JSON insertion instructions

**Reason**: Replaced by create-or-merge. The installer now writes/merges the native hook block into `.claude/settings.json` instead of refusing on collision and printing hand-edit instructions.

## MODIFIED Requirements

### Requirement: Hook source artifacts are embedded at compile time

The installer SHALL embed the Pi hook source (`hooks/pi/index.ts`) at compile time via `include_str!()`. It SHALL NOT embed any Claude Code hook source, because Claude Code integration is the native `scryrs hook claude-code` command and ships no hook file.

#### Scenario: Pi hook source is embedded

- **GIVEN** `hooks/pi/index.ts` exists
- **WHEN** `crates/scryrs-cli` is compiled
- **THEN** its contents are embedded via `include_str!("../../../hooks/pi/index.ts")`

#### Scenario: No Claude Code hook source is embedded

- **WHEN** inspecting `init.rs`
- **THEN** there is no `include_str!` for any `hooks/claude-code/` file
- **AND** `hooks/claude-code/scryrs-hook.mjs` does not exist in the repository

### Requirement: Claude Code install creates or merges `.claude/settings.json`

`scryrs init --agent claude-code` SHALL create `.claude/settings.json` if absent, or merge into it if present, adding `"hooks": { "PreToolUse": [{ "matcher": "", "hooks": [{ "type": "command", "command": "scryrs hook claude-code" }] }] }`. The merge SHALL preserve unrelated existing keys and SHALL be idempotent (re-running does not duplicate the hook). The installer SHALL NOT write any `.claude/hooks/*.mjs` file.

#### Scenario: settings.json created when absent

- **GIVEN** no `.claude/settings.json` exists
- **WHEN** `scryrs init --agent claude-code` runs
- **THEN** `.claude/settings.json` is created with the native `PreToolUse` hook block
- **AND** no `.mjs` file is written

#### Scenario: existing settings.json is merged, not clobbered

- **GIVEN** `.claude/settings.json` exists with unrelated keys (and possibly other hooks)
- **WHEN** `scryrs init --agent claude-code` runs
- **THEN** the native hook block is added under `PreToolUse`
- **AND** unrelated keys and existing hooks are preserved

#### Scenario: re-run is idempotent

- **WHEN** `scryrs init --agent claude-code` runs twice
- **THEN** the `scryrs hook claude-code` command appears exactly once in `PreToolUse`

### Requirement: Install eagerly scaffolds the `.scryrs/` runtime store

Before installing the harness hook, `scryrs init` SHALL scaffold the `.scryrs/` runtime directory relative to the resolved target base: a schema-initialized `.scryrs/scryrs.db` and a `.scryrs/.gitignore` excluding runtime trace data from version control. Scaffolding SHALL be idempotent — an existing store SHALL be opened (never clobbered) and an existing `.gitignore` SHALL be preserved. Scaffolding SHALL run only after the harness name is validated, so an unsupported harness leaves the filesystem untouched. An I/O failure during scaffolding SHALL exit 1.

#### Scenario: claude-code install scaffolds the store

- **WHEN** `scryrs init --agent claude-code` succeeds
- **THEN** `.scryrs/scryrs.db` exists and is a schema-initialized scryrs datastore with zero events
- **AND** `.scryrs/.gitignore` exists and excludes runtime trace data
- **AND** stdout notes the `.scryrs/` store was initialized

#### Scenario: pi install scaffolds the store

- **WHEN** `scryrs init --agent pi` succeeds
- **THEN** `.scryrs/scryrs.db` and `.scryrs/.gitignore` exist relative to the resolved target base

#### Scenario: unsupported harness does not scaffold

- **WHEN** `scryrs init --agent <unsupported>` runs
- **THEN** the command exits 2
- **AND** no `.scryrs/` directory is created

#### Scenario: scaffolding is idempotent

- **WHEN** `scryrs init` runs twice in the same directory
- **THEN** the existing `.scryrs/scryrs.db` is preserved (not clobbered)
- **AND** the existing `.scryrs/.gitignore` is preserved

### Requirement: Successful install prints deterministic next-step text

On success the installer SHALL print deterministic next-step text. For Claude Code it SHALL note that `scryrs` must be on PATH and that the session must be restarted; it SHALL NOT reference any `.mjs` file or hand-edit of settings.json.

#### Scenario: Claude Code next steps reference the native command

- **WHEN** `scryrs init --agent claude-code` succeeds
- **THEN** stdout states that `.claude/settings.json` was created/updated with `scryrs hook claude-code`
- **AND** notes scryrs must be on PATH and the session restarted
- **AND** makes no mention of a `.mjs` file
