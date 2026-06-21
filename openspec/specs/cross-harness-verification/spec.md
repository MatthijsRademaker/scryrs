# cross-harness-verification Specification

## Purpose
TBD - created by archiving change task-0cb48e7a-ad81-4ad4-a451-7bb21ef6a750. Update Purpose after archive.
## Requirements
### Requirement: Docker-backed verification entrypoint exercises both harnesses

The system SHALL provide a `scripts/verify-trace-capture` bash entrypoint that builds the real `scryrs` binary in a Rust Docker container and drives both the Claude Code and Pi hook fixtures against it in a Node.js Docker container. The entrypoint SHALL NOT require host-installed Node.js and SHALL be runnable in the worker environment.

#### Scenario: Entrypoint runs end-to-end successfully
- **WHEN** `scripts/verify-trace-capture` is invoked in the repository root
- **THEN** the scryrs binary is built via `cargo build --release` in a Rust container
- **AND** the Claude Code fixture is executed in a Node container, piping hook events to the real scryrs binary
- **AND** the Pi fixture is executed in the Node container, loading `hooks/pi/index.ts` via `tsx` against a fake `ExtensionAPI`
- **AND** the entrypoint exits 0 when all assertions pass

#### Scenario: Entrypoint exits non-zero on failure
- **WHEN** any assertion in either fixture fails
- **THEN** the entrypoint prints the failure details to stderr
- **AND** the entrypoint exits with a non-zero code

#### Scenario: Targeted fixture execution is supported
- **WHEN** `scripts/verify-trace-capture --claude-only` is invoked
- **THEN** only the Claude Code fixture runs
- **WHEN** `scripts/verify-trace-capture --pi-only` is invoked
- **THEN** only the Pi fixture runs

### Requirement: Claude Code fixture proves success capture against real scryrs

The Claude Code verification fixture SHALL pipe hook-generated events from all nine whitelisted tools to the real `scryrs record --stdin` binary and SHALL assert that events are persisted to `.scryrs/scryrs.db` through the canonical SQLite datastore contract.

#### Scenario: All nine tools produce accepted events
- **GIVEN** the real `scryrs` binary is on PATH
- **WHEN** the fixture invokes the Claude Code hook for each of the nine whitelisted tools: `read`, `bash`, `grep`, `glob`, `edit`, `write`, `notebookedit`, `web_search`, and `web_fetch`
- **THEN** `scryrs record --stdin` emits a deterministic JSON summary with `accepted` count equal to the number of non-empty event lines
- **AND** `.scryrs/scryrs.db` contains the same number of persisted events
- **AND** the stored event JSON preserves the canonical `TraceEvent` envelope with `schema_version`, `timestamp`, `session_id`, `event_type`, `tool_name`, `payload`, and `outcome`

#### Scenario: Claude Code hook produces no stdout or stderr
- **WHEN** the fixture invokes the hook for any supported tool
- **THEN** the hook subprocess writes zero bytes to stdout
- **AND** the hook subprocess writes zero bytes to stderr

#### Scenario: Unlisted tools produce no events
- **WHEN** the fixture invokes the hook for a tool not in the whitelist, such as `Task`
- **THEN** the hook returns `{continue: true}`
- **AND** no corresponding event row is written to `.scryrs/scryrs.db`

### Requirement: Pi fixture proves success capture and SessionStart lifecycle

The Pi verification fixture SHALL load `hooks/pi/index.ts` via `tsx` against a fake `ExtensionAPI`, emit `session_start` and representative `tool_result` events for all six tracked Pi tools, and SHALL assert that events are persisted through the canonical SQLite datastore contract.

#### Scenario: SessionStart is emitted and persisted
- **GIVEN** the fake `ExtensionAPI` fires the `session_start` event
- **WHEN** the hook handler processes the event
- **THEN** `.scryrs/scryrs.db` contains a `SessionStart` persisted event
- **AND** the stored event JSON carries `event_type: SessionStart`, `payload.type: SessionStart`, and success outcome
- **AND** the event carries a non-empty `session_id`

