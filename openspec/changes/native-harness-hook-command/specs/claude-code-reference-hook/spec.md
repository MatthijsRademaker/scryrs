## REMOVED Requirements

### Requirement: Reference hook source lives under hooks/claude-code/

**Reason**: The JavaScript `.mjs` transport is deleted. Claude Code integration is now the native `scryrs hook claude-code` subcommand; there is no hook source file to host.

### Requirement: Hook forwards events to scryrs record --stdin

**Reason**: There is no separate JS process. The native subcommand persists directly through the shared `EventStore` path; it does not shell out to `scryrs record`.

## MODIFIED Requirements

### Requirement: Claude Code integration is the native `scryrs hook claude-code` subcommand

Claude Code integration SHALL be provided by the native `scryrs hook claude-code` subcommand invoked as a `command` hook, not by any JavaScript or node runtime. The hook SHALL be configured in `.claude/settings.json` as `"hooks": [{ "type": "command", "command": "scryrs hook claude-code" }]` under `PreToolUse`. The subcommand SHALL receive the PreToolUse event JSON on stdin.

#### Scenario: settings.json uses the native command

- **WHEN** Claude Code integration is installed
- **THEN** the `PreToolUse` hook command is `scryrs hook claude-code`
- **AND** no `.mjs` file and no `node` invocation are involved

#### Scenario: event arrives on stdin

- **WHEN** Claude Code is about to execute a tool
- **THEN** it pipes the PreToolUse event JSON to `scryrs hook claude-code` on stdin

### Requirement: Hook maps Claude Code tools to canonical TraceEvent families

The mapping SHALL be performed in the `scryrs-adapter-harness` crate (not in JavaScript). Tool names SHALL be matched as documented PascalCase. The mapping SHALL be: Read→FileOpened, Grep→SearchRun, Glob→SearchRun, Edit→EditMade, Write→EditMade, NotebookEdit→EditMade, WebSearch→SearchRun, WebFetch→DocRetrieved; Bash→CommandExecuted only when `SCRYRS_DEBUG` is non-empty. Each event SHALL carry `tool_name` set to the original Claude Code tool name.

#### Scenario: PascalCase names match without lowercasing

- **WHEN** the hook processes a `"WebSearch"` PreToolUse event
- **THEN** it emits a `SearchRun` event with `tool_name` `"WebSearch"`

### Requirement: PreToolUse events carry unconditional Success outcome

Because PreToolUse fires before execution, every persisted event SHALL carry `outcome = Success`. (Unchanged in intent; now enforced by the `claude-code` adapter.)

#### Scenario: outcome is Success

- **WHEN** the hook persists any Claude Code event
- **THEN** its `outcome` is `Success`

### Requirement: Hook fails open when scryrs is unavailable

The integration SHALL never block Claude Code. Exit 0 with empty stdout is the allow signal. Any internal error SHALL append to `.scryrs/hooks/claude-code-warnings.log` and still exit 0. If the `scryrs` binary is entirely absent, Claude Code's own missing-command handling SHALL allow the tool to proceed.

#### Scenario: internal error still allows the tool

- **GIVEN** a valid PreToolUse event but a failing store
- **WHEN** `scryrs hook claude-code` runs
- **THEN** it exits 0 with empty stdout
- **AND** a warning is logged

### Requirement: Session IDs come from the PreToolUse payload

The integration SHALL read `session_id` from the PreToolUse payload rather than generating a per-process UUID or reading `CLAUDE_SESSION_ID`-style environment variables.

#### Scenario: payload session_id is used

- **WHEN** the hook persists an event for a payload whose `session_id` is `"abc123"`
- **THEN** the persisted event's `session_id` is `"abc123"`
