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

The Claude Code verification fixture SHALL pipe hook-generated events from all nine whitelisted tools to the real `scryrs record --stdin` binary and SHALL assert that events are persisted to `.scryrs/events.jsonl` with canonical `TraceEvent` envelope shape.

#### Scenario: All nine tools produce accepted events
- **GIVEN** the real `scryrs` binary is on PATH
- **WHEN** the fixture invokes the Claude Code hook for each of the nine whitelisted tools (read, bash, grep, glob, edit, write, notebookedit, web_search, web_fetch)
- **THEN** `scryrs record --stdin` emits a deterministic JSON summary with `accepted` count equal to the number of non-empty event lines
- **AND** `.scryrs/events.jsonl` contains the same number of persisted events
- **AND** each persisted event carries `schema_version: "0.1.0"`, `timestamp` (RFC 3339), `session_id` (non-empty string), `event_type` (matching the tool-to-event mapping), `tool_name` (original Claude Code tool name), `payload` (self-describing JSON with `type` tag), and `outcome: { result: "Success" }`

#### Scenario: Claude Code hook produces no stdout or stderr
- **WHEN** the fixture invokes the hook for any supported tool
- **THEN** the hook subprocess writes zero bytes to stdout
- **AND** the hook subprocess writes zero bytes to stderr

#### Scenario: Unlisted tools produce no events
- **WHEN** the fixture invokes the hook for a tool not in the whitelist (e.g., Task)
- **THEN** the hook returns `{continue: true}`
- **AND** no event is written to `.scryrs/events.jsonl` for that tool

### Requirement: Pi fixture proves success capture and SessionStart lifecycle

The Pi verification fixture SHALL load `hooks/pi/index.ts` via `tsx` against a fake `ExtensionAPI`, emit `session_start` and representative `tool_result` events for all six tracked Pi tools, and SHALL assert that events are persisted with canonical shape.

#### Scenario: SessionStart is emitted and persisted
- **GIVEN** the fake `ExtensionAPI` fires the `session_start` event
- **WHEN** the hook handler processes the event
- **THEN** `.scryrs/events.jsonl` contains a `SessionStart` event
- **AND** the event carries `event_type: "SessionStart"`, `payload.type: "SessionStart"`, and `outcome.result: "Success"`
- **AND** the event carries a `session_id` (UUID v4 string)

#### Scenario: All six tracked tools produce correct events
- **GIVEN** the fake `ExtensionAPI` fires `tool_result` events for `read`, `bash`, `ast_grep_search`, `edit`, `write`, and `lsp_navigation` (success)
- **WHEN** the hook handler processes each event
- **THEN** `.scryrs/events.jsonl` contains one event per tracked tool with the correct event type and payload per the canonical mapping:
  - `read` → `FileOpened` (payload.path)
  - `bash` → `CommandExecuted` (payload.command)
  - `ast_grep_search` → `SearchRun` (payload.query)
  - `edit` → `EditMade` (payload.target)
  - `write` → `EditMade` (payload.target)
  - `lsp_navigation` (success) → `SymbolInspected` (payload.name)
- **AND** each event carries `tool_name` set to the Pi tool name
- **AND** all tool events share the same `session_id` as the `SessionStart` event

#### Scenario: Pi handler returns undefined for all events
- **WHEN** the hook handler processes any tracked tool event
- **THEN** the handler return value is `undefined`
- **AND** the original `ToolResultEvent` input (content, details, isError) is unchanged after the handler completes

#### Scenario: Unlisted Pi tools are silently ignored
- **WHEN** the fake `ExtensionAPI` fires a `tool_result` event for a tool not in the tracked set (e.g., `grep`, `web_search`)
- **THEN** no event is written to `.scryrs/events.jsonl` for that tool
- **AND** the handler returns `undefined`

### Requirement: Pi fixture proves failure propagation

The Pi verification fixture SHALL prove that a failing `lsp_navigation` tool result produces a `FailedLookup` event with failure outcome while the original error-state event input is preserved unchanged.

#### Scenario: Failing lsp_navigation records FailedLookup with failure outcome
- **GIVEN** the fake `ExtensionAPI` fires a `tool_result` event for `lsp_navigation` with `isError: true` and `input.symbol: "nonexistent_fn"`
- **WHEN** the hook handler processes the event
- **THEN** `.scryrs/events.jsonl` contains a `FailedLookup` event
- **AND** the event carries `event_type: "FailedLookup"`, `tool_name: "lsp_navigation"`, `payload.type: "FailedLookup"`, and `payload.subject: "nonexistent_fn"`
- **AND** the event carries `outcome.result: "Failure"`

#### Scenario: Original error payload is unchanged
- **GIVEN** the original `ToolResultEvent` input is snapshotted before passing to the hook handler
- **WHEN** the handler completes for a failing `lsp_navigation`
- **THEN** the post-handler `ToolResultEvent` input is deep-equal to the pre-handler snapshot
- **AND** the handler returns `undefined`

#### Scenario: Failure outcome reason is not strictly asserted
- **WHEN** the fixture asserts a `FailedLookup` event
- **THEN** it verifies `outcome.result === "Failure"`
- **AND** it does NOT require the `outcome.reason` string to match any specific value (e.g., `"Tool execution error"`)

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

The verification entrypoint and fixtures SHALL be read-only consumers of hook source files. They SHALL NOT modify any file under `hooks/`, `crates/`, or `openspec/specs/`.

#### Scenario: Hook sources are imported read-only
- **WHEN** either fixture loads a hook module
- **THEN** the hook source file is imported or executed without modification
- **AND** no hook source file is written, patched, or transpiled to disk

#### Scenario: No Rust crate or CLI changes
- **WHEN** this change is implemented
- **THEN** no files in `crates/` are modified
- **AND** the `scryrs` binary behavior is unchanged

#### Scenario: No existing OpenSpec specs are modified
- **WHEN** this change is implemented
- **THEN** `openspec/specs/scryrs-record-endpoint/spec.md` is unchanged
- **AND** `openspec/specs/claude-code-reference-hook/spec.md` is unchanged
- **AND** `openspec/specs/pi-reference-hook/spec.md` is unchanged
- **AND** `openspec/specs/trace-hook-contract/spec.md` is unchanged

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

