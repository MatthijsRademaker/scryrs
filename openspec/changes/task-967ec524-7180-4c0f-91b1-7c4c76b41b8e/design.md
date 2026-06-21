## Context

The task requires a deterministic read path over the SQLite trace datastore created by Phase 1's `EventStore`. The existing `EventStore::open` creates parent directories, opens read-write, enables WAL via `PRAGMA journal_mode=WAL`, and calls `ensure_schema` which creates tables and indexes if absent (including an `INSERT OR REPLACE` into `schema_meta`). None of these side effects are acceptable for a read-only hotspot query — the read path must never fabricate directories, initialize schema, or alter WAL/journal state.

The `trace_events` table already has four indexes covering exactly the query dimensions this task requires: `idx_trace_events_subject` on `(subject_kind, subject)`, `idx_trace_events_event_type` on `(event_type)`, `idx_trace_events_session_ts` on `(session_id, timestamp)`, and `idx_trace_events_outcome_reason` on `(outcome, failure_reason)`. The `id` column is `INTEGER PRIMARY KEY AUTOINCREMENT`, providing the tie-breaker for deterministic timestamp ordering.

The `rusqlite` crate version 0.31 with `bundled` feature is already a dependency, supporting `OpenFlags::SQLITE_OPEN_READ_ONLY` and `OpenFlags::SQLITE_OPEN_NO_CREATE` for guaranteed non-mutating opens.

## Goals

- Add a read-only, non-mutating query surface in `scryrs-core` over the existing normalized `trace_events` columns.
- Return persisted events in deterministic `ORDER BY timestamp ASC, id ASC` order.
- Expose filter primitives for `subject_kind` (file, symbol, search, command, document), `event_type`, and outcome-based failure queries.
- Model missing store, empty store, and malformed/unsupported database as distinct, testable outcomes.
- Keep all read logic in `scryrs-core` (not adapters/runtime), architected as a separate type from `EventStore`.

## Non-Goals

- Do not implement hotspot ranking rules or final Phase 2 hotspot output — this task is foundation-only.
- Do not move datastore read logic into adapters, runtime layers, or `scryrs-cli`.
- Do not add or preserve `.scryrs/events.jsonl` fallback or migration behavior.
- Do not change the trace-event schema, hook capture contract, or `EventStore` write path.
- Do not add graph, proposal, adapter, dashboard, or LLM features.
- Do not wire CLI `hotspots` output — the placeholder envelope is preserved for a follow-up materialization task.

## Decisions

### D1. Separate `query.rs` module with `TraceQuery` type

Read logic lives in a new `crates/scryrs-core/src/query.rs` module with its own `TraceQuery` struct, not as methods on `EventStore`. This maintains single-responsibility separation: `EventStore` owns write/append/schema creation, `TraceQuery` owns read/query/filter. A separate module prevents accidental coupling and keeps the ~300-line query surface from bloating `store.rs`.

### D2. Open with read-only, no-create flags

`TraceQuery::open()` uses `rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_NO_CREATE`. `READ_ONLY` prevents any write at the SQLite level; `NO_CREATE` prevents file creation if the path does not exist. This is a stronger guarantee than `READ_ONLY` alone and eliminates the TOCTOU risk of separate `Path::exists()` / `open()` calls (acceptable for a non-concurrent local CLI use case).

### D3. Schema validation via read-only SELECT

On open, `TraceQuery` queries `schema_meta` for `datastore_schema_version` via `SELECT`. If the table is missing, the query fails (no `schema_meta` exists) and the error is mapped to `QueryError::UnsupportedStore`. If the version does not match `DATASTORE_SCHEMA_VERSION` (currently `1`), it returns `QueryError::UnsupportedStore`. At no point does the read path execute `CREATE TABLE`, `INSERT`, or `PRAGMA journal_mode=WAL`.

### D4. Single `QueryError` enum for all error states

A single `QueryError` enum covers all distinguishable outcomes: `MissingStore` (file absent), `EmptyStore` (file exists, valid schema, zero rows in `trace_events` — returned from query methods, not `open`), `UnsupportedStore(String)` (schema version mismatch or missing schema tables), and `StorageError(rusqlite::Error)` (corrupt file, permissions, or other SQLite-level failures). This avoids the complexity of separate success/error enums while still giving callers distinct branches.

### D5. Materialized `Vec<TraceEvent>` return type