#### Scenario: All six tracked tools produce correct events
- **GIVEN** the fake `ExtensionAPI` fires `tool_result` events for `read`, `bash`, `ast_grep_search`, `edit`, `write`, and successful `lsp_navigation`
- **WHEN** the hook handler processes each event
- **THEN** `.scryrs/scryrs.db` contains one persisted event per tracked tool with the correct event type and payload mapping:
  - `read` to `FileOpened`
  - `bash` to `CommandExecuted`
  - `ast_grep_search` to `SearchRun`
  - `edit` to `EditMade`
  - `write` to `EditMade`
  - successful `lsp_navigation` to `SymbolInspected`
- **AND** each stored event carries `tool_name` set to the Pi tool name
- **AND** all tool events share the same `session_id` as the `SessionStart` event

#### Scenario: Pi handler returns undefined for all events
- **WHEN** the hook handler processes any tracked tool event
- **THEN** the handler return value is `undefined`
- **AND** the original `ToolResultEvent` input is unchanged after the handler completes

#### Scenario: Unlisted Pi tools are silently ignored
- **WHEN** the fake `ExtensionAPI` fires a `tool_result` event for a tool not in the tracked set, such as `grep` or `web_search`
- **THEN** no corresponding event row is written to `.scryrs/scryrs.db`
- **AND** the handler returns `undefined`

### Requirement: Pi fixture proves failure propagation

The Pi verification fixture SHALL prove that a failing `lsp_navigation` tool result produces a `FailedLookup` event with failure outcome while the original error-state event input is preserved unchanged.

#### Scenario: Failing lsp_navigation records FailedLookup with failure outcome
- **GIVEN** the fake `ExtensionAPI` fires a `tool_result` event for `lsp_navigation` with `isError: true` and `input.symbol: nonexistent_fn`
- **WHEN** the hook handler processes the event
- **THEN** `.scryrs/scryrs.db` contains a persisted `FailedLookup` event
- **AND** the stored event carries `event_type: FailedLookup`, `tool_name: lsp_navigation`, `payload.type: FailedLookup`, and `payload.subject: nonexistent_fn`
- **AND** the stored event carries failure outcome

#### Scenario: Original error payload is unchanged
- **GIVEN** the original `ToolResultEvent` input is snapshotted before passing to the hook handler
- **WHEN** the handler completes for a failing `lsp_navigation`
- **THEN** the post-handler `ToolResultEvent` input is deep-equal to the pre-handler snapshot
- **AND** the handler returns `undefined`

#### Scenario: Failure outcome reason is not strictly asserted
- **WHEN** the fixture asserts a persisted `FailedLookup` event
- **THEN** it verifies failure outcome
- **AND** it does not require any specific `outcome.reason` string

### Requirement: Both fixtures prove fail-open behavior

The verification SHALL prove that when `scryrs` is not on PATH or cannot execute, both hooks continue normally and do not corrupt tool output.

#### Scenario: Claude Code fail-open when scryrs missing
- **GIVEN** `scryrs` is not on PATH in the fixture's test environment
- **WHEN** the Claude Code hook processes a tracked tool event
- **THEN** the hook returns `{continue: true}`
- **AND** the hook writes zero bytes to stdout
- **AND** the hook writes zero bytes to stderr
- **AND** the warning log `.scryrs/hooks/claude-code-warnings.log` is created with a timestamped entry

#### Scenario: Pi fail-open when scryrs missing
- **GIVEN** `scryrs` is not on PATH in the fixture's test environment
- **WHEN** the Pi hook processes any tracked tool event
- **THEN** the handler returns `undefined`
- **AND** the handler does not throw
- **AND** `console.error` is called with a descriptive scryrs-failure message

### Requirement: run_node Docker helper follows existing run_rust pattern

The `run_node` function in `scripts/lib/docker-verification.sh` SHALL follow the same pattern as `run_rust`: mount the repository root at `/workspace`, map the caller's UID/GID for correct file ownership, use `NODE_IMAGE` from `scripts/.versions` (default `node:22-alpine`), and pull the image on first use.

