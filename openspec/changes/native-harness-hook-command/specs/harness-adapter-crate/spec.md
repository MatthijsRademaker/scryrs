## ADDED Requirements

### Requirement: `scryrs-adapter-harness` crate is the single source of truth for translation

A workspace crate `scryrs-adapter-harness` SHALL hold all harness-event → canonical `TraceEvent` translation. It SHALL expose a `HarnessAdapter` abstraction with a `claude-code` and a `pi` implementation. No tool→event mapping logic SHALL remain in `hooks/pi/index.ts`, and no JavaScript translator SHALL exist for Claude Code.

#### Scenario: crate is a workspace member

- **WHEN** inspecting the workspace `Cargo.toml`
- **THEN** `crates/scryrs-adapter-harness` is listed under `members`

#### Scenario: translation is not duplicated in TypeScript

- **WHEN** inspecting `hooks/pi/index.ts`
- **THEN** it contains no tool→event-type mapping switch
- **AND** it delegates translation to `scryrs hook pi`

### Requirement: adapter returns zero or one canonical TraceEvent per harness event

`HarnessAdapter::translate` SHALL return `Some(TraceEvent)` for a tracked tool and `None` for an untracked tool (pass-through). Returned events SHALL carry the canonical envelope: `schema_version`, `timestamp`, `session_id`, `event_type`, `tool_name`, self-describing `payload`, and `outcome`. Multi-line payload values SHALL be collapsed so each serialized event occupies one line.

#### Scenario: tracked tool maps to a canonical event

- **WHEN** an adapter receives a tracked tool event
- **THEN** it returns `Some` with the canonical envelope and the correct `event_type`

#### Scenario: untracked tool passes through

- **WHEN** an adapter receives an untracked tool event
- **THEN** it returns `None`

### Requirement: Claude Code adapter maps PascalCase tools under observer-first defaults

The `claude-code` adapter SHALL match Claude Code tool names as documented PascalCase and map: Read→FileOpened, Grep→SearchRun, Glob→SearchRun, Edit→EditMade, Write→EditMade, NotebookEdit→EditMade, WebSearch→SearchRun, WebFetch→DocRetrieved. Bash→CommandExecuted SHALL be emitted only when `SCRYRS_DEBUG` is set to a non-empty value. Because PreToolUse fires pre-execution, every emitted event SHALL carry `outcome = Success`.

#### Scenario: PascalCase WebSearch maps to SearchRun

- **WHEN** the adapter receives a `tool_name` of `"WebSearch"`
- **THEN** it emits a `SearchRun` event (the name is matched as PascalCase, not lowercased)

#### Scenario: Bash is debug-gated

- **WHEN** the adapter receives a `"Bash"` event and `SCRYRS_DEBUG` is unset
- **THEN** it returns `None`
- **AND** when `SCRYRS_DEBUG` is set it returns a `CommandExecuted` event with `command` copied verbatim

#### Scenario: pre-execution outcome is always Success

- **WHEN** the adapter emits any Claude Code event
- **THEN** its `outcome` is `Success`

### Requirement: Pi adapter maps lowercase tools and reflects execution outcome

The `pi` adapter SHALL match Pi tool names as lowercase and map: read→FileOpened, ast_grep_search→SearchRun, edit→EditMade, write→EditMade. For `lsp_navigation` it SHALL emit `SymbolInspected` on success and `FailedLookup` when the event indicates an error. Bash→CommandExecuted SHALL be debug-gated. Because `tool_result` fires post-execution, the adapter SHALL set `outcome` to `Failure` when the event indicates an error and `Success` otherwise.

#### Scenario: lsp_navigation success vs failure

- **WHEN** the adapter receives an `lsp_navigation` event that did not error
- **THEN** it emits `SymbolInspected`
- **AND** when the event indicates an error it emits `FailedLookup`

#### Scenario: outcome reflects isError

- **WHEN** the adapter receives a tracked tool event whose `isError` is true
- **THEN** the emitted event's `outcome` is `Failure`
