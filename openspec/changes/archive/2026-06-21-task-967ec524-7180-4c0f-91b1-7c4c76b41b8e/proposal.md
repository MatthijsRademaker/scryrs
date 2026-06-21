## Why

Phase 1 delivered a deterministic SQLite write path through `EventStore`, but hotspot analysis still has no way to read persisted evidence from that store. `scryrs-core` currently exposes only ingestion plus a slice-based `score_events` helper, while `scryrs-cli` still returns a placeholder for `hotspots`. The only datastore API today (`EventStore::open`) creates directories, opens the database read-write, enables WAL, and initializes schema — it is the wrong primitive for hotspot reads because it can fabricate or mutate state instead of reporting explicit missing/empty/no-data conditions.

This task is the first Phase 2 foundation step: add a core-owned SQLite read/query path over normalized `trace_events` rows so later hotspot analysis can consume real sessions without reparsing JSONL files.

## What Changes

1. **Add `query.rs` module to `scryrs-core`** with a public `TraceQuery` type that owns a read-only SQLite connection and exposes deterministic query methods over the existing normalized `trace_events` columns.

2. **Define `QueryError` enum** with explicit variants for `MissingStore` (file does not exist), `EmptyStore` (valid database with zero `trace_events` rows), `UnsupportedStore(String)` (schema version mismatch or missing schema tables), and `StorageError(rusqlite::Error)` (file exists but is not a valid SQLite database or other I/O failure).

3. **Implement `TraceQuery::open(repo_root: impl AsRef<Path>)`** that resolves `<repo_root>/.scryrs/scryrs.db` via `Path::join`, opens with `rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_NO_CREATE` to guarantee no directory creation, no schema initialization, and no mutation, then validates the datastore schema version through a read-only `SELECT` against `schema_meta`.

4. **Expose query methods returning `Vec<TraceEvent>`**: `iter_events_ordered()` with `ORDER BY timestamp ASC, id ASC` for deterministic ordering with tie-breaking; `query_by_subject_kind(kind: &str)` using the existing `idx_trace_events_subject` index; `query_by_event_type(event_type: &str)` using the existing `idx_trace_events_event_type` index; and `query_failures()` filtering `WHERE outcome = 'Failure'` using the existing `idx_trace_events_outcome_reason` index.

5. **Re-export `TraceQuery` and `QueryError`** from `crates/scryrs-core/src/lib.rs` alongside the existing `EventStore` re-exports, making them available to the CLI and future consumers.

6. **Add comprehensive tests** covering deterministic ordering with same-timestamp tie-breaking, subject-kind filtering across all seven subject-bearing families, event-type filtering, failure queries, missing store, empty store, schema version mismatch, non-SQLite file, and read-only open of a database written by the existing write path.

## Impact

- **Code changes** are localized to two files: a new `crates/scryrs-core/src/query.rs` module (~300 lines with tests) and a one-line re-export addition to `crates/scryrs-core/src/lib.rs`. No changes to `store.rs`, `scryrs-types`, `scryrs-cli`, or any other crate.

- **Hotspot analysis** gains the ability to query persisted evidence through `TraceQuery`, but ranking logic (`score_events`) and CLI output remain unchanged — the hotspot command still returns the placeholder envelope until a follow-up materialization task wires real output.

- **No schema changes**: the `trace_events` table and all four indexes created by Phase 1 remain exactly as-is. The read path queries them without modification.

- **No mutation risk**: `QueryError::EmptyStore` is returned when `trace_events` has zero rows after a successful open+validation, giving callers a distinguishable signal from `MissingStore` (file absent) and `UnsupportedStore` (schema mismatch).

- **Verification** expands test coverage to include read-path failure modes (missing, empty, malformed) that the existing write-path tests do not cover.