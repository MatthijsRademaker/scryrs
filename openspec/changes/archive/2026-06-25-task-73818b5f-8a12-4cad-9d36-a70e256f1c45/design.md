## Context

`scryrs server` currently exposes only `POST /v1/trace-events/batch`. Live hotspot accumulators and signals exist in its SQLite store but lack production read methods. Existing store helpers (`get_accumulator_row`, `get_signals`) are test-oriented raw-row accessors. Signal ordering uses `created_at ASC` with second-only precision — unsafe for deterministic streaming.

## Goals / Non-Goals

**Goals:**
- Expose `GET /v1/repositories/{repository_id}/hotspots` returning `LiveHotspotsResponse` from server accumulator state
- Expose `GET /v1/repositories/{repository_id}/signals` as SSE with id-ordered `HotspotSignal` records
- Support `window=cumulative`; reject unsupported windows with 400
- Defer session-scoped filtering with a deterministic 400
- Order signal stream by `hotspot_signals.id ASC` (autoincrement PK), not `created_at`
- Support SSE replay via `after=<id>` and `Last-Event-ID`
- Use separate read-only `rusqlite::Connection` per SSE stream
- Preserve `.scryrs/hotspots.json` export path

**Non-Goals:**
- WebSocket transport
- Graph, proposal, or runtime APIs
- Dashboard rewiring
- Session-scoped hotspot rankings (deferred)
- Paginated hotspot lists (cursor is snapshot marker only)
- Recent-window accumulation beyond `cumulative`

## Decisions

### Decision 1: Accumulator-based materialization (not rescoring raw events)
`query_hotspots` directly materializes `HotspotEntry` from `hotspot_accumulators` using stored score/counts/sessions/timestamps/evidence. Does NOT call `score_hotspots()` — that would require resolving evidence row IDs back to `TraceEvent` objects, defeating the purpose of precomputed accumulators. The six-key tie-break is replicated: score DESC, sessionCount DESC, lastSeen DESC, subjectKind ASC, subject ASC, firstEvidenceId ASC.

### Decision 2: Signal ordering by autoincrement id
`hotspot_signals.id` is `INTEGER PRIMARY KEY AUTOINCREMENT` — durable, strictly monotonic. `created_at` has second-only precision from `chrono_now()`. `poll_signals` orders by `id ASC` and uses `WHERE id > ?` for gap-tolerant replay.

### Decision 3: Separate read-only connection per SSE stream
`Arc<Mutex<ServerStore>>` is designed for short-lived batch inserts. Holding it for a long-lived SSE stream would serialize all operations. Each stream opens a separate `rusqlite::Connection` leveraging SQLite WAL mode.

### Decision 4: Session-scoped filtering deferred with 400
`hotspot_accumulators` stores only a distinct session set, not per-session scores. The `session_id` parameter returns `400 Bad Request` with message explaining the deferral.

### Decision 5: SSE payload = HotspotSignalEvent wrapper
New type in `scryrs-types` adds `id: i64` to all existing `HotspotSignal` fields. Used as `data:` payload; `id` set as SSE `id:` field.

### Decision 6: cursor = generatedAt snapshot marker
`LiveHotspotsResponse.cursor` is set to `generatedAt` (RFC 3339). A point-in-time snapshot marker, not a pagination cursor.

## Risks

- **SSE mutex blocking:** Mitigated by separate read-only connection per stream.
- **Same-second signal ordering:** Mitigated by autoincrement id ordering.
- **Session filter confusion:** Mitigated by explicit 400 deferral message.
- **Autoincrement gap replay:** Mitigated by `WHERE id > ?` semantics.
- **Cursor misinterpretation:** Mitigated by documenting as snapshot marker.

## Traceability

- Task: `73818b5f-8a12-4cad-9d36-a70e256f1c45`
- Dossier: `2026-06-25T19:56:17.631Z`
- Decisions: `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, `1-swarm-reviewer-recommendation`
- Round outputs: round 1 from swarm-architect, swarm-lead-dev, swarm-reviewer
- Source files: `crates/scryrs-server/src/server.rs`, `crates/scryrs-server/src/store.rs`, `crates/scryrs-server/src/time.rs`, `crates/scryrs-types/src/lib.rs`, `crates/scryrs-core/src/scoring.rs`
- Spec files: `openspec/specs/live-hotspot-server-contract/spec.md`, `openspec/specs/live-hotspot-accumulators/spec.md`, `openspec/specs/hotspot-report/spec.md`