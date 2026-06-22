## MODIFIED Requirements

### Requirement: Hook intercepts observer-first Claude Code PreToolUse events with debug-gated Bash

The reference hook SHALL intercept Claude Code PreToolUse events by default for Read, Grep, Glob, Edit, Write, NotebookEdit, WebSearch, and WebFetch tool invocations. The `Bash` tool SHALL be excluded from default capture and SHALL be intercepted only when `SCRYRS_DEBUG` is set to a non-empty value. The hook SHALL pass through any other tool event without emitting trace data.

#### Scenario: Default mode intercepts non-Bash observer tools

- **GIVEN** the hook is installed in a Claude Code environment
- **WHEN** Claude Code is about to execute Read, Grep, Glob, Edit, Write, NotebookEdit, WebSearch, or WebFetch while `SCRYRS_DEBUG` is unset
- **THEN** the hook receives the PreToolUse event and forwards a TraceEvent to `scryrs record --stdin`

#### Scenario: Default mode ignores Bash

- **GIVEN** the hook is installed in a Claude Code environment
- **WHEN** Claude Code is about to execute Bash while `SCRYRS_DEBUG` is unset
- **THEN** the hook takes no trace-capture action and returns success to Claude Code

#### Scenario: Debug mode intercepts Bash

- **GIVEN** the hook is installed in a Claude Code environment
- **WHEN** Claude Code is about to execute Bash while `SCRYRS_DEBUG` is set to a non-empty value
- **THEN** the hook forwards a `CommandExecuted` TraceEvent to `scryrs record --stdin`

### Requirement: Hook maps Claude Code tools to canonical TraceEvent families under observer-first defaults

The reference hook SHALL map Claude Code tool names to scryrs `TraceEventType` variants as follows: Read→FileOpened, Grep→SearchRun, Glob→SearchRun, Edit→EditMade, Write→EditMade, NotebookEdit→EditMade, WebSearch→SearchRun, WebFetch→DocRetrieved. Bash→CommandExecuted SHALL remain available only in debug mode. Each emitted event SHALL carry `tool_name` set to the original Claude Code tool name.

#### Scenario: Default observer tools retain canonical mappings

- **WHEN** the hook processes any default-captured Claude Code tool
- **THEN** the emitted TraceEvent uses the same canonical family and payload shape previously defined for that tool

#### Scenario: Bash mapping is conditional on debug mode

- **WHEN** the hook processes a Bash PreToolUse event
- **THEN** it emits `CommandExecuted` only when `SCRYRS_DEBUG` is set to a non-empty value
- **AND** no Bash trace event is emitted when `SCRYRS_DEBUG` is unset

### Requirement: Debug-gated Bash rewrite-tool commands are recorded exactly as observed on PreToolUse

For Claude Code `Bash` PreToolUse events observed while `SCRYRS_DEBUG` is set to a non-empty value, the reference hook SHALL copy `tool_input.command` into `CommandExecuted.payload.command` exactly as observed when the scryrs hook runs. The hook SHALL NOT normalize, strip rewrite prefixes, split compound commands, or reconstruct original intent.

#### Scenario: RTK-prefixed Bash command is persisted as-is in debug mode

- **WHEN** the Claude Code hook processes a Bash PreToolUse event whose `tool_input.command` is `rtk ls -la` and `SCRYRS_DEBUG` is set
- **THEN** the emitted `CommandExecuted` event has `payload.command` equal to `rtk ls -la`
- **AND** the hook returns `{continue: true}` to Claude Code

#### Scenario: Bash command is not persisted outside debug mode

- **WHEN** the Claude Code hook processes a Bash PreToolUse event and `SCRYRS_DEBUG` is unset
- **THEN** no `CommandExecuted` event is emitted

### Requirement: Hook README documents observer-first installation and debug-gated Bash limitation

The `hooks/claude-code/README.md` SHALL document consumer-side installation steps, default tool coverage excluding Bash, `SCRYRS_DEBUG` as the opt-in control for Bash trace capture, the PreToolUse limitations, the fail-open behavior and warning log location, the tool-to-event mapping table, the debug-gated rewrite-order caveat for Bash, and the prerequisite that `scryrs` is on PATH.

#### Scenario: Integrator sees Bash is not default product surface

- **WHEN** a Claude Code integrator reads `hooks/claude-code/README.md`
- **THEN** the README states that Bash capture is disabled by default
- **AND** the README states that `SCRYRS_DEBUG` re-enables Bash tracing for diagnostic sessions only

#### Scenario: README explains debug-gated hook-order caveat

- **WHEN** a Claude Code integrator reads the rewrite-tool compatibility section
- **THEN** it states that scryrs records Bash only when debug mode is enabled
- **AND** it states that co-installed rewrite hooks can change the observed command value based on hook order
