# pi-reference-hook Specification

## Purpose
TBD - created by archiving change task-e6fdee54-7420-49cf-a72c-d3101433411a. Update Purpose after archive.
## Requirements
### Requirement: Hook is a transport-only Pi extension in hooks/pi/
The reference hook SHALL live under `hooks/pi/` as a TypeScript Pi extension file and companion documentation. The hook SHALL NOT include consumer-specific `.pi/extensions/` wiring committed to this repository.

#### Scenario: Hook source is discoverable
- **GIVEN** a Pi integrator wants to add scryrs trace capture
- **WHEN** they inspect the repository
- **THEN** they find `hooks/pi/index.ts` with a complete reference hook implementation
- **AND** they find `hooks/pi/README.md` with install instructions and tool-to-event mapping documentation

#### Scenario: No consumer config is committed
- **WHEN** the change is implemented
- **THEN** no `.pi/extensions/` files are modified or created that wire the hook into Pi auto-discovery
- **AND** no `scryrs.json` manifest is created at the repository root

### Requirement: Hook listens on tool_result post-execution only

The hook SHALL subscribe exclusively to the Pi `tool_result` event. The hook SHALL NOT subscribe to `tool_call` or any pre-execution event. The handler SHALL return `undefined` (no return value) so Pi passes the original tool result through unchanged.

#### Scenario: Hook intercepts tool_result, not tool_call
- **GIVEN** the Pi reference hook is installed
- **WHEN** an agent invokes a Pi tool
- **THEN** the hook's `tool_result` handler fires after the tool completes
- **AND** the hook never blocks, modifies, or replaces the tool call itself

#### Scenario: Hook does not modify tool results
- **WHEN** the hook handler runs and completes
- **THEN** the original tool `content`, `details`, `isError`, stdout, stderr, and exit status are preserved unmodified
- **AND** the agent observes exactly the same tool output as if the hook were not installed

### Requirement: Hook filters to the six named Pi tools

The hook SHALL forward trace events only for Pi tool names `read`, `bash`, `ast_grep_search`, `lsp_navigation`, `edit`, and `write`. Calls to any other tool SHALL be silently ignored by the hook.

#### Scenario: Named tool is forwarded
- **WHEN** an agent calls `read`, `bash`, `ast_grep_search`, `lsp_navigation`, `edit`, or `write`
- **THEN** the hook constructs a TraceEvent and forwards it to `scryrs record`

#### Scenario: Unnamed tool is ignored
- **WHEN** an agent calls a tool not in the filter set (e.g., `grep`, `web_search`, a custom tool)
- **THEN** the hook handler returns without constructing a TraceEvent or invoking scryrs

### Requirement: Tool events map to canonical TraceEvent families

The hook SHALL map each supported Pi tool to the correct `TraceEventType` and payload shape as defined in `scryrs-types`:

| Pi tool name | TraceEvent type | Payload type | Key field extraction |
|---|---|---|---|
| `read` | `FileOpened` | `FileOpenedPayload` | `path` ← `event.input.path` |
| `bash` | `CommandExecuted` | `CommandExecutedPayload` | `command` ← `event.input.command` |
| `ast_grep_search` | `SearchRun` | `SearchRunPayload` | `query` ← `event.input?.query` (defensive) |
| `edit` | `EditMade` | `EditMadePayload` | `target` ← `event.input.path` |
| `write` | `EditMade` | `EditMadePayload` | `target` ← `event.input.path` |
| `lsp_navigation` (success) | `SymbolInspected` | `SymbolInspectedPayload` | `name` ← `event.input?.symbol` (defensive) |
| `lsp_navigation` (failure) | `FailedLookup` | `FailedLookupPayload` | `subject` ← `event.input?.symbol` (defensive) |

#### Scenario: read maps to FileOpened
- **WHEN** an agent calls the `read` tool with a file path
- **THEN** the hook emits a `FileOpened` TraceEvent with `payload.path` set to the file path
- **AND** `tool_name` is set to `"read"`

#### Scenario: bash maps to CommandExecuted
- **WHEN** an agent calls the `bash` tool with a shell command
- **THEN** the hook emits a `CommandExecuted` TraceEvent with `payload.command` set to the command string
- **AND** `tool_name` is set to `"bash"`

