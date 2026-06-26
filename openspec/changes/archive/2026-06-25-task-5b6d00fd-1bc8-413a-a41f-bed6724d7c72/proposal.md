## Why

The central ingest server already persists deduplicated trace events, but it does not maintain live hotspot state or signal history. Without an incremental accumulator updated in the same commit path as accepted ingest, every dashboard or agent consumer would need to rescan `server_trace_events` or rerun batch hotspot analysis after each message. This task closes that gap while keeping live scoring fully deterministic and aligned with existing `score_hotspots` semantics.

## What Changes

- Extract a shared per-event hotspot contribution API from `crates/scryrs-core/src/scoring.rs` so batch scoring and live accumulation use the same weight table and failure bonus.
- Bump the server store schema to v2 and add server-owned `hotspot_accumulators` and `hotspot_signals` tables in the server database only; keep the local `.scryrs/scryrs.db` schema and local `HotspotsReport` contract unchanged.
- Refactor accepted-event ingest so event insert, cumulative accumulator update, threshold-crossing detection, and optional signal insert commit atomically inside `ServerStore`. Duplicate, rejected, and lifecycle events must not mutate accumulator or signal state.
- Add a shared `HotspotSignal` type and a deterministic server `signal_threshold` configuration field with a default value suitable for tests and runtime use.
- Store cumulative live state only for this foundation, but carry a `window` field/value of `"cumulative"` in accumulator and signal records so future recent-window work can extend the model additively.
- Add server-side read/query helpers needed for tests and cumulative comparisons, plus batch-vs-live alignment coverage against existing `score_hotspots` / `scryrs hotspots` output. Do not add dashboard UI, recent-window accumulation, backfill, or new streaming behavior in this task.

## Impact

- Accepted subject-bearing server events immediately update server-authoritative live hotspot state.
- Threshold crossings persist append-only signal history with subject identity, score delta, window, threshold, and evidence references tied to server event rows.
- Existing server databases require an additive schema migration with no backfill; only events accepted after the upgrade contribute to the new accumulator rows.
- Local CLI recording, local hotspot reporting, dashboard UI, and previously defined read/stream transport follow-up work remain unchanged.