All query methods return `Vec<TraceEvent>`, materialized via `rusqlite::query_map` and `serde_json::from_str` on the `event_json` column. This matches the existing `score_events(&[TraceEvent])` signature, avoids lifetime/ownership complexity of streaming iterators, and keeps the API simple. Streaming can be added as a performance optimization in a follow-up without breaking the contract.

### D6. Deterministic ordering with id tie-breaker

`iter_events_ordered()` queries with `ORDER BY timestamp ASC, id ASC`. The `id` column is `INTEGER PRIMARY KEY AUTOINCREMENT` and serves as a stable tie-breaker when multiple events share the same timestamp. Since all rows come from already-accepted events, every row is valid by construction — no re-validation of `event_json` is needed.

### D7. Query methods leverage existing indexes

`query_by_subject_kind(kind: &str)` uses `WHERE subject_kind = ?1` (indexed by `idx_trace_events_subject`). `query_by_event_type(event_type: &str)` uses `WHERE event_type = ?1` (indexed by `idx_trace_events_event_type`). `query_failures()` uses `WHERE outcome = 'Failure'` (indexed by `idx_trace_events_outcome_reason`). No new indexes are created.

### D8. Caller-responsible path resolution

`TraceQuery::open(repo_root)` joins `repo_root` with `.scryrs/scryrs.db` via `Path::join`. No upward-walking, no marker detection, no heuristics. The caller is responsible for providing a valid repository root. This matches the CLI contract where `hotspots <PATH>` documents PATH as "Path to the repository root directory".

### D9. CLI stays placeholder

The `scryrs hotspots <PATH>` command continues to return the placeholder envelope `{"schemaVersion":"0.1.0","command":"hotspots","status":"placeholder"}`. Wiring real output is deferred to a follow-up materialization task that consumes `TraceQuery`.

## Risks

- **Read-only open of WAL-mode database**: SQLite handles this correctly (readers see WAL pages transparently), but the interaction must be explicitly tested with a database written by the existing `EventStore` write path, verifying all rows are visible and no schema/data loss occurs.
- **Per-row `event_json` deserialization failure**: stored JSON could theoretically corrupt. Each row should fail the entire query if deserialization fails (fail-fast), since all rows are produced by the validated write path.
- **TOCTOU between check and open**: acceptable for a non-concurrent local CLI tool. The `NO_CREATE` flag provides an atomic open-or-fail, and the read-only flag prevents any write. Document as an explicit limitation.
- **Schema migration in future versions**: if `DATASTORE_SCHEMA_VERSION` is bumped, `TraceQuery` will reject existing stores with `QueryError::UnsupportedStore`. A future migration task must handle this.

## Traceability

- Task `967ec524-7180-4c0f-91b1-7c4c76b41b8e`
- Dossier `2026-06-21T09:48:41.498Z`
- Accepted decisions `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, `1-swarm-reviewer-recommendation`
- Round outputs `round:1:agent:swarm-architect`, `round:1:agent:swarm-lead-dev`, `round:1:agent:swarm-reviewer`
- Artifact base `initial`

## Conflict Resolution

- **Type name**: resolved as `TraceQuery` (architect's recommendation) in a new `query.rs` module, rather than `StoreReader` (lead-dev's recommendation) in `store.rs`. The name `TraceQuery` better conveys the purpose (querying traces, not reading a store), and a separate module avoids bloating `store.rs` and prevents accidental coupling with write-path internals.
- **Error modeling**: resolved as a single `QueryError` enum (architect's approach) rather than separate `OpenResult` + `ReadError` enums (lead-dev's approach). A single enum eliminates type proliferation while still giving callers distinct branches for `MissingStore`, `EmptyStore`, `UnsupportedStore`, and `StorageError`.
- **Open flags**: resolved as `SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_NO_CREATE` (lead-dev + reviewer's stronger guarantee) rather than `READ_ONLY` alone (architect). `NO_CREATE` provides an atomic open-or-fail without relying on a separate `Path::exists()` pre-check.
- **Query method surface**: includes `query_by_event_type` (lead-dev's suggestion) alongside the architect's `query_by_subject_kind` and `query_failures`, since the `idx_trace_events_event_type` index already exists and the method costs little to add.
- **`EmptyStore` return point**: returned from query methods (e.g., `iter_events_ordered` returns `Err(QueryError::EmptyStore)` when `trace_events` has zero rows), not from `open`. This follows the lead-dev's model where the open succeeds but queries report emptiness. The architect's design placed it alongside `MissingStore` at open time; placing it at query time is more natural since the store is valid and openable — it just has no data.