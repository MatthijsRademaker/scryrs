## MODIFIED Requirements

### Requirement: Integration hooks both post-tool events and derives a real outcome

The Claude Code integration SHALL be registered on **both** `PostToolUse` and `PostToolUseFailure`, and SHALL NOT be registered on `PreToolUse`.

`PostToolUse` fires only after a tool call *succeeds*; failures arrive on the separate `PostToolUseFailure` event. Registering on `PostToolUse` alone would make every recorded outcome `Success`, reproducing the `PreToolUse` defect this requirement replaces. The two events are mutually exclusive per tool call, so registering both SHALL NOT double-count.

The outcome SHALL be derived from the payload's `hook_event_name`, not inferred from `tool_response` contents.

`scryrs init --agent claude-code` SHALL write both registrations into `.claude/settings.json` and SHALL remove any scryrs `PreToolUse` registration it previously wrote. Foreign `PreToolUse` entries belonging to other tools SHALL be preserved.

#### Scenario: Successful tool call yields a success outcome

- **WHEN** Claude Code fires `PostToolUse` for a supported tool
- **THEN** `scryrs hook claude-code` emits a TraceEvent with `Outcome::Success`

#### Scenario: Failed tool call yields a failure outcome

- **WHEN** Claude Code fires `PostToolUseFailure` for a supported tool
- **THEN** the emitted TraceEvent carries `Outcome::Failure`
- **AND** the outcome is derived from `hook_event_name`, not from `tool_response`

#### Scenario: A failed read maps to FailedLookup

- **WHEN** a `Read` tool call fails and `PostToolUseFailure` fires
- **THEN** the hook emits a `FailedLookup` TraceEvent with the attempted path as subject
- **AND** it does NOT emit a `FileOpened` event with a failure outcome
- **AND** this matches the Pi adapter's behavior for the same situation

#### Scenario: FileOpened never carries a failure outcome

- **WHEN** any Claude Code tool event is translated
- **THEN** no emitted `FileOpened` event carries `Outcome::Failure`

#### Scenario: init registers both post-tool events and removes PreToolUse

- **GIVEN** a `.claude/settings.json` containing a scryrs `PreToolUse` registration written by a prior install
- **WHEN** `scryrs init --agent claude-code` runs
- **THEN** the settings file contains the scryrs registration under both `PostToolUse` and `PostToolUseFailure`
- **AND** the scryrs `PreToolUse` registration is removed
- **AND** a `PreToolUse` key left holding nothing else is removed entirely

#### Scenario: Foreign PreToolUse entries survive the migration

- **GIVEN** a `PreToolUse` array containing both a foreign hook entry and a scryrs entry
- **WHEN** `scryrs init --agent claude-code` runs
- **THEN** the foreign entry remains
- **AND** the scryrs entry is removed

#### Scenario: No double-counting after migration

- **WHEN** a supported tool runs after migration
- **THEN** exactly one TraceEvent is persisted for that tool call

#### Scenario: init is idempotent across both events

- **WHEN** `scryrs init --agent claude-code` runs twice
- **THEN** each registered event holds exactly one scryrs registration

#### Scenario: doctor reports a stale PreToolUse registration

- **GIVEN** a `.claude/settings.json` still carrying a scryrs `PreToolUse` registration
- **WHEN** `scryrs doctor` runs
- **THEN** it reports the stale registration as a warn finding with an actionable message
- **AND** the exit code is not escalated to an error

#### Scenario: doctor reports a partial registration

- **GIVEN** the scryrs hook is registered on only one of the two post-tool events
- **WHEN** `scryrs doctor` runs
- **THEN** it reports a warn finding naming the missing event

### Requirement: The hook emits no decision and no output transformation

`PostToolUse` supports a top-level `decision: "block"` and an `updatedToolOutput` field that replaces the tool's result before Claude sees it. The hook SHALL emit neither.

The hook SHALL write nothing to stdout, SHALL NOT modify the agent-visible tool result, and SHALL always exit 0 — including on the drop path for a missing key input field.

Diagnostics SHALL be written to `.scryrs/hooks/claude-code-warnings.log` rather than unconditionally to stderr, because Claude Code can surface hook stderr and a trace hook must not inject its own output into the model's context.

#### Scenario: No decision or output transformation is emitted

- **WHEN** the hook command processes any post-tool event
- **THEN** stdout is empty
- **AND** no `decision` or `updatedToolOutput` field is produced
- **AND** the agent-visible tool result is unmodified

#### Scenario: Non-interference holds on the drop path

- **WHEN** the hook drops an event because a key input field was absent
- **THEN** stdout is empty and the exit code is 0
- **AND** the diagnostic is recorded in the warning log

### Requirement: Hook intercepts the supported Claude Code tool events with normalized subjects

The integration SHALL intercept `PostToolUse` and `PostToolUseFailure` events for `Read`, `Bash`, `Grep`, `Glob`, `Edit`, `Write`, `NotebookEdit`, `WebSearch`, and `WebFetch`, and SHALL pass through any other tool event without emitting trace data.

Path subjects (`Read`, `Edit`, `Write`, `NotebookEdit`) SHALL be persisted in canonical repository-relative form per the `trace-hook-contract` normalization requirement. Non-path subjects SHALL NOT be normalized: `Grep`/`Glob` patterns, `WebSearch` queries, `WebFetch` URLs, and `Bash` commands are not repository paths.

`Bash` SHALL remain captured only when `SCRYRS_DEBUG` is set to a non-empty value.

#### Scenario: Supported tool with a path subject is normalized

- **WHEN** a `Read`, `Edit`, `Write`, or `NotebookEdit` event is processed
- **THEN** the persisted subject is in canonical repository-relative POSIX form

#### Scenario: A WebFetch URL is not path-normalized

- **WHEN** a `WebFetch` event carries a URL containing `..` segments
- **THEN** the persisted `doc_ref` is the URL verbatim

#### Scenario: Unsupported tool passes through

- **WHEN** a post-tool event arrives for a tool outside the supported set
- **THEN** no trace event is emitted and the command exits 0

#### Scenario: Bash stays opt-in

- **WHEN** a `Bash` post-tool event arrives and `SCRYRS_DEBUG` is unset or empty
- **THEN** no trace event is emitted
