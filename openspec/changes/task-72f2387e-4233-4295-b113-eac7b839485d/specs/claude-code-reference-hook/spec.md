# claude-code-reference-hook Specification

## Purpose

Defines requirements for the Claude Code reference trace hook under `hooks/claude-code/` — a thin JavaScript transport module that intercepts Claude Code PreToolUse events, maps them to canonical `TraceEvent` JSON objects, and forwards them to `scryrs record --stdin` without ever becoming a tool proxy or altering agent-visible tool behavior.

## ADDED Requirements

### Requirement: Reference hook source lives under hooks/claude-code/

The system SHALL provide a reference hook implementation under `hooks/claude-code/` containing a JavaScript hook module (`scryrs-hook.js`), a consumer-facing README, and no committed consumer-side `.claude/` configuration files.

#### Scenario: Hook source is discoverable

- **WHEN** a Claude Code integrator navigates the repository
- **THEN** they find `hooks/claude-code/scryrs-hook.js` as the reference hook module
- **AND** they find `hooks/claude-code/README.md` with installation and usage instructions

#### Scenario: No consumer config is committed

- **WHEN** the repository is inspected after this change
- **THEN** no `.claude/` directory, no Claude Code hook configuration file, and no consumer-side installation artifacts are present in the repository

### Requirement: Hook intercepts the nine specified Claude Code PreToolUse events

The reference hook SHALL intercept Claude Code PreToolUse events for Read, Bash, Grep, Glob, Edit, Write, NotebookEdit, WebSearch, and WebFetch tool invocations. The hook SHALL only respond to these nine tool names and SHALL pass through any other tool event without emitting trace data.

#### Scenario: Listed tool event is intercepted

- **GIVEN** the hook is installed in a Claude Code environment
- **WHEN** Claude Code is about to execute a Read, Bash, Grep, Glob, Edit, Write, NotebookEdit, WebSearch, or WebFetch tool
- **THEN** the hook receives the PreToolUse event
- **AND** the hook constructs and forwards a TraceEvent to `scryrs record --stdin`

#### Scenario: Unlisted tool event is passed through

- **GIVEN** the hook is installed in a Claude Code environment
- **WHEN** Claude Code is about to execute a tool not in the nine-tool whitelist (e.g., Task, AskUserQuestion)
- **THEN** the hook takes no action and returns success to Claude Code

### Requirement: Hook maps Claude Code tools to canonical TraceEvent families

The reference hook SHALL map Claude Code tool names to scryrs `TraceEventType` variants as follows: Read→FileOpened, Bash→CommandExecuted, Grep→SearchRun, Glob→SearchRun, Edit→EditMade, Write→EditMade, NotebookEdit→EditMade, WebSearch→SearchRun, WebFetch→DocRetrieved. Each event SHALL carry `tool_name` set to the original Claude Code tool name.

#### Scenario: Read tool maps to FileOpened

- **WHEN** the hook processes a Read PreToolUse event
- **THEN** the emitted TraceEvent has `event_type: "FileOpened"`
- **AND** `payload.type: "FileOpened"`
- **AND** `payload.path` contains the file path from the tool input
- **AND** `tool_name` is `"read"`

#### Scenario: Bash tool maps to CommandExecuted

- **WHEN** the hook processes a Bash PreToolUse event
- **THEN** the emitted TraceEvent has `event_type: "CommandExecuted"`
- **AND** `payload.type: "CommandExecuted"`
- **AND** `payload.command` contains the command string from the tool input
- **AND** `tool_name` is `"bash"`

#### Scenario: Grep tool maps to SearchRun

- **WHEN** the hook processes a Grep PreToolUse event
- **THEN** the emitted TraceEvent has `event_type: "SearchRun"`
- **AND** `payload.type: "SearchRun"`
- **AND** `payload.query` contains the search pattern from the tool input
- **AND** `tool_name` is `"grep"`

#### Scenario: Glob tool maps to SearchRun

