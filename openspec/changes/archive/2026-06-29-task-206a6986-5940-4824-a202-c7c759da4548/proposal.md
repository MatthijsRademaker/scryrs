## Why

The requested dashboard experience cannot be delivered as a frontend-only change. `scryrs server` already exposes live hotspot rankings and hotspot signal SSE, but `scryrs dashboard` still assumes local `.scryrs` artifacts in its CLI, backend routes, frontend copy, and tests. Multi-agent operators need an explicit live dashboard mode that loads current rankings from the server, streams replayed and live hotspot signals without gaps, and makes disconnect/reconnect state visible while preserving the existing local dashboard as the default.

## What Changes

- Add explicit live dashboard activation to `scryrs dashboard` with `--server-url` and `--repository-id`; keep local mode as the default and fail loudly on partial live configuration.
- Extend the dashboard backend with a `SourceMode` split, mode-aware `/api/meta`, live `/api/hotspots` proxying to `GET /v1/repositories/{repository_id}/hotspots?window=cumulative`, and a new `/api/signals` SSE proxy that forwards `after` and streams replay plus live events without buffering the full upstream response.
- Keep browser code on same-origin `/api/*`; the SPA does not call `scryrs server` directly.
- Update the Vue/Pinia/shadcn-vue frontend to:
  - render live rankings in Hotspots without local `.scryrs/hotspots.json` source copy,
  - add a live-mode Signals route and timeline for replayed plus live `HotspotSignal` events,
  - manage `EventSource` reconnect explicitly with in-memory `after=<lastSeenId>` cursor state for the current page lifecycle,
  - show connecting, connected, disconnected/reconnecting, and error states,
  - hide Sessions and Events in live mode and show an unavailable explanation on direct navigation,
  - update shell, footer, and About copy for local vs live mode.
- Add live-mode backend test coverage while preserving existing local dashboard tests.
- Update project docs and verification docs for the live dashboard workflow.

## Impact

- Affected areas: `crates/scryrs-cli`, `crates/scryrs-dashboard` backend/frontend/tests, dashboard docs, and verification docs.
- New backend HTTP/SSE proxy dependencies are required for live mode, including a streaming HTTP client configuration compatible with the Docker build environment.
- Local dashboard behavior remains the default and must continue to pass its existing tests without live configuration.
- Scope remains read-only only: no dashboard mutation behavior, no new server write APIs, no auth/TLS/browser-direct transport, and no local/live data merging or silent fallback.