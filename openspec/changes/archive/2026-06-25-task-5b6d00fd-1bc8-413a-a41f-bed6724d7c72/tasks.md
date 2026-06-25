## 1. Shared scoring and type contracts

- [x] 1.1 Extract a public per-event hotspot contribution API from `crates/scryrs-core/src/scoring.rs` and update batch scoring to use it.
- [x] 1.2 Add `HotspotSignal` to `crates/scryrs-types/src/lib.rs` with subject identity, score, delta, window, threshold, evidence references, and creation timestamp fields.
- [x] 1.3 Add `signal_threshold` to `crates/scryrs-server` configuration with a deterministic default and tests for the new type/config behavior.

## 2. Server-store schema and migration

- [x] 2.1 Bump the server store schema to v2 with an additive migration path for existing `server.db` files.
- [x] 2.2 Add a cumulative `hotspot_accumulators` table keyed by `(repository_id, window, subject_kind, subject)` and storing the aggregate state needed for cumulative hotspot materialization.
- [x] 2.3 Add an append-only `hotspot_signals` table stored separately from accumulators.
- [x] 2.4 Keep the local `.scryrs/scryrs.db` schema and local hotspot-report artifacts unchanged, and do not backfill historical `server_trace_events` rows.

## 3. Transactional live hotspot updates on ingest

- [x] 3.1 Refactor the accepted-event ingest path so the inserted event context is available for live hotspot updates without rescoring from scratch.
- [x] 3.2 Wrap the accepted event insert, accumulator mutation, threshold-crossing check, and optional signal insert in one explicit SQLite transaction inside `ServerStore`.
- [x] 3.3 Skip accumulator and signal mutation for duplicate, rejected, and lifecycle events.
- [x] 3.4 Apply only the cumulative `window = "cumulative"` live hotspot model in this task.

## 4. Read helpers and verification

- [x] 4.1 Add internal server-store query helpers for cumulative accumulator state and persisted signal history so tests can inspect live results without adding new HTTP read routes.
- [x] 4.2 Ensure any materialized evidence references are ordered by `timestamp ASC, id ASC` to match batch hotspot semantics.
- [x] 4.3 Add tests for first accepted updates, duplicate replay, mixed valid/rejected batches, lifecycle exclusion, failure-bonus scoring, threshold crossing, and no duplicate signal on replay.
- [x] 4.4 Add cumulative batch-vs-live alignment tests using the existing hotspot fixture/event-family coverage and compare scores and ranks with `score_hotspots` / `scryrs hotspots`.

## 5. Regression boundaries

- [x] 5.1 Re-run or extend targeted server and scoring tests to prove deterministic idempotency and migration behavior.
- [x] 5.2 Verify no dashboard frontend files, new HTTP read routes, or SSE fanout behavior are added in this task.
- [x] 5.3 Verify local recording flows and local hotspot-report behavior remain unchanged.
