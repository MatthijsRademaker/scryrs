# scryrs-record-endpoint Specification

## Purpose

Defines requirements for the scryrs record endpoint — the JSONL trace-event ingestion command covering stdin/file input modes, deterministic output contract, exit-code semantics, and minimal append-only event store persistence.
## Requirements
### Requirement: Record command accepts JSONL trace events from stdin and file

The system SHALL expose `scryrs record --stdin` and `scryrs record --file <PATH>` as the only public ingestion modes for this change. Both modes MUST read newline-delimited JSON using the existing `scryrs-types::TraceEvent` wire contract, use the same ingestion path, and reject invocations that provide both or neither input mode.

#### Scenario: Hook pipes events via stdin

- **WHEN** a hook runs `scryrs record --stdin` and writes newline-delimited `TraceEvent` JSON to stdin
- **THEN** the system ingests each non-empty line through the shared record path
- **THEN** accepted events are persisted

#### Scenario: Record reads events from file

- **WHEN** a user runs `scryrs record --file session.jsonl`
- **THEN** the system reads the file as newline-delimited `TraceEvent` JSON
- **THEN** the system uses the same validation, storage, summary, and exit-code behavior as `--stdin`

#### Scenario: Invalid input mode fails fast

- **WHEN** `scryrs record` is invoked with both `--stdin` and `--file`, or with neither
- **THEN** the system writes a usage error to stderr
- **THEN** the system exits with code 2

### Requirement: Accepted events are persisted through a versioned local SQLite datastore

The system SHALL persist each accepted `TraceEvent` through a core-owned SQLite trace datastore. The canonical local store SHALL be `.scryrs/scryrs.db` relative to the current working directory, and `.scryrs/events.jsonl` SHALL NOT remain the canonical persistence store.

#### Scenario: Valid event is inserted into the canonical datastore
- **WHEN** a valid record input is accepted and the default store is used
- **THEN** the system opens or creates `.scryrs/scryrs.db`
- **AND** the accepted event is inserted into the SQLite datastore
- **AND** `.scryrs/events.jsonl` is not used as the canonical accepted-event store

#### Scenario: Datastore schema ownership stays in scryrs-core
- **WHEN** the datastore is initialized
- **THEN** `scryrs-core` owns schema creation and compatibility validation
- **AND** `scryrs-cli` only composes the core datastore API
- **AND** the datastore tracks an independent schema version in `schema_meta` or an equivalent version table starting at integer `1`

#### Scenario: Stored rows preserve raw trace truth and normalized query fields
- **WHEN** an accepted `TraceEvent` is inserted
- **THEN** the row stores canonical JSON serialization of the validated event for auditability
- **AND** the row stores normalized values for `schema_version`, `timestamp`, `session_id`, `event_type`, `tool_name`, `subject_kind`, `subject`, `outcome`, and `failure_reason`
- **AND** `subject_kind` is derived from the concrete subject-bearing event family and is NULL for lifecycle events
- **AND** `subject` uses the existing TraceEvent subject extraction and is NULL for lifecycle events
- **AND** `failure_reason` stores `Outcome::Failure.reason` when present and is NULL otherwise

#### Scenario: Datastore indexes support hotspot-oriented filtering
- **WHEN** the SQLite schema is created
- **THEN** indexes exist for subject lookup via `subject_kind` and `subject`
- **AND** indexes exist for `event_type` filtering
- **AND** indexes exist for ordering by `session_id` and `timestamp`
- **AND** indexes exist for failure analysis using `outcome` and `failure_reason`

#### Scenario: Storage remains ingestion-only
- **WHEN** this change is implemented
- **THEN** the store surface only opens or creates the datastore, inserts accepted events, and reports stored counts needed by record ingestion
- **AND** the change does not add hotspot analysis, promotion logic, query APIs, hosted storage, legacy JSONL migration, or alternate canonical write paths

### Requirement: Validation rejects malformed non-empty lines without aborting ingestion

The system SHALL validate each non-empty physical line as a `TraceEvent`. Malformed JSON or schema-invalid events MUST be rejected with deterministic diagnostics containing the 1-based physical line number, the failing field/path when available, and a reason, while ingestion continues with later lines.

#### Scenario: Malformed JSON line is rejected

