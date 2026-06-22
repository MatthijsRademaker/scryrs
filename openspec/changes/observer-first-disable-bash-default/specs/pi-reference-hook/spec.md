## MODIFIED Requirements

### Requirement: Hook filters to observer-first Pi tools with debug-gated Bash

The hook SHALL forward trace events by default only for Pi tool names `read`, `ast_grep_search`, `lsp_navigation`, `edit`, and `write`. The `bash` tool SHALL be excluded from default capture and SHALL be forwarded only when `SCRYRS_DEBUG` is set to a non-empty value. Calls to any other tool SHALL be silently ignored by the hook.

#### Scenario: Default mode forwards only observer-first native tools

- **WHEN** an agent calls `read`, `ast_grep_search`, `lsp_navigation`, `edit`, or `write` while `SCRYRS_DEBUG` is unset
- **THEN** the hook constructs a TraceEvent and forwards it to `scryrs record`

#### Scenario: Default mode ignores Bash

- **WHEN** an agent calls `bash` while `SCRYRS_DEBUG` is unset
- **THEN** the hook returns without constructing a TraceEvent or invoking scryrs

#### Scenario: Debug mode re-enables Bash capture

- **WHEN** an agent calls `bash` while `SCRYRS_DEBUG` is set to a non-empty value
- **THEN** the hook constructs a `CommandExecuted` TraceEvent and forwards it to `scryrs record`

### Requirement: Tool events map to canonical TraceEvent families under observer-first defaults

The hook SHALL map each supported Pi tool to the correct `TraceEventType` and payload shape as defined in `scryrs-types`:

| Pi tool name | Capture mode | TraceEvent type | Payload type | Key field extraction |
|---|---|---|---|---|
| `read` | default | `FileOpened` | `FileOpenedPayload` | `path` ← `event.input.path` |
| `ast_grep_search` | default | `SearchRun` | `SearchRunPayload` | `query` ← `event.input?.query` (defensive) |
| `edit` | default | `EditMade` | `EditMadePayload` | `target` ← `event.input.path` |
| `write` | default | `EditMade` | `EditMadePayload` | `target` ← `event.input.path` |
| `lsp_navigation` (success) | default | `SymbolInspected` | `SymbolInspectedPayload` | `name` ← `event.input?.symbol` (defensive) |
| `lsp_navigation` (failure) | default | `FailedLookup` | `FailedLookupPayload` | `subject` ← `event.input?.symbol` (defensive) |
| `bash` | debug-only | `CommandExecuted` | `CommandExecutedPayload` | `command` ← `event.input.command` |

#### Scenario: Default tools keep existing event mappings

- **WHEN** an agent calls any default-captured Pi tool
- **THEN** the hook emits the same canonical `TraceEventType` and payload family previously defined for that tool

#### Scenario: Bash mapping is conditional on debug mode

- **WHEN** an agent calls `bash`
- **THEN** the hook emits `CommandExecuted` only when `SCRYRS_DEBUG` is set to a non-empty value
- **AND** no Bash trace event is emitted when `SCRYRS_DEBUG` is unset

### Requirement: Debug-gated Bash rewrite-tool commands are recorded exactly as observed on tool_result

For Pi `bash` tool results observed while `SCRYRS_DEBUG` is set to a non-empty value, the reference hook SHALL copy `event.input.command` into `CommandExecuted.payload.command` exactly as observed at the `tool_result` boundary. The hook SHALL NOT normalize, strip rewrite prefixes, split compound commands, or reconstruct original intent.

#### Scenario: RTK-prefixed Bash command is persisted as-is in debug mode

- **WHEN** the Pi hook processes a `tool_result` event for `bash` whose `event.input.command` is `rtk ls -la` and `SCRYRS_DEBUG` is set
- **THEN** the emitted `CommandExecuted` event has `payload.command` equal to `rtk ls -la`
- **AND** the handler returns `undefined`

#### Scenario: Bash command is not persisted outside debug mode

- **WHEN** the Pi hook processes a `tool_result` event for `bash` and `SCRYRS_DEBUG` is unset
- **THEN** no `CommandExecuted` event is emitted

### Requirement: Companion README documents observer-first boundary and debug-gated Bash capture

The `hooks/pi/README.md` SHALL document: consumer installation steps, default tracked tools excluding Bash, `SCRYRS_DEBUG` as the opt-in control for Bash capture, the full default tool-to-TraceEvent mapping table, debug-gated observed-command semantics for Bash, the `write` → `EditMade` mapping rationale, the `lsp_navigation` conditional success/failure mapping, the assumed input field names for `ast_grep_search` and `lsp_navigation`, the fail-open guarantee, the deferred `SessionEnd` status, and that scryrs must be on PATH.

#### Scenario: README explains default observer-first behavior

- **WHEN** a Pi integrator reads the companion README
- **THEN** it states that Bash is not captured by default
- **AND** it lists `SCRYRS_DEBUG` as the switch that re-enables Bash tracing for diagnostics

#### Scenario: README explains debug-gated observed-command semantics

- **WHEN** a Pi integrator reads the Bash compatibility section
- **THEN** it states that scryrs records `event.input.command` only in debug mode
- **AND** it states that rewrite prefixes such as `rtk` are persisted as-is when Bash capture is enabled