#### Scenario: ast_grep_search maps to SearchRun
- **WHEN** an agent calls the `ast_grep_search` tool with a search query
- **THEN** the hook emits a `SearchRun` TraceEvent with `payload.query` set to the extracted query
- **AND** `tool_name` is set to `"ast_grep_search"`
- **AND** if the query field is missing, `payload.query` defaults to `"unknown"` and a warning is logged

#### Scenario: edit maps to EditMade
- **WHEN** an agent calls the `edit` tool with a file path
- **THEN** the hook emits an `EditMade` TraceEvent with `payload.target` set to the file path
- **AND** `tool_name` is set to `"edit"`

#### Scenario: write maps to EditMade
- **WHEN** an agent calls the `write` tool with a file path
- **THEN** the hook emits an `EditMade` TraceEvent with `payload.target` set to the file path
- **AND** `tool_name` is set to `"write"`

#### Scenario: lsp_navigation success maps to SymbolInspected
- **WHEN** an agent calls `lsp_navigation` and it succeeds (`event.isError` is false)
- **THEN** the hook emits a `SymbolInspected` TraceEvent with `payload.name` set to the navigation target
- **AND** the outcome is `Success`
- **AND** `tool_name` is set to `"lsp_navigation"`

#### Scenario: lsp_navigation failure maps to FailedLookup
- **WHEN** an agent calls `lsp_navigation` and it fails (`event.isError` is true)
- **THEN** the hook emits a `FailedLookup` TraceEvent with `payload.subject` set to the navigation target
- **AND** the outcome is `Failure` with a reason derived from the error context
- **AND** `tool_name` is set to `"lsp_navigation"`

### Requirement: Every TraceEvent carries the canonical envelope fields