#### Scenario: run_node executes Node.js commands
- **WHEN** `run_node node --version` is invoked
- **THEN** the command runs inside a `node:22-alpine` container with the repository mounted at `/workspace`
- **AND** the output is the Node.js version string
- **AND** the exit code matches the child process exit code

#### Scenario: run_node uses pinned image from .versions
- **WHEN** `run_node` is invoked
- **THEN** it sources or reads the `NODE_IMAGE` value from `scripts/.versions`
- **AND** it pulls the image if not already present locally

### Requirement: Verification does not modify hook sources, CLI behavior, or OpenSpec specs

The verification entrypoint and fixtures SHALL remain read-only consumers of hook source files and SHALL align their persistence assertions with the current canonical datastore contract instead of freezing prior JSONL-only behavior.

#### Scenario: Hook sources are imported read-only
- **WHEN** either fixture loads a hook module
- **THEN** the hook source file is imported or executed without modification
- **AND** no hook source file is written, patched, or transpiled to disk

#### Scenario: Persistence assertions follow the canonical datastore
- **WHEN** either fixture verifies persisted events
- **THEN** it inspects `.scryrs/scryrs.db` as the canonical local store
- **AND** it does not require `.scryrs/events.jsonl` to exist as the canonical persisted artifact

### Requirement: Claude Code verification covers rewritten Bash inputs without RTK installed

The Claude Code verification fixture SHALL simulate rewrite-tool compatibility by feeding RTK-style Bash command strings directly into the hook. The fixture SHALL cover both a simple RTK-prefixed command and a compound command with rewritten subcommands, and SHALL prove that scryrs persists the observed command string through the canonical SQLite datastore without stdout or stderr side effects.

#### Scenario: Simple rewritten Bash command is accepted
- **GIVEN** the real `scryrs` binary is on PATH
- **WHEN** the Claude Code fixture invokes the Bash hook path with `tool_input.command` set to `rtk ls -la`
- **THEN** `.scryrs/scryrs.db` contains a persisted `CommandExecuted` event whose `payload.command` is `rtk ls -la`
- **AND** the hook subprocess writes zero bytes to stdout
- **AND** the hook subprocess writes zero bytes to stderr

#### Scenario: Compound rewritten Bash command is accepted
- **GIVEN** the real `scryrs` binary is on PATH
- **WHEN** the Claude Code fixture invokes the Bash hook path with `tool_input.command` set to `echo "=== BACKEND ===" && rtk ls backend/api/ && rtk ls backend/cmd/`
- **THEN** `.scryrs/scryrs.db` contains a persisted `CommandExecuted` event whose `payload.command` is the full compound command string
- **AND** the fixture does not require RTK to be installed

### Requirement: Pi verification simulates upstream rewrite-tool output

The Pi verification fixture SHALL simulate rewrite-tool compatibility by emitting `tool_result` events whose Bash `event.input.command` already contains RTK-style rewritten command strings. The fixture SHALL cover both a simple RTK-prefixed command and a compound command with rewritten subcommands, and SHALL prove that scryrs remains non-interfering while persisting the observed command string through the canonical SQLite datastore.

#### Scenario: Simple rewritten Pi Bash command is accepted
- **GIVEN** the fake `ExtensionAPI` fires a `tool_result` event for `bash`
- **WHEN** the event input command is `rtk ls -la`
- **THEN** `.scryrs/scryrs.db` contains a persisted `CommandExecuted` event whose `payload.command` is `rtk ls -la`
- **AND** the handler returns `undefined`
- **AND** the original `ToolResultEvent` input remains unchanged

#### Scenario: Compound rewritten Pi Bash command is accepted
- **GIVEN** the fake `ExtensionAPI` fires a `tool_result` event for `bash`
- **WHEN** the event input command is `echo "=== BACKEND ===" && rtk ls backend/api/ && rtk ls backend/cmd/`
- **THEN** `.scryrs/scryrs.db` contains a persisted `CommandExecuted` event whose `payload.command` is the full compound command string
- **AND** the fixture does not require RTK to be installed

