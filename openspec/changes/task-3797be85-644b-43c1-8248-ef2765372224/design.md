## Context

The `scryrs dashboard` command was designed and implemented as a local-only viewer for `.scryrs/hotspots.json` and `.scryrs/scryrs.db` artifacts. Its CLI (`crates/scryrs-cli/src/dashboard.rs`) supports only `--port`, `--bind`, `--no-open`, and `--dev`. Its backend (`crates/scryrs-dashboard/src/server.rs`) reads local files and SQLite directly. Its frontend (`DashboardShell.vue`) labels the product "Local-only viewer for .scryrs artifacts." and its nav contains Hotspots, Sessions, Events, and About only.

Meanwhile, `scryrs server` already exposes read-only live hotspot rankings and SSE hotspot signals at `GET /v1/repositories/{repository_id}/hotspots` and `GET /v1/repositories/{repository_id}/signals`. The live server contract (`openspec/specs/live-hotspot-server-contract/spec.md`) specifies `LiveHotspotsResponse` envelope shape, `?after=` cursor replay, and the intentional omission of filesystem path fields.

There is an existing `scryrs.json` `remote.*` config namespace with env-over-manifest precedence used for record ingest transport configuration. However, that namespace carries mandatory ingest-identity fields (`workspace_id`, `agent_id`, `timeout_ms`) that are irrelevant to a read-only dashboard.

Refinement converged on three architectural decisions:
1. A dedicated dashboard config namespace independent of `remote.*`.
2. Backend proxy for all live API access, preserving the same-origin `/api/*` SPA contract.
3. A new Signals view with SSE-based signal timeline; Sessions and Events explicitly hidden in live mode.

## Goals / Non-Goals

### Goals

- Define an explicit read-only live dashboard mode that never replaces local mode by accident.
- Define config inputs and precedence for server URL and repository identity before implementation.
- Map live hotspot and signal APIs to concrete dashboard UI/data needs, including reconnect and empty/error behavior.
- Specify exactly which current dashboard views remain local-only, are hidden, or are replaced in live mode.
- Document an end-to-end verification plan for both live mode and unchanged local default behavior.

### Non-Goals

- Adding write/mutation APIs, auth, TLS, hosted multi-tenant behavior, or websocket transport.
- Changing `scryrs server` ingest/query wire contracts beyond what the existing live hotspot and signal APIs already provide.
- Merging local `.scryrs` artifacts with remote server state or introducing silent fallback from live mode to local files.
- Guaranteeing full live parity for Sessions/Event drill-down views (server has no equivalent read APIs).
- Implementing the live mode — this change defines the contract only.

## Decisions

### Decision 1: Dedicated dashboard config namespace

Define `dashboard.server_url` and `dashboard.repository_id` in `scryrs.json`, with CLI flags `--server-url <URL>` and `--repository-id <ID>`, and env vars `SCRYRS_DASHBOARD_SERVER_URL` and `SCRYRS_DASHBOARD_REPOSITORY_ID`. Precedence: CLI > env > manifest. Absence of both fields means local mode (current default).

**Rationale**: The existing `remote.*` namespace in `scryrs.json` and `remote_config.rs` carries `ingest_url`, `workspace_id`, `agent_id`, and `timeout_ms` — fields irrelevant to a read-only dashboard. Reusing `remote.*` would force operators to supply unnecessary agent identity fields and conflate ingest transport config with read query config. A dedicated namespace cleanly separates concerns.

### Decision 2: Both serverUrl and repositoryId required together

Live mode activates only when both `server_url` and `repository_id` are explicitly configured. No auto-resolution from git remotes or server discovery.

**Rationale**: The server API requires `repository_id` as a URL path segment (`GET /v1/repositories/{repository_id}/...`) and exposes no repository discovery endpoint. The dashboard cannot derive `repository_id` from the server. Requiring both prevents ambiguous partial configuration.

### Decision 3: Backend proxy for all live API access

The dashboard Rust backend proxies all live API calls. The SPA continues to talk same-origin `/api/*`. In live mode:
- `/api/hotspots` → proxies to `GET /v1/repositories/{id}/hotspots?window=cumulative`
- `/api/signals` → proxies SSE from `GET /v1/repositories/{id}/signals?after=<cursor>`
- `/api/meta` → returns `mode`, `repositoryId`, and `repositoryPath`

**Rationale**: (a) The server today has no CORS headers, so browser-to-server direct connections would fail. (b) `?after=` cursor resume requires stateful management that belongs in the backend, not the SPA. (c) All existing frontend fetch calls are relative `/api/*`; maintaining same-origin preserves this pattern.

### Decision 4: Per-client upstream SSE connections with in-memory cursor state

The backend maintains one persistent upstream SSE connection to the server per connected dashboard browser client. The last-seen signal ID is stored in-memory per connection. On backend restart, cursor resets to `after=0`. On upstream disconnection, the proxy sends 502 to the SSE client without silent retry.

**Rationale**: Per-client connections are simpler than broadcast with shared fan-out. In-memory cursor state matches the ephemeral nature of a local dashboard; the server supports `after=0` replay for full recovery. Silent retry would mask server-unavailable state from the operator.

### Decision 5: Sessions and Events hidden in live mode

In live mode, Sessions and Events navigation items are hidden. If navigated to directly via URL, an explanatory unavailable view is shown. The About view is updated to describe live mode capabilities.

