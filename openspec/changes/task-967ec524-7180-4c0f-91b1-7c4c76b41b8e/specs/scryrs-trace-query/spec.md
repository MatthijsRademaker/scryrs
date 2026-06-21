# scryrs-trace-query Specification

## ADDED Requirements

### Requirement: TraceQuery opens an existing scryrs datastore without mutation

The system SHALL expose `TraceQuery::open(repo_root)` in `scryrs-core` that resolves `<repo_root>/.scryrs/scryrs.db` via path join without upward-walking heuristics. The open operation SHALL use `rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_NO_CREATE` to guarantee the database is never created, never written to, and never has WAL mode or journal pragma applied. Schema validation SHALL be performed through read-only SQL `SELECT` queries without executing any DDL (`CREATE`, `ALTER`, `INSERT`, `PRAGMA journal_mode`).

#### Scenario: Existing valid store is opened successfully

- **GIVEN** `.scryrs/scryrs.db` exists at `<repo_root>/.scryrs/scryrs.db` with `schema_meta.datastore_schema_version = 1` and a valid `trace_events` table
- **WHEN** `TraceQuery::open(repo_root)` is called
- **THEN** the database is opened read-only via `OpenFlags::SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_NO_CREATE`
- **AND** the schema version is validated via `SELECT value FROM schema_meta WHERE key = 'datastore_schema_version'`
- **AND** no directories are created
- **AND** no `PRAGMA journal_mode=WAL` is executed
- **AND** no tables or indexes are created or altered
- **AND** a valid `TraceQuery` handle is returned

#### Scenario: Missing store returns explicit error

- **GIVEN** no `.scryrs/scryrs.db` exists at `<repo_root>/.scryrs/scryrs.db`
- **WHEN** `TraceQuery::open(repo_root)` is called
- **THEN** the system returns `QueryError::MissingStore`
- **AND** no file is created at the target path
- **AND** no directory `.scryrs` is created

#### Scenario: Unsupported schema version returns explicit error

- **GIVEN** `.scryrs/scryrs.db` exists with `schema_meta.datastore_schema_version` set to a value other than `1`
- **WHEN** `TraceQuery::open(repo_root)` is called
- **THEN** the system returns `QueryError::UnsupportedStore` with a message describing the version mismatch
- **AND** the database is not mutated

#### Scenario: Non-SQLite file returns explicit error

- **GIVEN** a regular file exists at `<repo_root>/.scryrs/scryrs.db` that is not a valid SQLite database
- **WHEN** `TraceQuery::open(repo_root)` is called
- **THEN** the system returns `QueryError::StorageError` wrapping the underlying `rusqlite` error
- **AND** the file is not modified

### Requirement: Query methods return events in deterministic order

All query methods on `TraceQuery` SHALL return `Vec<TraceEvent>` sorted by `ORDER BY timestamp ASC, id ASC`, where `id` is the `INTEGER PRIMARY KEY AUTOINCREMENT` column serving as a tie-breaker for events with identical timestamps. Repeated queries over the same unchanged database SHALL return events in the same order.

#### Scenario: Events are ordered by timestamp then id

- **GIVEN** `trace_events` contains three events: `E1` with timestamp `T1` and id `1`, `E2` with timestamp `T1` and id `3`, `E3` with timestamp `T2` and id `2`
- **WHEN** any query method returning events is called
- **THEN** events are returned in order `E1`, `E2`, `E3` (timestamp ascending, then id ascending)
- **AND** repeated identical queries return the same ordering

#### Scenario: Events with identical timestamps are deterministically ordered by id

- **GIVEN** `trace_events` contains two events both with the same `timestamp` but different `id` values
- **WHEN** any query method returning events is called
- **THEN** the event with the lower `id` value appears first
- **AND** the ordering is stable across repeated queries

### Requirement: Subject-kind query filters by normalized subject category

The system SHALL expose `TraceQuery::query_by_subject_kind(kind: &str)` that returns only events whose `subject_kind` column matches the provided kind string. The query SHALL use the existing `idx_trace_events_subject` index on `(subject_kind, subject)` and SHALL return results in deterministic `timestamp ASC, id ASC` order.

#### Scenario: Filter returns only matching subject-kind events

- **GIVEN** `trace_events` contains file, search, and command events
- **WHEN** `query_by_subject_kind("file")` is called
- **THEN** only events with `subject_kind = 'file'` are returned, including both `FileOpened` and `EditMade` events
- **AND** search events (`subject_kind = 'search'`) and command events (`subject_kind = 'command'`) are excluded

#### Scenario: Filter by search kind returns search events

- **GIVEN** `trace_events` contains events of various kinds
- **WHEN** `query_by_subject_kind("search")` is called
- **THEN** only events with `subject_kind = 'search'` are returned
- **AND** lifecycle events (which have `subject_kind IS NULL`) are excluded

#### Scenario: Kind with no matching events returns empty

- **GIVEN** `trace_events` contains events but none with `subject_kind = 'symbol'`
- **WHEN** `query_by_subject_kind("symbol")` is called
- **THEN** `QueryError::EmptyStore` is returned

### Requirement: Event-type query filters by event type string

