# cross-harness-verification Specification

## ADDED Requirements

### Requirement: Claude Code verification covers rewritten Bash inputs without RTK installed

The Claude Code verification fixture SHALL simulate rewrite-tool compatibility by feeding RTK-style Bash command strings directly into the hook. The fixture SHALL cover both a simple RTK-prefixed command and a compound command with rewritten subcommands, and SHALL prove that scryrs persists the observed command string without stdout or stderr side effects.

#### Scenario: Simple rewritten Bash command is accepted
- **GIVEN** the real `scryrs` binary is on PATH
- **WHEN** the Claude Code fixture invokes the Bash hook path with `tool_input.command` set to `rtk ls -la`
- **THEN** `.scryrs/events.jsonl` contains a `CommandExecuted` event whose `payload.command` is `rtk ls -la`
- **AND** the hook subprocess writes zero bytes to stdout
- **AND** the hook subprocess writes zero bytes to stderr

#### Scenario: Compound rewritten Bash command is accepted
- **GIVEN** the real `scryrs` binary is on PATH
- **WHEN** the Claude Code fixture invokes the Bash hook path with `tool_input.command` set to `echo "=== BACKEND ===" && rtk ls backend/api/ && rtk ls backend/cmd/`
- **THEN** `.scryrs/events.jsonl` contains a `CommandExecuted` event whose `payload.command` is the full compound command string
- **AND** the fixture does not require RTK to be installed

### Requirement: Pi verification simulates upstream rewrite-tool output

The Pi verification fixture SHALL simulate rewrite-tool compatibility by emitting `tool_result` events whose Bash `event.input.command` already contains RTK-style rewritten command strings. The fixture SHALL cover both a simple RTK-prefixed command and a compound command with rewritten subcommands, and SHALL prove that scryrs remains non-interfering while persisting the observed command string.

#### Scenario: Simple rewritten Pi Bash command is accepted
- **GIVEN** the fake `ExtensionAPI` fires a `tool_result` event for `bash`
- **WHEN** the event input command is `rtk ls -la`
- **THEN** `.scryrs/events.jsonl` contains a `CommandExecuted` event whose `payload.command` is `rtk ls -la`
- **AND** the handler returns `undefined`
- **AND** the original `ToolResultEvent` input remains unchanged

#### Scenario: Compound rewritten Pi Bash command is accepted
- **GIVEN** the fake `ExtensionAPI` fires a `tool_result` event for `bash`
- **WHEN** the event input command is `echo "=== BACKEND ===" && rtk ls backend/api/ && rtk ls backend/cmd/`
- **THEN** `.scryrs/events.jsonl` contains a `CommandExecuted` event whose `payload.command` is the full compound command string
- **AND** the fixture does not require RTK to be installed
