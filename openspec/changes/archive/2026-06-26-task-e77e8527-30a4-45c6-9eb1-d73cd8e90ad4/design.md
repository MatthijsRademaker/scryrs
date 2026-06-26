## Context

The repository already has the server-side foundations this task depends on: `scryrs-server` accepts remote ingest, stores accepted events in SQLite, updates cumulative hotspot accumulators, and persists threshold-crossing `HotspotSignal` rows. The gap is on the read side. The server currently exposes only `POST /v1/trace-events/batch`, the dashboard reads `.scryrs/hotspots.json` from disk, and the live source-of-truth contract says remote hotspot consumers must query server state rather than artifact files.

This change turns that existing server-owned state into read-only REST and SSE APIs without expanding into graph, proposal, route, runtime retrieval, or dashboard mutation behavior.

## Goals / Non-Goals

**Goals**

- Expose a read-only hotspot query endpoint for a repository that returns ranked `LiveHotspotsResponse` data from server-owned state.
- Expose a read-only SSE endpoint that streams `HotspotSignal` records in deterministic order with cursor/resume semantics suitable for dashboard and CLI consumers.
- Support repository, cumulative-window, and session-scoped hotspot queries without misrepresenting global accumulator totals as session-filtered results.
- Preserve `.scryrs/hotspots.json` as an export/cache artifact rather than the live source of truth.
- Keep the implementation additive and limited to live hotspot query and signal APIs.

**Non-Goals**

- Websocket transport.
- Dashboard mutation workflows or other write APIs beyond the existing ingest endpoint.
- Graph, proposal, route, or runtime retrieval APIs.
- Changes to the inner `TraceEvent` schema or the local `HotspotsReport` artifact contract.
- Historical backfill beyond the accumulator behavior already defined by the previous foundation.
- Dashboard frontend migration to the new endpoints.

## Decisions

### Decision 1: Split hotspot query materialization by scope

Unfiltered repository queries use the cumulative `hotspot_accumulators` state already maintained by the server. Session-scoped queries must not filter those repository-level aggregates by session membership because the accumulator rows keep global counts and only distinct session identifiers. When `session_id` is supplied, the server recomputes rankings from matching `server_trace_events` rows through the existing deterministic `scryrs_core::scoring::score_hotspots` path.

### Decision 2: Standardize the public hotspot query on `window=cumulative` and optional `session_id`

This foundation supports only the cumulative window. `GET /v1/repositories/{repository_id}/hotspots` accepts `window` and optional `session_id`; unsupported window values return `400 Bad Request` rather than guessed data. The endpoint returns all ranked entries for the requested scope, and the `cursor` field in `LiveHotspotsResponse` remains an opaque response field for future use rather than enabling request pagination in this task.

### Decision 3: Use snake_case query parameters for the server API surface

The route contract already uses `repository_id` in the path and the existing server-side ingest contract is snake_case. This specification standardizes hotspot query and signal cursor parameters as `session_id`, `window`, and `after` to keep the public server API internally consistent and avoid adding alias behavior that refinement did not require.

### Decision 4: Define SSE delivery around persisted `hotspot_signals.id`

`GET /v1/repositories/{repository_id}/signals` uses Server-Sent Events, not websockets. Each message carries a serialized `HotspotSignal` JSON payload in the SSE `data` field and uses the persisted `hotspot_signals.id` as the SSE `id`. The endpoint accepts `after=<signal_id>`; on connect it replays persisted repository signals with `id > after` ordered by `id ASC`, then tails newly committed signals for that repository in the same deterministic order. `after=0` replays the full persisted history for that repository.

### Decision 5: Add signal fanout infrastructure outside the store mutex

The current `Arc<Mutex<ServerStore>>` has no notification mechanism for new signals. The minimal working design is a `tokio::sync::broadcast` channel owned in server app state outside the store mutex, plus the Tokio stream support needed to adapt receivers into Axum SSE responses. Ingest keeps its external contract unchanged and publishes newly committed signals to the broadcast channel after the database commit.

### Decision 6: Preserve artifact export as a separate path

The live query and signal handlers read only server-owned state. They do not consult `.scryrs/hotspots.json`. Existing local `scryrs hotspots` behavior remains the export path for portable reports, and public discovery text for `scryrs server` should mention the new read-only endpoints once they are added.

## Risks

| Risk | Mitigation |
| --- | --- |
| Slow SSE consumers can fall behind a bounded broadcast buffer. | Use cursor replay semantics (`after=<signal_id>`) so reconnecting clients can recover missed signals from persisted rows. |
| Session-scoped queries recompute rankings from filtered event rows and may cost more than accumulator-backed repository queries. | Keep correctness primary, constrain scope to cumulative-only behavior, and leave storage-level tuning outside the behavioral contract for this task. |
| Holding the store mutex across async SSE work would block ingest. | Limit store lock usage to short-lived replay queries and release it before entering the streaming loop. |
| Unsupported window values could silently produce incorrect data if defaulted. | Reject any non-`cumulative` window with a clear `400 Bad Request` response. |

## Conflict Resolution

1. **Query parameter casing**: refinement raised both camelCase and snake_case options. This specification chooses snake_case (`session_id`) because the server contract already exposes snake_case path/query terminology and no accepted decision required dual-format compatibility.
2. **Hotspot query pagination**: refinement questioned whether to add request pagination now. This specification returns all ranked entries for the requested scope and preserves `LiveHotspotsResponse.cursor` as an opaque response field for future use, matching the scope-discipline recommendation.
3. **Signal replay semantics**: refinement asked whether replay should happen before tailing. This specification requires replay of persisted signals with `id > after` in `id ASC` order before tailing newly committed repository signals so reconnecting clients can recover deterministically.
4. **Performance tuning vs behavioral contract**: refinement discussed adding a composite `(repository_id, session_id)` index. Because the accepted evidence converged on correctness requirements rather than a mandatory storage shape, this specification keeps the behavior mandatory and leaves index choice to implementation judgment.

## Traceability

- Task: `e77e8527-30a4-45c6-9eb1-d73cd8e90ad4`
- Exploration dossier: `2026-06-26T20:50:17.734Z`
- Accepted decisions: `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, `1-swarm-reviewer-recommendation`
- Round outputs: `round:1:agent:swarm-architect`, `round:1:agent:swarm-lead-dev`, `round:1:agent:swarm-reviewer`
- Artifact base: `initial`