**Rationale**: The server has no Session or Event read APIs. Showing these navigation items would imply capabilities that don't exist. Hiding them is honest and prevents broken user journeys. An explanatory stub view covers the edge case of direct URL navigation.

### Decision 6: Shape normalization at the backend boundary

The backend normalizes `LiveHotspotsResponse` into the existing frontend `HotspotsReport` envelope shape. The shared `HotspotEntry` type ensures inner entry rendering works unchanged. The `cursor` field from `LiveHotspotsResponse` is preserved in the normalized response for future use.

**Rationale**: The frontend already has working `HotspotsView.vue` that renders `HotspotEntry` items. Normalizing the envelope at the backend keeps the frontend changes minimal and avoids duplicating hotspot rendering code.

### Decision 7: Distinguish server-unavailable from empty-repository

`/api/hotspots` returns 502 with an error body when the upstream server is unreachable (connection refused, timeout, DNS failure). It returns 200 with empty entries when the server responds with an empty `LiveHotspotsResponse` (e.g., unknown repository).

**Rationale**: The server contract returns 200 with empty entries for unknown repositories. These are semantically different states — an operator needs to know whether the server is down or whether the repository simply has no data.

### Decision 8: Subject paths displayed raw in live mode

Subject paths are displayed as-is in live mode because `LiveHotspotsResponse` intentionally omits `repositoryPath`. The dashboard includes its own CWD-derived `repositoryPath` in `/api/meta` for optional path shortening by the frontend, but this is the dashboard's CWD, not the server's — it may not match.

**Rationale**: The live hotspot server contract requires that live responses contain no filesystem path fields. Displaying raw subjects is acceptable and documented as a deliberate visual difference from local mode.

## Risks

| Risk | Mitigation |
| --- | --- |
| Subject display without filesystem path context may produce confusing long paths in live mode | Document as a deliberate difference in the About view. Include dashboard CWD `repositoryPath` in `/api/meta` so frontend can optionally normalize paths if the CWD matches the server's repo root. |
| SSE channel capacity mismatch — server channel is 1024; slow dashboard consumer could lose signals during burst ingest | Document risk in the live dashboard contract. The backend proxy logs lag events. `?after=` cursor replay is the recovery mechanism. |
| Browser EventSource reconnection does not use `?after=` cursor natively, requiring frontend cursor management | The frontend EventSource client manages `after` explicitly in the URL query parameter, not relying on `Last-Event-ID` header behavior. |
| Cursor state lost on backend restart forces full replay from `after=0` | Acceptable for a local dashboard. The server replays all persisted signals on `after=0`. Document this behavior. |
| Multiple browser tabs sharing one dashboard backend will have independent per-connection cursor state | Acceptable — each tab gets its own SSE connection with independent cursor state. No signal duplication risk. |

## Conflict Resolution

1. **SSE cursor state storage**: Architect specified backend-centralized cursor management; lead-dev specified in-memory per connection; reviewer raised `localStorage` as an alternative. Resolution: backend in-memory per connection (aligns with architect's backend-centralized architecture and lead-dev's explicit specification). `localStorage` is deferred as a future enhancement.
2. **Sessions/Events visibility in live mode**: Architect said "explicitly hidden"; lead-dev said "visually disabled with tooltip"; reviewer said hidden. Resolution: navigation items hidden, with an explanatory unavailable view rendered when navigated to directly via URL (covers both hiding and the edge case).
3. **Server-unavailable vs empty semantics**: Lead-dev required distinguishing 502 (unreachable) from 200-with-empty (empty repository). Architect and reviewer did not contradict. Adopted as specified.
4. **Config namespace**: All three agents converged on dedicated `dashboard` namespace independent of `remote.*`. Adopted.

## Traceability

| Source | How it is used |
| --- | --- |
| Task `3797be85-644b-43c1-8248-ef2765372224` | Defines feature scope, scenarios, technical notes, and acceptance criteria. |
| Dossier `2026-06-29T18:07:05.167Z` | Supplies goals, non-goals, assumptions, open questions, proposal sketch, and affected areas. |
| Accepted decision `1-swarm-architect-recommendation` | Fixes dedicated config namespace, backend SSE proxy, Signals view, Sessions/Events hidden. |
| Accepted decision `1-swarm-lead-dev-recommendation` | Fixes config namespace separation, repositoryId required, in-memory cursor state, mode-aware /api/meta, 502 vs 200 distinction, shape normalization. |
| Accepted decision `1-swarm-reviewer-recommendation` | Fixes config namespace, repository_id sourcing, SSE proxy design parameters. |
| `openspec/specs/live-hotspot-server-contract/spec.md` | Authoritative contract for live hotspot and signal APIs that dashboard live mode consumes. |
| `crates/scryrs-types/src/lib.rs` | Source of truth for `LiveHotspotsResponse`, `HotspotsReport`, `HotspotEntry`, and `HotspotSignal` type shapes. |
| `crates/scryrs-cli/src/dashboard.rs`, `crates/scryrs-dashboard/src/lib.rs`, `crates/scryrs-dashboard/src/server.rs` | Current dashboard implementation that the contract extends. |
| `.devagent/docs/docs/live-hotspots.md`, `.devagent/docs/docs/cli-v0-contract.md`, `.devagent/docs/docs/roadmap.mdx` | Documentation surfaces that this contract updates. |
| `scripts/verification/README.md`, `scripts/verification/live-hotspots-e2e.mjs` | Existing verification infrastructure to extend with dashboard live-mode smoke. |