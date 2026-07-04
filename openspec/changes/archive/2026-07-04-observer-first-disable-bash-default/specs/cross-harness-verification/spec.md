## MODIFIED Requirements

### Requirement: Claude Code fixture proves observer-first default capture against real scryrs

The Claude Code verification fixture SHALL pipe hook-generated events from the default observer-first tool set to the real `scryrs record --stdin` binary and SHALL assert that events are persisted to `.scryrs/scryrs.db` through the canonical SQLite datastore contract. Default coverage SHALL include `read`, `grep`, `glob`, `edit`, `write`, `notebookedit`, `web_search`, and `web_fetch`. Default coverage SHALL exclude `bash`.

#### Scenario: Default observer-first tools produce accepted events

- **GIVEN** the real `scryrs` binary is on PATH
- **WHEN** the fixture invokes the Claude Code hook for `read`, `grep`, `glob`, `edit`, `write`, `notebookedit`, `web_search`, and `web_fetch` while `SCRYRS_DEBUG` is unset
- **THEN** `scryrs record --stdin` emits a deterministic JSON summary with `accepted` count equal to the number of those non-empty event lines
- **AND** `.scryrs/scryrs.db` contains the same number of persisted events

#### Scenario: Default mode does not persist Bash

- **GIVEN** the real `scryrs` binary is on PATH
- **WHEN** the fixture invokes the Claude Code Bash hook path while `SCRYRS_DEBUG` is unset
- **THEN** no corresponding `CommandExecuted` row is written to `.scryrs/scryrs.db`

### Requirement: Pi fixture proves observer-first default capture and SessionStart lifecycle

The Pi verification fixture SHALL load `hooks/pi/index.ts` via `tsx` against a fake `ExtensionAPI`, emit `session_start` and representative `tool_result` events for the default observer-first Pi tools, and SHALL assert that events are persisted through the canonical SQLite datastore contract. Default coverage SHALL include `read`, `ast_grep_search`, `edit`, `write`, and successful or failing `lsp_navigation`. Default coverage SHALL exclude `bash`.

#### Scenario: Default observer-first Pi tools produce correct events

- **GIVEN** the fake `ExtensionAPI` fires `tool_result` events for `read`, `ast_grep_search`, `edit`, `write`, and successful `lsp_navigation` while `SCRYRS_DEBUG` is unset
- **WHEN** the hook handler processes each event
- **THEN** `.scryrs/scryrs.db` contains one persisted event per tracked tool with the correct event type and payload mapping
- **AND** no persisted event is required for `bash`

#### Scenario: Default mode does not persist Pi Bash

- **GIVEN** the fake `ExtensionAPI` fires a `tool_result` event for `bash` while `SCRYRS_DEBUG` is unset
- **WHEN** the hook handler processes the event
- **THEN** no corresponding `CommandExecuted` row is written to `.scryrs/scryrs.db`
- **AND** the handler returns `undefined`

## ADDED Requirements

### Requirement: Bash verification is debug-gated in both harness fixtures

Cross-harness verification SHALL prove that Bash trace capture remains available when `SCRYRS_DEBUG` is set to a non-empty value for both supported harnesses.

#### Scenario: Claude Code persists Bash in debug mode

- **GIVEN** the real `scryrs` binary is on PATH
- **WHEN** the Claude Code fixture invokes the Bash hook path with `tool_input.command` set to `rtk ls -la` and `SCRYRS_DEBUG` is set
- **THEN** `.scryrs/scryrs.db` contains a persisted `CommandExecuted` event whose `payload.command` is `rtk ls -la`

#### Scenario: Pi persists Bash in debug mode

- **GIVEN** the fake `ExtensionAPI` fires a `tool_result` event for `bash`
- **WHEN** the event input command is `rtk ls -la` and `SCRYRS_DEBUG` is set
- **THEN** `.scryrs/scryrs.db` contains a persisted `CommandExecuted` event whose `payload.command` is `rtk ls -la`
- **AND** the handler returns `undefined`
