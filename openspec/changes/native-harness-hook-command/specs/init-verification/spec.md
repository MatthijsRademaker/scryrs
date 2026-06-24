## REMOVED Requirements

### Requirement: Init installer contract remains non-mutating for settings.json

**Reason**: Directly reversed by this change. The installer now creates-or-merges `.claude/settings.json`. A requirement asserting it must never auto-create or modify settings.json cannot coexist with the new create/merge contract.

## MODIFIED Requirements

### Requirement: Installed-hook e2e validates consumer-installed artifacts

The installed-hook e2e fixture SHALL run `scryrs init --agent claude-code` and `scryrs init --agent pi` in temporary consumer project directories and prove at least one event is persisted in `.scryrs/scryrs.db`. For Claude Code, the fixture SHALL drive the native `scryrs hook claude-code` subcommand with a PreToolUse payload on stdin (no `.mjs`, no `node` load). For Pi, the fixture SHALL load the installed slimmed `index.ts` and prove it delegates to `scryrs hook pi`.

#### Scenario: Claude Code installed integration is functional

- **GIVEN** a temporary consumer project directory
- **WHEN** `scryrs init --agent claude-code` is executed in that directory
- **THEN** `.claude/settings.json` contains a `PreToolUse` hook whose command is `scryrs hook claude-code`
- **AND** no `.claude/hooks/*.mjs` file is created
- **AND** piping a valid PreToolUse payload to `scryrs hook claude-code` persists an event
- **AND** persistence is confirmed via `scryrs hotspots .` showing `analyzedEventCount >= 1`

#### Scenario: Pi installed hook delegates to the native command

- **GIVEN** a temporary consumer project directory
- **WHEN** `scryrs init --agent pi` is executed in that directory
- **THEN** the installed artifact exists at `.pi/extensions/pi-trace/index.ts`
- **AND** the artifact can be loaded (TypeScript transpiled via tsx) without errors
- **AND** when exercised with a simulated `tool_result` event, it invokes `scryrs hook pi --file <tmp>`
- **AND** persistence is confirmed via `scryrs hotspots .` showing `analyzedEventCount >= 1`

#### Scenario: Installed integration does not depend on repository source

- **GIVEN** the installed-hook e2e fixture is executing
- **WHEN** it exercises the integration
- **THEN** it uses the consumer-installed `.claude/settings.json` / `.pi/extensions/pi-trace/index.ts` and the `scryrs` binary
- **AND** it does NOT load anything from `hooks/claude-code/` or `hooks/pi/` in the repository source tree

### Requirement: Installed-hook e2e validates deterministic next-step text

The installed-hook e2e fixture SHALL assert that stdout from a successful `scryrs init` includes the deterministic next-step text. For Claude Code, the text SHALL state that `.claude/settings.json` was created or updated with `scryrs hook claude-code` and that the session must be restarted; it SHALL NOT instruct manual settings.json creation and SHALL NOT mention a `.mjs` file. For Pi, the text SHALL instruct the user to reload Pi.

#### Scenario: Claude Code next-step text references the native command

- **WHEN** `scryrs init --agent claude-code` completes successfully
- **THEN** stdout states `.claude/settings.json` was created/updated with `scryrs hook claude-code`
- **AND** stdout includes restart instructions
- **AND** stdout makes no mention of a `.mjs` file or manual settings.json editing

#### Scenario: Next-step text is deterministic across invocations

- **WHEN** `scryrs init --agent claude-code` is invoked twice in separate temporary directories
- **THEN** the stdout output is byte-identical for both invocations

### Requirement: Claude Code settings.json schema is consistent across all sources

The canonical `.claude/settings.json` hook configuration SHALL be the native command-block form `"hooks": [{ "type": "command", "command": "scryrs hook claude-code" }]` under `PreToolUse`, and SHALL be identical across the installer's written/merged output, the installer next-step text, and the hook README. The flat `"hook": "<string>"` form SHALL NOT appear in any source.

#### Scenario: Installer output matches README schema

- **WHEN** comparing the block the installer writes to `.claude/settings.json`, the installer next-step text, and `hooks/claude-code/README.md`
- **THEN** all three describe the same `"type":"command"` command-block structure
- **AND** none uses the flat `"hook": "<string>"` form