- **WHEN** a non-empty line contains invalid JSON
- **THEN** the system records a rejection for that 1-based physical line number
- **THEN** the rejection diagnostic includes the parse reason
- **THEN** the system continues processing subsequent lines

#### Scenario: Schema-invalid event is rejected

- **WHEN** a non-empty line parses as JSON but fails `TraceEvent` validation
- **THEN** the system records a rejection for that 1-based physical line number
- **THEN** the rejection diagnostic includes the failing field/path when available and a reason
- **THEN** the system continues processing subsequent lines

#### Scenario: Blank line is ignored

- **WHEN** a physical input line is empty or whitespace-only
- **THEN** the system skips that line
- **THEN** the line does not increment accepted or rejected counts

### Requirement: Record output and exit codes are deterministic

The `record` command SHALL emit exactly one JSON summary object to stdout with fields `command`, `schemaVersion`, `accepted`, and `rejected`. Rejection diagnostics MUST be emitted deterministically to stderr as one JSON object per rejected non-empty line containing `line`, `field` when available, and `reason`. Exit code `0` MUST mean all processed non-empty lines were accepted, exit code `1` MUST mean ingestion completed with one or more rejected events, and exit code `2` MUST mean fatal usage, input, or datastore failure.

#### Scenario: All events are valid
- **WHEN** every processed non-empty line is accepted
- **THEN** stdout contains a single JSON object with `command: record` and matching `accepted` and `rejected` counts
- **AND** stderr contains no rejection diagnostics
- **AND** the system exits with code `0`

#### Scenario: Some events are rejected
- **WHEN** at least one processed non-empty line is rejected and later lines still complete
- **THEN** stdout still contains one summary JSON object with final `accepted` and `rejected` counts
- **AND** stderr contains one rejection diagnostic object per rejected line
- **AND** the system exits with code `1`

#### Scenario: Fatal file or stream setup error
- **WHEN** the input file cannot be opened or another fatal record setup error occurs
- **THEN** the system writes the fatal error to stderr
- **AND** the system does not emit a success summary for partially unread input
- **AND** the system exits with code `2`

#### Scenario: Fatal datastore error fails fast
- **WHEN** SQLite open, write, or schema-compatibility validation fails during `scryrs record`
- **THEN** the system writes the fatal error to stderr
- **AND** the system does not emit a success summary for the failed operation
- **AND** the system exits with code `2`
- **AND** the system does not silently fall back to JSONL persistence

### Requirement: Discovery surfaces describe the record endpoint accurately

The CLI discovery surfaces SHALL document `record` as a first-class command. `scryrs --help`, `scryrs --help-json`, `README.md`, and the CLI contract note MUST describe `--stdin` and `--file <PATH>`, the deterministic summary contract, the command-specific `0/1/2` exit semantics, and `.scryrs/scryrs.db` as the canonical local trace store while keeping JSONL described only as an input format.

#### Scenario: Help text lists record
- **WHEN** `scryrs --help` is invoked after this change
- **THEN** the output lists `scryrs record --stdin` and `scryrs record --file <PATH>` alongside the existing `hotspots` placeholder
- **AND** the output describes the record command output and exit codes

#### Scenario: Help-json includes record metadata
- **WHEN** `scryrs --help-json` is invoked after this change
- **THEN** the surface document includes a `record` command entry with mutually exclusive `--stdin` and `--file` modes
- **AND** the document describes the summary JSON fields and the record exit-code contract
- **AND** the `surfaceVersion` reflects an additive minor bump from the prior surface

#### Scenario: Project docs distinguish input JSONL from canonical persistence
- **WHEN** a reader reviews the user and developer documentation for `scryrs record`
- **THEN** the docs describe JSONL as the accepted ingestion format
- **AND** the docs describe `.scryrs/scryrs.db` as the canonical persisted store
- **AND** the docs no longer describe `.scryrs/events.jsonl` as canonical persistence

### Requirement: Record remains ingestion-only

The `record` endpoint SHALL be limited to ingestion. This change MUST NOT trigger hotspot scoring, promotion logic, graph building, routing, LLM calls, or harness-specific IPC beyond JSONL over stdin or file.

#### Scenario: Record does not invoke analysis behavior

- **WHEN** `scryrs record` accepts or rejects events
- **THEN** it only validates, stores, summarizes, and reports diagnostics
- **THEN** no hotspot report or promotion output is produced