The system SHALL expose `TraceQuery::query_by_event_type(event_type: &str)` that returns only events whose `event_type` column matches the provided string. The query SHALL use the existing `idx_trace_events_event_type` index and SHALL return results in deterministic `timestamp ASC, id ASC` order.

#### Scenario: Filter returns only matching event-type events

- **GIVEN** `trace_events` contains events of types `FileOpened`, `SearchRun`, and `SessionStart`
- **WHEN** `query_by_event_type("FileOpened")` is called
- **THEN** only events with `event_type = 'FileOpened'` are returned
- **AND** `SearchRun` and `SessionStart` events are excluded

#### Scenario: Type with no matching events returns empty

- **GIVEN** `trace_events` contains no `DocRetrieved` events
- **WHEN** `query_by_event_type("DocRetrieved")` is called
- **THEN** `QueryError::EmptyStore` is returned

### Requirement: Failure query returns only failed events

The system SHALL expose `TraceQuery::query_failures()` that returns only events where `outcome = 'Failure'`. The query SHALL use the existing `idx_trace_events_outcome_reason` index on `(outcome, failure_reason)` and SHALL return results in deterministic `timestamp ASC, id ASC` order. Each returned event's `Outcome::Failure` payload SHALL include the `failure_reason` column value when available.

#### Scenario: Failure query returns only failure events

- **GIVEN** `trace_events` contains three events: two with `outcome = 'Success'` and one with `outcome = 'Failure'` and `failure_reason = 'exit code 1'`
- **WHEN** `query_failures()` is called
- **THEN** only the failure event is returned
- **AND** the success events are excluded

#### Scenario: Failure query with no failures returns empty

- **GIVEN** `trace_events` contains only success events
- **WHEN** `query_failures()` is called
- **THEN** `QueryError::EmptyStore` is returned

### Requirement: Empty store is distinguishable from missing store

The system SHALL distinguish between a missing database file (`QueryError::MissingStore`) and an existing, valid database whose `trace_events` table contains zero rows (`QueryError::EmptyStore`). `EmptyStore` SHALL be returned from query methods when the connection is valid but the result set has zero rows, not from `TraceQuery::open`.

#### Scenario: Existing empty store returns EmptyStore from queries

- **GIVEN** `.scryrs/scryrs.db` exists with valid schema and `trace_events` table but zero rows
- **WHEN** `iter_events_ordered()`, `query_by_subject_kind(...)`, `query_by_event_type(...)`, or `query_failures()` is called
- **THEN** `QueryError::EmptyStore` is returned from each query method
- **AND** the `TraceQuery` handle remains usable for subsequent queries

#### Scenario: Missing store returns MissingStore from open

- **GIVEN** no `.scryrs/scryrs.db` exists at the target path
- **WHEN** `TraceQuery::open(repo_root)` is called
- **THEN** `QueryError::MissingStore` is returned
- **AND** no `TraceQuery` handle is created

### Requirement: TraceQuery and QueryError are re-exported from scryrs-core

The system SHALL re-export `TraceQuery` and `QueryError` from `crates/scryrs-core/src/lib.rs` alongside the existing `EventStore` and `CANONICAL_STORE_PATH` re-exports, making the read API available to `scryrs-cli` and future consumers without requiring a direct dependency on the `query` module path.

#### Scenario: CLI can import TraceQuery from scryrs-core

- **WHEN** a consumer writes `use scryrs_core::TraceQuery;`
- **THEN** the import resolves through the crate root re-export
- **AND** the consumer can call `TraceQuery::open(repo_root)`

#### Scenario: QueryError is importable from crate root

- **WHEN** a consumer writes `use scryrs_core::QueryError;`
- **THEN** all four variants (`MissingStore`, `EmptyStore`, `UnsupportedStore`, `StorageError`) are accessible

### Requirement: No JSONL fallback path exists

The read path SHALL exclusively read from `.scryrs/scryrs.db`. The system SHALL NOT fall back to reading `.scryrs/events.jsonl` or any other JSONL file when the SQLite store is unavailable, empty, or malformed. The `TraceQuery` API SHALL NOT expose any JSONL-parsing functionality.

#### Scenario: Missing store does not trigger JSONL read

- **GIVEN** `query_by_subject_kind` on a repository with no `.scryrs/scryrs.db` returns `MissingStore`
- **WHEN** the consumer inspects the result
- **THEN** no attempt was made to open or parse `.scryrs/events.jsonl`
- **AND** no JSONL content is read or returned

### Requirement: CLI hotspot command remains placeholder

The `scryrs hotspots <PATH>` command SHALL continue to emit the existing placeholder JSON envelope regardless of whether a readable `.scryrs/scryrs.db` exists. The command SHALL NOT call `TraceQuery`, and its output SHALL NOT depend on persisted trace data in this task.

#### Scenario: Hotspot command still returns placeholder

- **WHEN** `scryrs hotspots <PATH>` is invoked in a directory with a populated `.scryrs/scryrs.db`
- **THEN** the output is `{"schemaVersion":"0.1.0","command":"hotspots","status":"placeholder"}`
- **AND** no events are read from the database

#### Scenario: Hotspot output is independent of store state

- **WHEN** `scryrs hotspots <PATH>` is invoked in a directory with no `.scryrs/scryrs.db`
- **THEN** the output is identical to the populated-store case
- **AND** the exit code is `0`