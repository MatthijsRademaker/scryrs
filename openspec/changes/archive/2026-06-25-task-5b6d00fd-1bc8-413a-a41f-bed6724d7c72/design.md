## Context

The repository already has deterministic batch hotspot scoring in `scryrs-core` and a central ingest server that stores deduplicated events in a dedicated SQLite database. What is missing is live, materialized hotspot state owned by the server. This task adds that ingest-time foundation only: accepted subject-bearing events must update cumulative hotspot state immediately, threshold crossings must be persisted separately as signals, and live scoring must stay aligned with the existing deterministic batch rules.

## Goals / Non-Goals

**Goals**

- Update accepted subject-bearing server ingest so event persistence and cumulative hotspot mutation happen atomically.
- Reuse the existing deterministic hotspot scoring semantics for live accumulation, including base weights, failure bonus, subject grouping, lifecycle exclusion, counts, sessions, and first/last seen semantics.
- Persist threshold-crossing `HotspotSignal` history separately from accumulator state.
- Preserve idempotency so duplicate replay does not change accumulator rows or emit duplicate signals.
- Add cumulative batch-vs-live alignment tests against `score_hotspots` / `scryrs hotspots`.

**Non-Goals**

- Dashboard UI, visualization, or client-side streaming behavior.
- Recent-window accumulation beyond a cumulative foundation.
- Backfilling pre-upgrade `server_trace_events` rows into new accumulator tables.
- Changing the inner `TraceEvent` schema, local `.scryrs/scryrs.db` schema, or local `HotspotsReport` contract.
- New HTTP read routes or SSE fanout behavior in this task.

## Decisions

### Decision 1: Share per-event contribution logic from `scryrs-core`

Live accumulation must not duplicate the weight table from batch scoring. Extract a public deterministic per-event hotspot contribution API from `crates/scryrs-core/src/scoring.rs` and route both batch scoring and server-side live accumulation through it so `base_weight` and failure-bonus semantics cannot drift.

### Decision 2: Add server-store schema v2 with separate accumulator and signal tables

Add `hotspot_accumulators` and `hotspot_signals` to the server-owned SQLite database only. The accumulator key is `(repository_id, window, subject_kind, subject)` with `window = "cumulative"` for this task. Accumulator state must retain the aggregate fields needed to materialize cumulative hotspot entries deterministically: score, per-event-type counts, per-outcome counts, distinct-session state, `first_seen`, and `last_seen`. `hotspot_signals` remains append-only and separate from accumulator rows.

### Decision 3: Make accepted ingest updates atomic inside `ServerStore`

The accepted-event path must use explicit SQLite transaction management so the stored event row, accumulator mutation, threshold-crossing check, and optional signal insert commit together. The implementation may choose the exact transaction scope inside `ServerStore`, but accepted writes must no longer rely on SQLite auto-commit. `InsertResult::Accepted` or equivalent accepted-event flow must expose the inserted row context needed for accumulator updates without reinterpreting the event.

### Decision 4: Use configurable, edge-triggered cumulative signals

Add `signal_threshold` to `scryrs-server` configuration with a deterministic default of `10`. Emit a `HotspotSignal` only when an accepted update changes a subject score from below the threshold to at or above it. Each signal record must include repository identity, subject kind, subject, score, delta, `window`, threshold, evidence references, and `created_at`.

### Decision 5: Use server event row IDs as evidence references and preserve batch ordering when materialized

Signal evidence references must use stable `server_trace_events` row IDs. When cumulative live state is materialized for tests or future read paths, evidence ordering must match batch hotspot semantics: `timestamp ASC, id ASC`. This keeps cumulative rank and evidence comparisons aligned even if ingest arrival order differs from event timestamp order.

### Decision 6: Ship cumulative-only foundation with no backfill and internal query helpers

This task implements only the cumulative live window, but stores a `window` field/value of `"cumulative"` so recent-window extensions remain additive. Existing `server_trace_events` rows are not backfilled during schema upgrade; only events accepted after migration contribute to accumulators. Read behavior needed for verification is satisfied by internal server-store query helpers rather than new HTTP read/SSE routes.

## Risks

| Risk | Mitigation |
| --- | --- |
| Existing `server.db` files become unopenable if schema version handling stays strict. | Define an additive v1→v2 server-store migration path that creates the new tables and records the new version without touching local stores. |
| Live/batch alignment can drift if evidence is kept in ingest order instead of chronological order. | Materialize evidence in `timestamp ASC, id ASC` order whenever live state or signals expose contributing event references. |
| Atomic accumulator work increases ingest lock time. | Keep the change limited to deterministic accumulator/signal writes inside the existing server-owned SQLite critical section and cover duplicate/concurrency behavior with tests. |
| Session-count tracking can silently drift if accumulator state omits distinct-session information. | Persist deterministic distinct-session state or an equivalent normalized companion representation as part of the server-side live hotspot model. |

## Conflict Resolution

1. **HTTP read surfaces**: accepted refinement evidence disagreed on whether to add GET routes now. This specification defers new HTTP read/SSE behavior and keeps the task on ingest-time foundation work because the accepted task criteria are satisfied by transactional persistence, internal query helpers, and alignment tests, while the existing transport contract still carries deferred signal-stream details.
2. **Recent-window scope**: refinement raised recent-window questions but consistently treated cumulative alignment as the required target. This specification implements cumulative only and preserves a `window` field/value for additive follow-up work.
3. **Signal semantics**: the threshold crossing rule is resolved as edge-triggered only (`old_score < threshold <= new_score`), matching the accepted architecture guidance and the task wording about crossing a configured threshold.
4. **Backfill policy**: schema upgrade does not rebuild accumulators from historical `server_trace_events`; that work is explicitly deferred.

## Traceability

- Task: `5b6d00fd-1bc8-413a-a41f-bed6724d7c72`
- Exploration dossier: `2026-06-25T18:39:53.480Z`
- Accepted decisions: `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, `1-swarm-reviewer-recommendation`
- Round outputs: `round:1:agent:swarm-architect`, `round:1:agent:swarm-lead-dev`, `round:1:agent:swarm-reviewer`
- Artifact base: `initial`