- **WHEN** the hook processes a Glob PreToolUse event
- **THEN** the emitted TraceEvent has `event_type: "SearchRun"`
- **AND** `payload.type: "SearchRun"`
- **AND** `payload.query` contains the glob pattern from the tool input
- **AND** `tool_name` is `"glob"`

#### Scenario: Edit tool maps to EditMade

- **WHEN** the hook processes an Edit PreToolUse event
- **THEN** the emitted TraceEvent has `event_type: "EditMade"`
- **AND** `payload.type: "EditMade"`
- **AND** `payload.target` contains the file path from the tool input
- **AND** `tool_name` is `"edit"`

#### Scenario: Write tool maps to EditMade

- **WHEN** the hook processes a Write PreToolUse event
- **THEN** the emitted TraceEvent has `event_type: "EditMade"`
- **AND** `payload.type: "EditMade"`
- **AND** `payload.target` contains the file path from the tool input
- **AND** `tool_name` is `"write"`

#### Scenario: NotebookEdit tool maps to EditMade

- **WHEN** the hook processes a NotebookEdit PreToolUse event
- **THEN** the emitted TraceEvent has `event_type: "EditMade"`
- **AND** `payload.type: "EditMade"`
- **AND** `payload.target` contains the file path from the tool input
- **AND** `tool_name` is `"notebookedit"`

#### Scenario: WebSearch tool maps to SearchRun

- **WHEN** the hook processes a WebSearch PreToolUse event
- **THEN** the emitted TraceEvent has `event_type: "SearchRun"`
- **AND** `payload.type: "SearchRun"`
- **AND** `payload.query` contains the search term from the tool input
- **AND** `tool_name` is `"web_search"`

#### Scenario: WebFetch tool maps to DocRetrieved

- **WHEN** the hook processes a WebFetch PreToolUse event
- **THEN** the emitted TraceEvent has `event_type: "DocRetrieved"`
- **AND** `payload.type: "DocRetrieved"`
- **AND** `payload.doc_ref` contains the URL from the tool input
- **AND** `tool_name` is `"web_fetch"`

### Requirement: Emitted events conform to canonical TraceEvent schema

Every TraceEvent emitted by the hook SHALL conform to the canonical `TraceEvent` schema defined in `crates/scryrs-types/src/lib.rs`. Each event SHALL carry `schema_version` (the current `SCHEMA_VERSION`), `timestamp` (RFC 3339), `session_id`, `event_type`, `tool_name`, `payload`, and `outcome`.

#### Scenario: Event envelope fields are present

- **WHEN** the hook emits a TraceEvent
- **THEN** the JSON includes `schema_version` matching `"0.1.0"`
- **AND** `timestamp` is an RFC 3339 timestamp
- **AND** `session_id` is a non-empty string
- **AND** `event_type` is a valid TraceEventType variant
- **AND** `tool_name` is the original Claude Code tool name
- **AND** `payload` is a self-describing JSON object with a `type` tag
- **AND** `outcome` is present

#### Scenario: Payload is self-describing

- **WHEN** the hook emits a TraceEvent for any tool type
- **THEN** the serialized payload includes a `type` field matching the `event_type`
- **AND** consumers can identify the concrete payload family from JSON alone

### Requirement: PreToolUse events carry unconditional Success outcome

The reference hook SHALL emit `outcome: Success` on every event, since the PreToolUse hook fires before tool execution and the real outcome cannot be determined. This limitation SHALL be documented in the hook README and the trace-hook-contract documentation.

#### Scenario: Every event carries Success outcome

- **WHEN** the hook emits a TraceEvent for any intercepted tool
- **THEN** the `outcome` field serializes as `{"result":"Success"}`
- **AND** no event carries `outcome: Failure` regardless of tool behavior

#### Scenario: PreToolUse limitation is documented

- **WHEN** an integrator reads the hook README or trace-hook-contract.md
- **THEN** the documentation states that PreToolUse-only hooks emit `outcome: Success` unconditionally
- **AND** the documentation states that these are pre-execution metadata signals, not post-execution outcomes

### Requirement: Session IDs are per-process UUID v4 without lifecycle events

