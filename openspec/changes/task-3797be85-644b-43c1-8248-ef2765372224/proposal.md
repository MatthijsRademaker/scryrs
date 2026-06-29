## Why

The `scryrs dashboard` command is currently local-only: its CLI, backend, frontend copy, and tests assume `.scryrs/hotspots.json` and `.scryrs/scryrs.db` as the sole data sources. Meanwhile `scryrs server` already exposes read-only live hotspot rankings (`GET /v1/repositories/{repository_id}/hotspots`) and SSE hotspot signals (`GET /v1/repositories/{repository_id}/signals`), but there is no dashboard contract for activating live mode, resolving server/repository identity, mapping live response shapes into the current UI, or handling reconnect/empty/unavailable states. Without that contract, multi-agent operators must use curl or API inspection to see shared live state.

This change defines the read-only live dashboard mode contract. It specifies how live mode is activated, how the dashboard backend proxies server APIs through the existing same-origin `/api/*` boundary, how the frontend adapts to live mode, and what happens to locally-oriented views (Sessions, Events) that have no server read API equivalent. The contract is design-level: it resolves the config namespace, SSE proxy architecture, cursor resume state management, and UI behavior decisions that refinement identified as blocking for implementation.

## What Changes

- **New dashboard config namespace**: Define a dedicated `dashboard` section in `scryrs.json` with `server_url` and `repository_id` fields, mirrored by `SCRYRS_DASHBOARD_SERVER_URL` and `SCRYRS_DASHBOARD_REPOSITORY_ID` environment variables, and `--server-url`/`--repository-id` CLI flags on `scryrs dashboard`. Precedence: CLI > env > manifest. Absence of both fields means local mode (current default).
- **Backend mode switching**: Introduce a `SourceMode` enum (`Local` | `Live { server_url, repository_id }`) in the dashboard `Config`. Backend routes dispatch to file-backed or HTTP-proxied handlers based on mode.
- **Backend-proxied live API**: In live mode, `GET /api/hotspots` proxies to `GET /v1/repositories/{id}/hotspots?window=cumulative` on the configured server, normalizing the `LiveHotspotsResponse` envelope to the frontend's expected shape. `GET /api/meta` returns `mode: "live"` plus `repositoryId`. Sessions and Events endpoints (`/api/sessions`, `/api/sessions/:id`, `/api/events`) return 404 with an explicit "unavailable in live mode" error body.
- **Backend-proxied SSE signals**: New `GET /api/signals` SSE endpoint that maintains one persistent upstream connection per dashboard client to `GET /v1/repositories/{id}/signals?after=<cursor>`. Cursor state is stored in-memory per connection in the backend; on backend restart, cursor resets to `after=0`. On upstream disconnection, the proxy reports 502 to the SSE client without silent retry.
- **Frontend live-mode adaptation**: Dashboard shell reads `/api/meta` mode to conditionally render navigation — Sessions and Events are hidden in live mode with an explanatory unavailable view if navigated to directly. A new Signals view with SSE-based signal timeline becomes the primary live-mode navigation entry. Footer copy reflects the active mode.
- **Shape normalization and error handling**: `/api/hotspots` returns 502 when the upstream server is unreachable, and 200 with empty entries when the server returns an empty `LiveHotspotsResponse`. Subject display uses raw paths in live mode (documented as a deliberate difference since live responses omit `repositoryPath`).
- **Documentation and verification**: Update `.devagent/docs/docs/cli-v0-contract.md`, `.devagent/docs/docs/live-hotspots.md`, and `.devagent/docs/docs/roadmap.mdx` to document live dashboard mode. Define an end-to-end verification plan extending the existing live-hotspots verification infrastructure with a dashboard smoke step, plus a local-mode regression check.

## Impact

- **Affected specs**: Adds `openspec/specs/live-dashboard-mode/spec.md` (new). Does NOT modify existing specs (`live-hotspot-server-contract`, `cli-dashboard-command`, `dashboard-phase-goal`).
- **Affected code (design-level, implementation in follow-up tasks)**:
  - `crates/scryrs-cli/src/dashboard.rs` — new CLI flags
  - `crates/scryrs-dashboard/src/lib.rs` — `Config` gains `SourceMode`
  - `crates/scryrs-dashboard/src/server.rs` — new routes, mode-dispatch, SSE proxy
  - `crates/scryrs-dashboard/Cargo.toml` — add `reqwest`, `tokio-stream`
  - `crates/scryrs-dashboard/frontend/src/shared/api/client.ts` — `EventSource` client, mode-aware types
  - `crates/scryrs-dashboard/frontend/src/shared/ui/shell/DashboardShell.vue` — conditional nav, mode-aware footer
  - `crates/scryrs-dashboard/frontend/src/router/index.ts` — Signals route
  - `crates/scryrs-dashboard/frontend/src/views/` — new Signals view, About view updates
  - `scryrs.json` — new `dashboard` config section
  - `scripts/verification/` — dashboard live-mode smoke step
  - `.devagent/docs/` — live dashboard mode documentation
- **Local mode unchanged**: Existing local-only behavior, tests, and default startup are preserved exactly. No silent fallback or merge between local and live data.
- **Breaking changes**: None. Live mode is strictly opt-in via explicit configuration.