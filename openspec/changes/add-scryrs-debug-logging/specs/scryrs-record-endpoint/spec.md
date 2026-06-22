## ADDED Requirements

### Requirement: Record debug logging is opt-in through SCRYRS_DEBUG

The `scryrs record` command SHALL emit additional diagnostic logs only when `SCRYRS_DEBUG` is set to a non-empty value. With `SCRYRS_DEBUG` unset, `record` SHALL preserve its existing stdout summary, stderr rejection JSONL, exit codes, validation behavior, and persistence behavior.

#### Scenario: Debug disabled preserves record output contract

- **WHEN** `scryrs record --stdin` ingests valid or invalid JSONL with `SCRYRS_DEBUG` unset
- **THEN** stdout contains exactly the normal JSON summary object
- **AND** stderr contains only the normal rejection diagnostics or fatal errors already required by the record contract
- **AND** no `[scryrs-record]` debug lines are emitted

#### Scenario: Debug enabled emits record prefix lines

- **WHEN** `scryrs record --stdin` runs with `SCRYRS_DEBUG` set
- **THEN** additional debug diagnostics are emitted to stderr with the stable `[scryrs-record]` prefix
- **AND** stdout still contains the normal JSON summary object

### Requirement: Record debug logs expose ingestion and persistence stages

When debug logging is enabled, `scryrs record` SHALL emit single-line breadcrumbs for input receipt, accepted event parsing, rejected line diagnostics, datastore open, accepted event insertion, transaction completion, and final summary. Debug logs SHALL collapse or truncate multi-line values so each debug record remains one physical line.

#### Scenario: Received line debug breadcrumb is emitted

- **WHEN** `scryrs record --stdin` receives a non-empty input line with `SCRYRS_DEBUG` set
- **THEN** stderr includes a `[scryrs-record]` debug line identifying the line number and byte length
- **AND** any raw preview is bounded and single-line

#### Scenario: Accepted event debug breadcrumb is emitted

- **WHEN** `scryrs record --stdin` accepts a TraceEvent line with `SCRYRS_DEBUG` set
- **THEN** stderr includes a `[scryrs-record]` debug line identifying the accepted line number, event type, session id, and tool name when present

#### Scenario: Rejected event debug breadcrumb is emitted

- **WHEN** `scryrs record --stdin` rejects a non-empty line with `SCRYRS_DEBUG` set
- **THEN** stderr includes a `[scryrs-record]` debug line identifying the rejected line number, field when available, and reason
- **AND** the normal rejection JSON diagnostic is still emitted according to the existing record contract

#### Scenario: Persistence debug breadcrumbs are emitted

- **WHEN** accepted events are persisted with `SCRYRS_DEBUG` set
- **THEN** stderr includes `[scryrs-record]` debug lines for datastore open and insertion of accepted events
- **AND** the insertion debug line identifies the event type, session id, and tool name when present

#### Scenario: Summary debug breadcrumb is emitted

- **WHEN** `scryrs record` finishes with `SCRYRS_DEBUG` set
- **THEN** stderr includes a `[scryrs-record]` debug line with accepted count, rejected count, and intended exit category
- **AND** the process exit code remains the existing record exit code for the same input

### Requirement: Record debug logging is safe and bounded

Record debug logs SHALL avoid unbounded raw JSONL dumps by default. Any preview of raw input or serialized event data SHALL be truncated, have embedded newlines collapsed, and indicate truncation or byte length. Debug logging SHALL NOT add new datastore rows, alter transaction boundaries, or change accepted/rejected decisions.

#### Scenario: Large input line is bounded in debug output

- **WHEN** `scryrs record --stdin` receives a large non-empty line with `SCRYRS_DEBUG` set
- **THEN** any debug preview is truncated to a bounded length
- **AND** the debug line still includes the original byte length or an equivalent truncation indicator

#### Scenario: Debug logging does not alter persistence

- **WHEN** the same JSONL input is ingested once with `SCRYRS_DEBUG` unset and once with `SCRYRS_DEBUG` set
- **THEN** the accepted and rejected counts are identical
- **AND** the persisted accepted TraceEvents are equivalent apart from normal run-specific datastore row identifiers
