# scryrs-record-endpoint Specification

## MODIFIED Requirements

### Requirement: Accepted events are persisted through a versioned local SQLite datastore

The system SHALL persist each accepted `TraceEvent` through a core-owned SQLite trace datastore. The canonical local store SHALL be `.scryrs/scryrs.db` relative to the current working directory, and `.scryrs/events.jsonl` SHALL NOT remain the canonical persistence store. Accepted events from a single `scryrs record` invocation SHALL be inserted through one explicit transaction or equivalent batch boundary, and the command SHALL report success only after that commit succeeds.

#### Scenario: Valid event is inserted into the canonical datastore
- **WHEN** a valid record input is accepted and the default store is used
- **THEN** the system opens or creates `.scryrs/scryrs.db`
- **AND** the accepted event is inserted into the SQLite datastore
- **AND** `.scryrs/events.jsonl` is not used as the canonical accepted-event store

#### Scenario: One invocation commits accepted events before success
- **WHEN** one `scryrs record` invocation accepts multiple events
- **THEN** the system persists those accepted events through one explicit SQLite transaction or equivalent batch boundary
- **AND** the command does not rely on one SQLite autocommit per accepted line
- **AND** the command reports success only after the batch commit succeeds

#### Scenario: Rejected lines never create datastore rows
- **WHEN** record input contains a mix of accepted and rejected non-empty lines
- **THEN** only the accepted events are inserted into `trace_events`
- **AND** no rejected line creates an event row in the canonical datastore

#### Scenario: Datastore schema ownership stays in scryrs-core
- **WHEN** the datastore is initialized
- **THEN** `scryrs-core` owns schema creation and compatibility validation
- **AND** `scryrs-cli` only composes the core datastore API
- **AND** the datastore tracks an independent schema version in `schema_meta` or an equivalent version table starting at integer `1`

### Requirement: Record output and exit codes are deterministic

The `record` command SHALL emit exactly one JSON summary object to stdout with fields `command`, `schemaVersion`, `accepted`, and `rejected`. Rejection diagnostics MUST be emitted deterministically to stderr as one JSON object per rejected non-empty line containing `line`, `field` when available, and `reason`. Exit code `0` MUST mean all processed non-empty lines were accepted, exit code `1` MUST mean ingestion completed with one or more rejected events, and exit code `2` MUST mean fatal usage, input, or datastore failure.

#### Scenario: Some events are rejected
- **WHEN** at least one processed non-empty line is rejected and later lines still complete
- **THEN** stdout still contains one summary JSON object with final `accepted` and `rejected` counts
- **AND** stderr contains one rejection diagnostic object per rejected line
- **AND** only accepted events are persisted
- **AND** the system exits with code `1`

#### Scenario: Fatal datastore error fails fast
- **WHEN** SQLite open, initialization, write, or commit fails during `scryrs record`
- **THEN** the system writes the fatal error to stderr
- **AND** the system does not emit a success summary for the failed operation
- **AND** the system exits with code `2`
- **AND** the system does not silently fall back to JSONL persistence

### Requirement: Discovery surfaces describe the record endpoint accurately

The CLI discovery surfaces SHALL document `record` as a first-class command. `scryrs --help`, `scryrs --help-json`, `README.md`, and the CLI contract note MUST describe `--stdin` and `--file <PATH>`, the deterministic summary contract, the command-specific `0/1/2` exit semantics, and `.scryrs/scryrs.db` as the canonical local trace store while keeping JSONL described only as an input format.

#### Scenario: Plain help and README document fatal store failure
- **WHEN** a reader checks `scryrs --help` or `README.md` for record exit semantics
- **THEN** exit code `2` is described as covering fatal datastore failure as well as the existing fatal record setup errors

#### Scenario: Project docs distinguish input JSONL from canonical persistence
- **WHEN** a reader reviews the user and developer documentation for `scryrs record`
- **THEN** the docs describe JSONL as the accepted ingestion format
- **AND** the docs describe `.scryrs/scryrs.db` as the canonical persisted store
- **AND** the docs no longer describe `.scryrs/events.jsonl` as canonical persistence
