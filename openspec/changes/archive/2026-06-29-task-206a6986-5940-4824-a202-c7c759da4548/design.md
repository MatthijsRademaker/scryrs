## Context

The current repository already has the live server contract needed by this task: `GET /v1/repositories/{repository_id}/hotspots` for live rankings and `GET /v1/repositories/{repository_id}/signals` for replayable SSE hotspot signals. The dashboard does not yet have the plumbing to use that contract. Its CLI only exposes local-mode flags, its backend only serves local `.scryrs` files and SQLite data, `/api/meta` has no mode or repository identity, there is no `/api/signals`, and the frontend shell and views are written as a local-artifact viewer.

Refinement converged on a narrow implementation slice: use the previously designed live-dashboard-mode architecture as the foundation, but absorb the minimum backend/config/proxy work needed for this task to function. The accepted scope refinement is CLI flags only for live configuration now, with in-memory frontend cursor state for the current page lifecycle.

## Goals / Non-Goals

### Goals

- Add an explicit live dashboard mode that fetches current hotspot rankings for a configured repository from the live server.
- Add a read-only Signals timeline that replays persisted `HotspotSignal` records and appends live SSE updates without gaps by reconnecting with the last seen `after` cursor.
- Preserve local dashboard mode as the default and keep existing local behavior and tests intact.
- Keep all browser traffic on same-origin `/api/*` endpoints through the dashboard backend.
- Document the live dashboard workflow, including startup, rankings, signals, reconnect behavior, and local-vs-live differences.

### Non-Goals

- Do not add dashboard mutation behavior or new server write APIs.
- Do not implement auth, TLS, hosted multi-tenant deployment, websocket transport, or browser-direct calls to `scryrs server`.
- Do not implement `scryrs.json` or env-var live dashboard config precedence in this task; accepted scope is CLI flags only.
- Do not persist signal cursor state across a full page refresh; refresh returns to `after=0` replay.
- Do not merge local `.scryrs` artifacts with live server state or silently fall back from live to local data.
- Do not add graph, route, proposal, runtime retrieval, or LLM interpretation features.

## Decisions

### Decision 1: Reuse the live-dashboard-mode architecture, but implement it here with tighter scope

This task uses the live-dashboard-mode design from task `3797be85-644b-43c1-8248-ef2765372224` as its architectural base, because the current task cannot meet its acceptance criteria as a frontend-only change. The implementation in this change absorbs the minimal CLI, backend proxy, frontend state, testing, and documentation work required for the live UI to function.

### Decision 2: Live mode is explicit and CLI-only for this task

`scryrs dashboard` gains `--server-url` and `--repository-id`. Both are required together to activate live mode. Omitting both keeps local mode as the default. Providing only one is a startup error. Manifest and env-var configuration are deferred.

### Decision 3: The dashboard backend remains the only browser-facing API surface

The SPA continues to use same-origin `/api/*`. In live mode, the backend adds mode-aware `/api/meta`, proxies `/api/hotspots` to `GET /v1/repositories/{repository_id}/hotspots?window=cumulative`, and exposes `/api/signals` as the dashboard-owned SSE endpoint. The browser does not connect directly to `scryrs server`.

### Decision 4: Live hotspot rankings reuse the current hotspot rendering path

The live hotspot proxy normalizes the live server envelope into a shape compatible with the current hotspot table, preserves the live `cursor`, and distinguishes upstream-unavailable errors from valid empty live results. The Hotspots view keeps existing loading, empty, and error behavior but removes local-file wording when in live mode.

### Decision 5: Cursor-based reconnect is owned by the frontend for the current page lifecycle

The frontend signal client owns `EventSource` lifecycle rather than relying on native browser auto-reconnect. It opens `/api/signals?after=0` on first connect, tracks the last seen signal id in memory, reconnects with `?after=<lastSeenId>` after disconnect, and deduplicates replayed events on resume. A full page refresh starts again from `after=0`.

### Decision 6: The SSE proxy must stream, not buffer

The dashboard backend uses a streaming HTTP client path for `/api/signals` so replayed and live signals are forwarded as they arrive. Refinement specifically called out `reqwest` with rustls-compatible streaming support and stream helpers as required backend dependencies. Silent upstream retry is out of scope; disconnects must surface to the frontend so the UI can show reconnecting or error state and reopen the stream with the saved cursor.

### Decision 7: Live mode changes the shell, not the local feature set

In live mode, the shell shows Hotspots, Signals, and About; Sessions and Events are hidden from navigation and direct URLs render a clear unavailable message. Footer and About copy reflect live server mode. Subject paths in live hotspots are shown as received from the server by default rather than implying local repository-root shortening.

### Decision 8: Verification must cover both live mode and unchanged local mode

Implementation must preserve existing local dashboard tests and add live-mode coverage for meta mode dispatch, hotspot proxy semantics, sessions/events unavailability, and signal replay/resume behavior. Refinement left the exact live proxy fixture open: a mock HTTP server or spawned minimal Axum server is acceptable as long as the reqwest-backed code path is exercised.

## Risks

| Risk | Mitigation |
| --- | --- |
| Native browser `EventSource` reconnect would reuse the original URL and lose the `after` cursor | Own reconnect in a store/composable that closes and recreates the stream with `?after=<lastSeenId>`. |
| A buffered SSE proxy would break real-time behavior | Stream the upstream response chunk-by-chunk; do not read the full upstream response before forwarding. |
| Live-mode proxy tests cannot be covered by the existing router-only local test pattern | Add live-mode tests against a mock or spawned upstream server so the HTTP client path is exercised. |
| Operators could confuse local and live views if both appear available at once | Make the shell mode-aware, hide live-incompatible navigation, and update copy to describe the active mode clearly. |
| Subject display differs between local and live modes because live responses omit repository-root metadata | Show raw live subjects by default and document the difference in the UI/docs. |

## Conflict Resolution

1. **Config scope**: refinement evidence for the earlier live-dashboard-mode design included CLI, env, and manifest precedence, but accepted decisions for this task narrowed implementation to CLI flags only. This change adopts the narrower scope.
2. **Reconnect cursor ownership**: the architectural foundation discussed backend-managed cursor state, but accepted task decisions require frontend-managed in-memory cursor tracking for the current page lifecycle. This change adopts frontend-managed `after` reconnects and full replay after refresh.
3. **Local-only views in live mode**: refinement converged on hiding Sessions and Events in live navigation while still rendering an explanatory unavailable view for direct URL access. This change adopts that behavior.
4. **Implementation boundary**: the task prompt is UI-focused, but the dossier and accepted decisions established that the feature cannot ship without CLI/config/backend proxy work. This change explicitly includes that minimum supporting work.

## Traceability

- Task `206a6986-5940-4824-a202-c7c759da4548`
- Dossier `2026-06-29T18:46:09.884Z`
- Accepted decisions `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, and `1-swarm-reviewer-recommendation`
- Validated round outputs from `swarm-architect`, `swarm-lead-dev`, and `swarm-reviewer`
- Current artifact snapshot `initial`
- Prior live-dashboard-mode change `task-3797be85-644b-43c1-8248-ef2765372224` as the adopted architectural foundation
- Existing live server contract `openspec/specs/live-hotspot-server-contract/spec.md`