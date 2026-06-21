## 1. Create the `query.rs` module

- [ ] 1.1 Create `crates/scryrs-core/src/query.rs` with module-level documentation describing the read-only trace query surface.
- [ ] 1.2 Define the `QueryError` enum with `MissingStore`, `EmptyStore`, `UnsupportedStore(String)`, and `StorageError(rusqlite::Error)` variants, implementing `std::fmt::Display`, `std::fmt::Debug`, and `std::error::Error`.
- [ ] 1.3 Define the `TraceQuery` struct owning a `rusqlite::Connection` opened read-only.

## 2. Implement `TraceQuery::open`

- [ ] 2.1 Implement `TraceQuery::open(repo_root: impl AsRef<Path>) -> Result<Self, QueryError>` that joins `repo_root` with `.scryrs/scryrs.db`.
- [ ] 2.2 Open the database with `rusqlite::Connection::open_with_flags` using `OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_CREATE`, mapping file-not-found to `QueryError::MissingStore`.
- [ ] 2.3 Validate the datastore schema version via read-only `SELECT` against `schema_meta`, mapping version mismatch or missing table to `QueryError::UnsupportedStore`.

## 3. Implement query methods

- [ ] 3.1 Implement `TraceQuery::iter_events_ordered(&self) -> Result<Vec<TraceEvent>, QueryError>` with `SELECT event_json FROM trace_events ORDER BY timestamp ASC, id ASC`, returning `EmptyStore` when zero rows are returned.
- [ ] 3.2 Implement `TraceQuery::query_by_subject_kind(&self, kind: &str) -> Result<Vec<TraceEvent>, QueryError>` with `WHERE subject_kind = ?1 ORDER BY timestamp ASC, id ASC`.
- [ ] 3.3 Implement `TraceQuery::query_by_event_type(&self, event_type: &str) -> Result<Vec<TraceEvent>, QueryError>` with `WHERE event_type = ?1 ORDER BY timestamp ASC, id ASC`.
- [ ] 3.4 Implement `TraceQuery::query_failures(&self) -> Result<Vec<TraceEvent>, QueryError>` with `WHERE outcome = 'Failure' ORDER BY timestamp ASC, id ASC`.
- [ ] 3.5 Implement shared row-to-`TraceEvent` deserialization via `serde_json::from_str` on the `event_json` column, failing the entire query on deserialization failure.

## 4. Re-export from crate root

- [ ] 4.1 Add `pub mod query;` to `crates/scryrs-core/src/lib.rs`.
- [ ] 4.2 Add `pub use query::{QueryError, TraceQuery};` to the existing re-export block in `crates/scryrs-core/src/lib.rs`.

## 5. Add comprehensive tests

- [ ] 5.1 Add test: open a database created by `EventStore` (write path) via `TraceQuery` and verify all rows are visible with correct normalized columns.
- [ ] 5.2 Add test: deterministic ordering with events sharing the same timestamp but different `id` values.
- [ ] 5.3 Add test: `query_by_subject_kind` correctly filters across all seven subject-bearing families (file, symbol, search, command, document).
- [ ] 5.4 Add test: `query_by_event_type` correctly filters by event type string.
- [ ] 5.5 Add test: `query_failures` returns only `Outcome::Failure` events and excludes success events.
- [ ] 5.6 Add test: `MissingStore` is returned when `.scryrs/scryrs.db` does not exist at the given path.
- [ ] 5.7 Add test: `EmptyStore` is returned from query methods when `trace_events` has zero rows but the database is valid.
- [ ] 5.8 Add test: `UnsupportedStore` is returned when `schema_meta.datastore_schema_version` does not match.
- [ ] 5.9 Add test: `StorageError` (or appropriate error) is returned when the file exists but is not a valid SQLite database (e.g., a text file renamed to `.scryrs.db`).
- [ ] 5.10 Add test: read-only connection to WAL-mode database written by `EventStore` correctly reads all committed rows.

## 6. Verify the workspace

- [ ] 6.1 Run `cargo test -p scryrs-core` and confirm all new and existing tests pass.
- [ ] 6.2 Run Docker-backed `scripts/test` and confirm full workspace health.
- [ ] 6.3 Run Docker-backed `scripts/check` (clippy, formatting) and confirm no new warnings.
- [ ] 6.4 Confirm `scryrs-cli` compiles without requiring changes (placeholder hotspot output is preserved).