## MODIFIED Requirements

### Requirement: Hook filters to the mapped Pi tools

The hook SHALL forward trace events only for the Pi core tools `read`, `bash`, `edit`, `write`, `grep`, and `find`, plus the `pi-lens` extension tool `ast_grep_search`. Calls to any other tool SHALL be silently ignored.

Pi's core tool set is the closed union `read | bash | edit | write | grep | find | ls`. `ls` SHALL NOT be mapped: its `path` input is optional and a directory listing is not a subject in any canonical event family.

`lsp_navigation` SHALL NOT be mapped. It is a `pi-lens` extension tool and an operation dispatcher with no single key input field, so there is no honest subject to record for it.

#### Scenario: Mapped tool is forwarded

- **WHEN** an agent calls `read`, `bash`, `edit`, `write`, `grep`, `find`, or `ast_grep_search`
- **THEN** the hook constructs a TraceEvent and forwards it to `scryrs record`

#### Scenario: ls is ignored

- **WHEN** an agent calls `ls`
- **THEN** no trace event is emitted and the hook exits 0

#### Scenario: lsp_navigation is ignored

- **WHEN** an agent calls `lsp_navigation` with any `operation`
- **THEN** no trace event is emitted and the hook exits 0

#### Scenario: Unmapped tool is ignored

- **WHEN** an agent calls a tool outside the mapped set
- **THEN** the hook returns without constructing a TraceEvent or invoking scryrs

### Requirement: Tool events map to canonical TraceEvent families

The hook SHALL map each mapped Pi tool to the correct `TraceEventType` and payload shape as defined in `scryrs-types`. Key input field names SHALL be those observed in real Pi payloads and SHALL be frozen by a golden fixture per tool. A missing key input field SHALL drop the event with a diagnostic per the `trace-hook-contract` extraction requirement; it SHALL NOT default to `"unknown"`.

A `read` failure (`event.isError` is true) means the agent addressed a path that does not exist and SHALL map to `FailedLookup`.

| Pi tool | Origin | TraceEvent type | Payload type | Key field |
|---|---|---|---|---|
| `read` (success) | core | `FileOpened` | `FileOpenedPayload` | `path` ← `input.path` |
| `read` (failure) | core | `FailedLookup` | `FailedLookupPayload` | `subject` ← `input.path` |
| `bash` | core | `CommandExecuted` | `CommandExecutedPayload` | `command` ← `input.command` |
| `edit` | core | `EditMade` | `EditMadePayload` | `target` ← `input.path` |
| `write` | core | `EditMade` | `EditMadePayload` | `target` ← `input.path` |
| `grep` | core | `SearchRun` | `SearchRunPayload` | `query` ← `input.pattern` |
| `find` | core | `SearchRun` | `SearchRunPayload` | `query` ← `input.pattern` |
| `ast_grep_search` | `pi-lens` | `SearchRun` | `SearchRunPayload` | `query` ← `input.pattern` |

#### Scenario: Successful read maps to FileOpened with a normalized subject

- **WHEN** an agent calls `read` and it succeeds
- **THEN** the hook emits a `FileOpened` TraceEvent with `payload.path` in canonical repository-relative form
- **AND** `tool_name` is `"read"` and the outcome is `Success`

#### Scenario: Failed read maps to FailedLookup

- **WHEN** an agent calls `read` for a path that does not exist and `event.isError` is true
- **THEN** the hook emits a `FailedLookup` TraceEvent with `payload.subject` set to the attempted path
- **AND** it does NOT emit a `FileOpened` event with a failure outcome

#### Scenario: FileOpened never carries a failure outcome

- **WHEN** any Pi tool event is translated
- **THEN** no emitted `FileOpened` event carries `Outcome::Failure`

#### Scenario: A file's successful and failed reads share one grouping key

- **GIVEN** a successful `read` of a file addressed absolutely
- **AND** a failed `read` of the same file addressed relatively
- **WHEN** both are translated
- **THEN** both carry the same `subject` and the same `subject_kind`
- **AND** hotspot scoring groups them into a single entry whose `counts.eventType` includes both `FileOpened` and `FailedLookup`

#### Scenario: grep and find produce SearchRun from pattern

- **WHEN** an agent calls `grep` or `find` with a search pattern
- **THEN** the hook emits a `SearchRun` TraceEvent whose `payload.query` is the value of `input.pattern`
- **AND** `tool_name` is `"grep"` or `"find"` respectively

#### Scenario: ast_grep_search reads pattern, not query

- **WHEN** an agent calls `ast_grep_search`
- **THEN** the hook emits a `SearchRun` TraceEvent whose `payload.query` is the value of `input.pattern`
- **AND** a payload carrying only a `query` key is treated as a missing key field

#### Scenario: No SearchRun subject is ever a placeholder

- **WHEN** any mapped search tool is translated
- **THEN** `payload.query` is the actual pattern the agent supplied and is never `"unknown"`

#### Scenario: Missing key field drops the event instead of recording a placeholder

- **WHEN** a mapped Pi tool event arrives whose key input field is absent
- **THEN** no event is persisted and the contract violation is recorded in the warning log
- **AND** the hook exits 0 without modifying the agent-visible tool result

#### Scenario: A Pi-side field rename fails a test

- **GIVEN** a golden fixture captured from a real Pi payload for each mapped tool
- **WHEN** a mapped tool's key input field name changes
- **THEN** the fixture test fails rather than the adapter degrading to a dropped or placeholder subject

#### Scenario: edit and write map to EditMade with normalized targets

- **WHEN** an agent calls `edit` or `write`
- **THEN** the hook emits an `EditMade` TraceEvent with `payload.target` in canonical repository-relative form

#### Scenario: bash maps to CommandExecuted and is not path-normalized

- **WHEN** an agent calls `bash` and Bash capture is enabled
- **THEN** the hook emits a `CommandExecuted` TraceEvent with `payload.command` set to the command string verbatim
- **AND** the command string is not path-normalized