Every event serialized by the hook SHALL include all required envelope fields: `schema_version` (`"0.1.0"` matching `scryrs_types::SCHEMA_VERSION`), `timestamp` (ISO 8601 / RFC 3339 string from `new Date().toISOString()`), `session_id` (the hook's generated session-scoped UUID), `event_type` (matching the mapped `TraceEventType`), `tool_name` (the Pi tool name string), `payload` (self-describing JSON object with `type` tag matching the payload family), and `outcome` (Success or Failure with optional reason).

#### Scenario: Event envelope is complete
- **WHEN** the hook constructs a TraceEvent for a supported tool
- **THEN** the serialized JSON includes all seven required fields
- **AND** `schema_version` equals `"0.1.0"`
- **AND** `timestamp` is a valid ISO 8601 string
- **AND** `session_id` is the hook's UUID, identical across all events in the session
- **AND** `payload` includes the `type` tag for self-describing dispatch

### Requirement: Hook delegates to scryrs record --stdin via pi.exec

The hook SHALL invoke `scryrs record --stdin` as a subprocess using Pi's `pi.exec()` API, passing the newline-delimited TraceEvent JSON as the `input` option. The hook SHALL apply a 5-second timeout. The hook SHALL NOT invoke `scryrs record --file` or any other command.

#### Scenario: Hook pipes JSONL via stdin
- **WHEN** the hook has constructed a TraceEvent
- **THEN** it calls `pi.exec('scryrs', ['record', '--stdin'], { input: jsonlString, timeout: 5000 })`
- **AND** the JSONL string is a single line of valid TraceEvent JSON terminated with a newline

#### Scenario: Hook does not use alternate ingestion modes
- **WHEN** the hook invokes scryrs
- **THEN** it uses only `record --stdin`
- **AND** it never uses `--file`, `hotspots`, or any other scryrs subcommand

### Requirement: scryrs is never registered as a Pi tool

The hook SHALL NOT call `pi.registerTool()`, `pi.registerCommand()`, `pi.setActiveTools()`, or any API that registers scryrs as a callable business tool or surface.

#### Scenario: Scryrs not in tool registry
- **WHEN** the Pi reference hook is loaded
- **THEN** no tool named `scryrs` appears in `pi.getAllTools()`
- **AND** the LLM can never call scryrs as a tool

### Requirement: Hook fails open on scryrs errors

The hook SHALL wrap the entire subprocess invocation in try-catch. If `pi.exec` throws (missing binary, non-zero exit, subprocess timeout, or any other error), the hook SHALL log the failure via `console.error` and SHALL NOT throw, block, or modify the tool result. The `tool_result` handler SHALL return `undefined` unconditionally.

#### Scenario: scryrs binary missing
- **GIVEN** scryrs is not installed or not on PATH
- **WHEN** the hook attempts to invoke `scryrs record --stdin`
- **THEN** the error is caught and logged via `console.error`
- **AND** the handler returns `undefined`
- **AND** the original tool result is delivered to the agent unchanged

#### Scenario: scryrs record exits non-zero
- **GIVEN** scryrs is available but rejects the event line
- **WHEN** the hook invokes `scryrs record --stdin`
- **THEN** the non-zero exit is caught
- **AND** the hook logs the tracing failure
- **AND** the agent-visible tool result is unchanged

#### Scenario: scryrs subprocess times out
- **GIVEN** scryrs hangs or exceeds the 5-second timeout
- **WHEN** the hook invokes `scryrs record --stdin`
- **THEN** Pi kills the subprocess and `pi.exec` throws
- **AND** the error is caught, logged, and the agent turn proceeds normally

### Requirement: Session demarcation — SessionStart emitted with unique session_id

The hook SHALL generate a unique session-scoped identifier on extension load and emit a `SessionStart` TraceEvent when the Pi `session_start` event fires. `SessionEnd` is explicitly deferred to a follow-up task.

#### Scenario: Session_id is generated on extension load
- **WHEN** the Pi extension factory function executes
- **THEN** a UUID session identifier is generated via `crypto.randomUUID()` and stored in module scope

#### Scenario: SessionStart is emitted on session_start
- **WHEN** Pi fires the `session_start` event
- **THEN** the hook emits a `SessionStart` TraceEvent with the generated `session_id`
- **AND** the event omits `tool_name` (lifecycle event)

#### Scenario: SessionEnd is not emitted
- **WHEN** Pi fires `session_shutdown`
- **THEN** the hook does not emit a `SessionEnd` event
- **AND** this is documented as a deferred concern in the README

### Requirement: Companion README documents install steps and mapping decisions

The `hooks/pi/README.md` SHALL document: consumer installation steps (copy to `~/.pi/agent/extensions/` or `.pi/extensions/`), the full tool-to-TraceEvent mapping table, the `write` → `EditMade` mapping rationale, the `lsp_navigation` conditional success/failure mapping, the assumed input field names for `ast_grep_search` and `lsp_navigation`, the fail-open guarantee, the deferred `SessionEnd` status, and that scryrs must be on PATH.

#### Scenario: Consumer can install from README
- **WHEN** a Pi user reads `hooks/pi/README.md`
- **THEN** they know where to copy the hook source
- **AND** they know what each Pi tool maps to in the trace output
- **AND** they know to verify `ast_grep_search` and `lsp_navigation` input fields against their Pi version

#### Scenario: Undocumented assumptions are surfaced
- **WHEN** a consumer reads the README
- **THEN** the assumed input field names for `ast_grep_search` (`query`) and `lsp_navigation` (`symbol`) are explicitly listed
- **AND** the README notes that these assumptions must be verified against the consumer's Pi tool definitions
- **AND** the `write` → `EditMade` decision is explained with a reference to the schema's lack of a `WriteMade` variant

### Requirement: Scope is limited to hooks/pi/ and companion docs

This change SHALL NOT modify any Rust crate, CLI behavior, wire format, existing OpenSpec capability specs, or any file outside `hooks/pi/`.

#### Scenario: No Rust changes are made
- **WHEN** this change is implemented
- **THEN** no files in `crates/` are modified

#### Scenario: No existing specs are modified
- **WHEN** this change is implemented
- **THEN** `openspec/specs/trace-event-schema/spec.md` is unchanged
- **AND** `openspec/specs/scryrs-record-endpoint/spec.md` is unchanged
- **AND** `openspec/specs/trace-hook-contract/spec.md` is unchanged