The reference hook SHALL generate a unique session identifier for each hook process invocation. If Claude Code provides a session-scoped stable identifier via the hook context, the hook SHALL prefer that; otherwise it SHALL generate a UUID v4. The hook SHALL NOT emit SessionStart or SessionEnd lifecycle events, as PreToolUse hooks have no session-open or session-close trigger.

#### Scenario: Session ID is stable per process

- **WHEN** the hook processes multiple PreToolUse events within a single Claude Code session
- **THEN** all emitted events carry the same `session_id` value
- **AND** the `session_id` is a UUID v4 string (preferred) or a stable session-scoped identifier

#### Scenario: No lifecycle events are emitted

- **WHEN** the hook runs during a Claude Code session
- **THEN** no event with `event_type: "SessionStart"` is emitted
- **AND** no event with `event_type: "SessionEnd"` is emitted
- **AND** only subject-bearing tool events are produced

### Requirement: Hook forwards events to scryrs record --stdin

The reference hook SHALL use `scryrs record --stdin` as the only ingestion mode, spawning `scryrs` as a child process and piping newline-delimited JSON TraceEvent data to its stdin. The hook SHALL NOT use file mode or any alternate ingestion path.

#### Scenario: Hook invokes scryrs record via stdin pipe

- **WHEN** the hook has constructed a TraceEvent
- **THEN** the hook spawns `scryrs record --stdin` as a subprocess
- **AND** the hook writes the JSON TraceEvent (single line, newline-terminated) to the subprocess stdin
- **AND** the hook closes stdin after writing

#### Scenario: Hook does not use alternate ingestion modes

- **WHEN** the hook forwards events to scryrs
- **THEN** it does not invoke `scryrs record --file <PATH>`
- **AND** it does not use any socket, HTTP, or IPC mechanism

### Requirement: Hook is transparent to the agent

The reference hook SHALL never alter the original Claude Code tool's stdout, stderr, or exit status. The hook SHALL return a valid JSON response to Claude Code indicating success regardless of scryrs behavior. The hook SHALL NOT proxy tool execution or sit in the tool execution path.

#### Scenario: Original tool output is preserved

- **GIVEN** the hook runs alongside a Claude Code tool invocation
- **WHEN** the tool completes (successfully or with error)
- **THEN** the tool's stdout is unmodified
- **AND** the tool's stderr is unmodified
- **AND** the tool's exit status is unmodified

#### Scenario: Hook returns success to Claude Code regardless of scryrs outcome

- **GIVEN** scryrs record rejects the event or is unavailable
- **WHEN** the hook completes
- **THEN** the hook returns a valid JSON response to Claude Code indicating success
- **AND** the original tool execution proceeds normally

### Requirement: Hook fails open when scryrs is unavailable

The reference hook SHALL fail open: when `scryrs` is not found on PATH, when `scryrs record` exits non-zero, or when the subprocess crashes, the hook SHALL return success to Claude Code and SHALL write a timestamped warning to `.scryrs/hooks/claude-code-warnings.log` (relative to the consumer's project root). The hook SHALL NOT write warnings to stderr (which Claude Code captures as tool stderr) and SHALL NOT block or alter tool execution.

#### Scenario: scryrs binary is missing

- **GIVEN** the `scryrs` binary is not on PATH
- **WHEN** the hook attempts to spawn `scryrs record --stdin`
- **THEN** the hook catches the spawn error
- **AND** the hook writes a timestamped warning to `.scryrs/hooks/claude-code-warnings.log`
- **AND** the hook returns success to Claude Code
- **AND** the original tool executes normally

#### Scenario: scryrs record exits non-zero

- **GIVEN** `scryrs record --stdin` exits with code 1 or 2
- **WHEN** the hook receives the subprocess exit
- **THEN** the hook writes a timestamped warning to `.scryrs/hooks/claude-code-warnings.log`
- **AND** the hook returns success to Claude Code
- **AND** no error is written to stderr

#### Scenario: Warning log is outside agent context

- **WHEN** the hook writes a fail-open warning
- **THEN** the warning is appended to a dedicated log file (`.scryrs/hooks/claude-code-warnings.log`)
- **AND** the warning includes an ISO-8601 timestamp and a human-readable reason
- **AND** the warning is NOT written to stdout or stderr

### Requirement: Multi-line payload values do not break JSONL

The reference hook SHALL ensure that payload values containing embedded newlines (notably Bash `CommandExecuted.payload.command`) are escaped or collapsed so that each TraceEvent occupies exactly one physical line in the JSONL stream piped to `scryrs record --stdin`.

#### Scenario: Multi-line command is collapsed

- **GIVEN** a Bash tool invocation with a multi-line command (e.g., a for-loop or heredoc)
- **WHEN** the hook constructs the TraceEvent
- **THEN** the `payload.command` string contains no literal newline characters
- **AND** the serialized JSON TraceEvent is a single physical line

### Requirement: Hook README documents consumer-side installation and limitations

The `hooks/claude-code/README.md` SHALL document consumer-side installation steps for Claude Code's hook system, the PreToolUse limitations (unconditional Success outcome, no session lifecycle events, per-process session IDs), the fail-open behavior and warning log location, the tool-to-event mapping table, and the prerequisites (`scryrs` on PATH). The README SHALL explicitly state that consumer `.claude/` configuration is not stored in this repository.

#### Scenario: Integrator can install the hook from documentation alone

- **WHEN** a Claude Code integrator reads `hooks/claude-code/README.md`
- **THEN** they can install and configure the hook in their Claude Code environment
- **AND** they understand the limitations of PreToolUse-only trace capture
- **AND** they know where fail-open warnings are written

#### Scenario: README states no consumer config in repo

- **WHEN** an integrator reads the README
- **THEN** it explicitly states that consumer-side `.claude/` configuration files are not stored in this repository
- **AND** the integrator understands they must create their own `.claude/` hook configuration

### Requirement: Hook contract and roadmap docs reflect hook existence

The project documentation SHALL be updated to reflect the existence of the Claude Code reference hook. `.devagent/docs/docs/trace-hook-contract.md` SHALL remove "forthcoming Phase 1 deliverable" language for the Claude Code hook and instead reference the existing implementation at `hooks/claude-code/`. `.devagent/docs/docs/roadmap.mdx` SHALL update the "Current Starting Point" section to remove the claim that reference hooks are absent.

#### Scenario: Hook contract no longer says Claude hook is forthcoming

- **WHEN** an integrator reads the Reference Hooks section of trace-hook-contract.md
- **THEN** the Claude Code hook is described as existing at `hooks/claude-code/`
- **AND** the text no longer says "forthcoming Phase 1 deliverable" or "does not exist in the repository yet"

#### Scenario: Roadmap no longer says reference hooks are absent

- **WHEN** a reader reviews the roadmap "Current Starting Point" section
- **THEN** it no longer states that reference hooks remain "forthcoming"
- **AND** it reflects that `hooks/claude-code/` exists

### Requirement: Automated verification covers forwarding and fail-open behavior

A verification script (`scripts/hook-test`) SHALL exercise the reference hook independently from the Rust toolchain. The script SHALL verify: (a) correct JSON shaping and tool-to-event mapping for all nine tools, (b) happy-path forwarding to `scryrs record --stdin`, and (c) fail-open behavior when `scryrs` is missing or exits non-zero, without altering simulated tool output or exit code.

#### Scenario: JSON shaping verification passes

- **WHEN** `scripts/hook-test` runs
- **THEN** it validates that the hook produces valid canonical TraceEvent JSON for each of the nine supported tool types
- **AND** each event carries correct `event_type`, `tool_name`, payload fields, and `outcome: Success`

#### Scenario: Happy-path forwarding verification passes

- **WHEN** `scripts/hook-test` runs with `scryrs` available on PATH
- **THEN** it verifies that hook events are accepted by `scryrs record --stdin`

#### Scenario: Fail-open verification passes

- **WHEN** `scripts/hook-test` runs with `scryrs` unavailable or simulating non-zero exit
- **THEN** it verifies that the hook returns success
- **AND** it verifies that the hook does not alter simulated tool output