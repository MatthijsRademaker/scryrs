## ADDED Requirements

### Requirement: Live dashboard mode is activated by explicit configuration

Dashboard live mode SHALL be activated only when both `server_url` and `repository_id` are explicitly configured. When either is absent, the dashboard SHALL run in local mode using `.scryrs/hotspots.json` and `.scryrs/scryrs.db` as it does today. Live mode SHALL NOT be activated by implicit detection or environmental heuristics. Local mode SHALL remain the default.

#### Scenario: Live mode activates with full configuration

- **GIVEN** `scryrs dashboard --server-url http://localhost:8081 --repository-id repo-a` is invoked
- **WHEN** the dashboard backend starts
- **THEN** it runs in live mode
- **AND** `/api/meta` returns `mode: "live"` and the configured `repositoryId`
- **AND** `/api/hotspots` proxies to `GET /v1/repositories/repo-a/hotspots?window=cumulative` on the configured server

#### Scenario: Local mode is the default

- **GIVEN** no `--server-url` or `--repository-id` flags, no `SCRYRS_DASHBOARD_SERVER_URL` or `SCRYRS_DASHBOARD_REPOSITORY_ID` env vars, and no `dashboard` section in `scryrs.json`
- **WHEN** `scryrs dashboard` is invoked
- **THEN** the dashboard runs in local mode
- **AND** `/api/meta` returns `mode: "local"`
- **AND** `/api/hotspots` reads `.scryrs/hotspots.json` directly
- **AND** `/api/sessions` and `/api/events` query `.scryrs/scryrs.db`

#### Scenario: Partial live configuration does not activate live mode

- **GIVEN** `--server-url http://localhost:8081` is provided but `--repository-id` is absent
- **WHEN** `scryrs dashboard` is invoked
- **THEN** the dashboard SHALL exit with a configuration error
- **AND** the error message SHALL state that both `server-url` and `repository-id` are required for live mode

#### Scenario: No silent mixing of local and live data

- **GIVEN** the dashboard is running in live mode
- **AND** a local `.scryrs/hotspots.json` and `.scryrs/scryrs.db` exist from prior local use
- **WHEN** dashboard API endpoints are queried
- **THEN** only server data is served
- **AND** local `.scryrs` files are not consulted or merged

### Requirement: Dashboard live-mode config uses a dedicated namespace

The dashboard live-mode configuration SHALL use a dedicated `dashboard` namespace in `scryrs.json` with `server_url` and `repository_id` fields. Environment variable overrides SHALL use `SCRYRS_DASHBOARD_SERVER_URL` and `SCRYRS_DASHBOARD_REPOSITORY_ID`. CLI flags SHALL be `--server-url <URL>` and `--repository-id <ID>` on `scryrs dashboard`. The existing `remote.*` ingest config namespace (`remote_config.rs`) SHALL NOT be reused for dashboard live mode.

#### Scenario: Dashboard config in scryrs.json is independent of remote.*

- **GIVEN** a `scryrs.json` with both a `remote` section (for record ingest) and a `dashboard` section (for dashboard live mode)
- **WHEN** `scryrs dashboard` is invoked
- **THEN** only the `dashboard` section configures dashboard live mode
- **AND** the `remote` section is ignored by the dashboard
- **AND** `scryrs record` continues to use the `remote` section independently

#### Scenario: Env var overrides manifest

- **GIVEN** `scryrs.json` contains `dashboard.server_url = "http://manifest.example.com"`
- **AND** `SCRYRS_DASHBOARD_SERVER_URL=http://env.example.com` is set
- **WHEN** `scryrs dashboard` is invoked
- **THEN** the dashboard uses `http://env.example.com` as the server URL
- **AND** the manifest value is ignored

#### Scenario: CLI flag overrides env var

- **GIVEN** `SCRYRS_DASHBOARD_SERVER_URL=http://env.example.com` is set
- **WHEN** `scryrs dashboard --server-url http://cli.example.com` is invoked
- **THEN** the dashboard uses `http://cli.example.com` as the server URL
- **AND** the env var value is ignored

#### Scenario: Dashboard config namespace is documented in CLI help

- **WHEN** `scryrs dashboard --help` is invoked
- **THEN** the help text SHALL list `--server-url`, `--repository-id`, `--port`, `--bind`, `--no-open`, and `--dev` flags
- **AND** the help text SHALL describe the config precedence (CLI > env > manifest)
- **AND** the help text SHALL state that omitting `--server-url` and `--repository-id` runs in local mode

### Requirement: Repository identity is mandatory for live mode

The dashboard SHALL require `repository_id` as mandatory configuration when live mode is activated. The dashboard SHALL NOT attempt to derive `repository_id` from the server (no repository discovery endpoint exists), from git remote origin, or from any heuristic. The operator MUST supply `repository_id` explicitly.

#### Scenario: Repository ID is required alongside server URL

- **GIVEN** `SCRYRS_DASHBOARD_SERVER_URL=http://localhost:8081` is set
- **AND** no `SCRYRS_DASHBOARD_REPOSITORY_ID` or `--repository-id` is provided
- **WHEN** `scryrs dashboard` is invoked
- **THEN** the dashboard SHALL exit with a non-zero status code
- **AND** stderr SHALL explain that `repository_id` is required for live mode

#### Scenario: Repository ID is passed to server API paths

- **GIVEN** the dashboard is configured with `--repository-id repo-a` and `--server-url http://localhost:8081`
- **WHEN** `/api/hotspots` is called
- **THEN** the backend proxies to `GET http://localhost:8081/v1/repositories/repo-a/hotspots?window=cumulative`
- **AND** the `repository_id` is used verbatim in the URL path segment

### Requirement: Backend proxies live API through same-origin endpoints

In live mode, the dashboard backend SHALL proxy all live API access through same-origin `/api/*` endpoints. The SPA SHALL NOT connect directly to the scryrs server. The backend SHALL use HTTP (reqwest or equivalent) to query the configured server and SHALL normalize response shapes for the frontend.

#### Scenario: /api/hotspots proxies to server live hotspot endpoint

- **GIVEN** the dashboard is running in live mode against server `http://localhost:8081` for repository `repo-a`
- **WHEN** `GET /api/hotspots` is called
- **THEN** the backend sends `GET http://localhost:8081/v1/repositories/repo-a/hotspots?window=cumulative`
- **AND** the backend normalizes the `LiveHotspotsResponse` envelope into the frontend's expected `HotspotsReport` shape
- **AND** the response includes a `cursor` field from the live response

#### Scenario: /api/meta returns live-mode context

- **GIVEN** the dashboard is running in live mode
- **WHEN** `GET /api/meta` is called
- **THEN** the response SHALL include `mode: "live"`
- **AND** the response SHALL include `repositoryId` matching the configured repository identity
- **AND** the response SHALL include `repositoryPath` derived from the dashboard's current working directory

#### Scenario: /api/meta returns local-mode context

- **GIVEN** the dashboard is running in local mode (default)
- **WHEN** `GET /api/meta` is called
- **THEN** the response SHALL include `mode: "local"`
- **AND** the response SHALL include `repositoryPath` as it does today

#### Scenario: Live hotspot entries render with existing HotspotEntry types

- **GIVEN** `LiveHotspotsResponse.entries` contains `HotspotEntry` items (same type as local `HotspotsReport.entries`)
- **WHEN** the backend normalizes the response
- **THEN** the entries array passes through unchanged
- **AND** the existing frontend `HotspotsView.vue` renders each entry without modification

### Requirement: Server-unavailable and empty-repository states are distinguished

The dashboard backend SHALL distinguish between an unreachable server and a server that returns empty results. `GET /api/hotspots` SHALL return 502 Bad Gateway when the upstream server is unreachable (connection refused, timeout, DNS failure). It SHALL return 200 OK with an empty entries array when the server responds with an empty `LiveHotspotsResponse` (e.g., unknown repository or no ingested events).

#### Scenario: Server unreachable returns 502

- **GIVEN** the dashboard is configured for live mode
- **AND** the configured server is not running
- **WHEN** `GET /api/hotspots` is called
- **THEN** the response status is `502 Bad Gateway`
- **AND** the response body is a JSON error object describing the connection failure

#### Scenario: Empty repository returns 200 with empty entries

- **GIVEN** the configured server is running
- **AND** no events have been ingested for the configured `repository_id`
- **WHEN** `GET /api/hotspots` is called
- **THEN** the response status is `200 OK`
- **AND** the response body contains `entries: []`
- **AND** all other envelope fields are present

#### Scenario: Server returns error status

- **GIVEN** the configured server returns a 4xx or 5xx status for the hotspot query
- **WHEN** the dashboard backend proxies the request
- **THEN** the backend SHALL forward the error as `502 Bad Gateway`
- **AND** the response body SHALL include the upstream status code and message

### Requirement: SSE signal stream is backend-proxied with cursor support

The dashboard backend SHALL expose a `GET /api/signals` SSE endpoint in live mode. It SHALL maintain one persistent upstream SSE connection to `GET /v1/repositories/{repository_id}/signals` per connected dashboard client. The endpoint SHALL support an optional `after` query parameter passed through to the upstream server as the cursor. The last-seen signal ID SHALL be stored in-memory per connection in the backend. On backend restart, cursor state SHALL be lost and the next connection SHALL use `after=0`. On upstream disconnection, the proxy SHALL signal the error to the SSE client.

#### Scenario: SSE endpoint delivers signals from upstream server

- **GIVEN** the dashboard is running in live mode
- **WHEN** a client connects to `GET /api/signals`
- **THEN** the backend opens a connection to `GET /v1/repositories/{repo_id}/signals` on the configured server
- **AND** the response Content-Type is `text/event-stream`
- **AND** SSE events from the upstream are relayed to the client

#### Scenario: after cursor is forwarded to upstream

- **GIVEN** a client connects to `GET /api/signals?after=42`
- **WHEN** the backend proxies the request
- **THEN** it connects to `GET /v1/repositories/{repo_id}/signals?after=42`
- **AND** only signals with `id > 42` are replayed

#### Scenario: after=0 replays all signals

- **GIVEN** a client connects to `GET /api/signals?after=0`
- **WHEN** the backend proxies the request
- **THEN** all persisted signals for the repository are replayed in id-ascending order

#### Scenario: Upstream disconnection notifies SSE client

- **GIVEN** a client is connected to `GET /api/signals`
- **WHEN** the upstream server connection drops
- **THEN** the backend SHALL send an SSE error event to the client
- **AND** the backend SHALL NOT silently retry the upstream connection
- **AND** the SSE stream SHALL end

#### Scenario: Cursor state is in-memory per connection

- **GIVEN** a client is connected to `GET /api/signals` and receiving signals
- **WHEN** the dashboard backend process restarts
- **THEN** all in-memory cursor state is lost
- **AND** the next client connection uses `after=0` (full replay)

### Requirement: Sessions and Events views are unavailable in live mode

In live mode, the dashboard SHALL not serve Session or Event data. The backend SHALL return explicit "unavailable in live mode" errors for these endpoints. The frontend SHALL hide Sessions and Events navigation items and SHALL display an explanatory unavailable view when these routes are accessed directly via URL.

#### Scenario: /api/sessions returns unavailable in live mode

- **GIVEN** the dashboard is running in live mode
- **WHEN** `GET /api/sessions` is called
- **THEN** the response status is `404 Not Found`
- **AND** the response body is a JSON error with `error: "Sessions are unavailable in live mode"`

#### Scenario: /api/events returns unavailable in live mode

- **GIVEN** the dashboard is running in live mode
- **WHEN** `GET /api/events` is called
- **THEN** the response status is `404 Not Found`
- **AND** the response body is a JSON error with `error: "Events are unavailable in live mode"`

#### Scenario: Sessions and Events nav items are hidden in live mode

- **GIVEN** `/api/meta` returns `mode: "live"`
- **WHEN** the frontend renders the navigation shell
- **THEN** Sessions and Events navigation items SHALL NOT be visible
- **AND** a Signals navigation item SHALL be visible

#### Scenario: Direct URL navigation to Sessions shows unavailable view

- **GIVEN** the dashboard is in live mode
- **WHEN** the user navigates directly to `/sessions` in the browser address bar
- **THEN** the page SHALL display an explanatory message that Sessions are only available in local mode
- **AND** no error or blank page is shown

### Requirement: Live mode includes a Signals view

The dashboard SHALL include a Signals view accessible via navigation when in live mode. The Signals view SHALL connect to the backend-proxied SSE endpoint (`/api/signals`) and display a timeline of received `HotspotSignal` events. The view SHALL handle connection state (connecting, connected, disconnected, error) explicitly.

#### Scenario: Signals view renders hotspot signal events

- **GIVEN** the dashboard is running in live mode
- **WHEN** the user navigates to the Signals view
- **THEN** an SSE connection is opened to `/api/signals`
- **AND** received `HotspotSignal` events are displayed in a timeline
- **AND** each event shows the signal `id`, `subject`, `kind`, and `score`

#### Scenario: Signals view shows connection state

- **GIVEN** the dashboard is running in live mode
- **WHEN** the user navigates to the Signals view
- **THEN** the view SHALL show a "Connecting..." state while the SSE connection is being established
- **AND** the view SHALL show a "Connected" badge once events are flowing
- **AND** the view SHALL show a "Disconnected" or "Error" state when the SSE connection drops

#### Scenario: Signals view manages cursor for reconnection

- **GIVEN** the user refreshes the Signals page
- **WHEN** the SSE connection is re-established
- **THEN** the client SHALL reconnect with `?after=0` (full replay)
- **AND** previously received signals may be replayed

### Requirement: Subject display in live mode uses raw subject paths

In live mode, hotspot entry subject paths SHALL be displayed as-is from the `HotspotEntry.subject` field. The dashboard SHALL NOT apply filesystem path-shortening that depends on `repositoryPath` from the live server, because `LiveHotspotsResponse` intentionally omits that field. The dashboard MAY use its own CWD-derived `repositoryPath` from `/api/meta` for path shortening if the operator is running the dashboard from the same repository root.

#### Scenario: Raw subjects are displayed in live mode

- **GIVEN** a live hotspot entry has `subject: "src/main.rs"`
- **WHEN** the frontend renders the hotspot table in live mode
- **THEN** the subject is displayed as `"src/main.rs"`
- **AND** no path shortening is applied unless the dashboard CWD provides a matching `repositoryPath`

#### Scenario: Subject display difference is documented

- **WHEN** the About view is rendered in live mode
- **THEN** it SHALL note that subject paths are displayed as received from the server
- **AND** it SHALL explain that this may differ from local mode where paths were shortened relative to the repository root

### Requirement: Footer copy reflects the active dashboard mode

The dashboard shell footer SHALL reflect whether the dashboard is operating in local or live mode. The existing "Local-only viewer for .scryrs artifacts." copy SHALL appear only in local mode.

#### Scenario: Footer shows local mode text

- **GIVEN** `/api/meta` returns `mode: "local"`
- **WHEN** the dashboard shell renders
- **THEN** the footer SHALL display text indicating local artifact mode

#### Scenario: Footer shows live mode text

- **GIVEN** `/api/meta` returns `mode: "live"`
- **WHEN** the dashboard shell renders
- **THEN** the footer SHALL display text indicating live server mode
- **AND** the footer SHALL include the configured repository identity for context

### Requirement: Live dashboard mode is documented in project docs

The live dashboard mode SHALL be documented in the authoritative project documentation. The CLI contract page SHALL describe live mode activation and behavior. The live hotspots documentation SHALL include a dashboard live-mode section. The roadmap SHALL reflect the defined live dashboard contract.

#### Scenario: CLI contract documents live mode

- **WHEN** `.devagent/docs/docs/cli-v0-contract.md` is read
- **THEN** it SHALL describe the `--server-url` and `--repository-id` flags
- **AND** it SHALL document config precedence and the local-mode default
- **AND** it SHALL list the endpoints served in live mode

#### Scenario: Live hotspots docs include dashboard section

- **WHEN** `.devagent/docs/docs/live-hotspots.md` is read
- **THEN** it SHALL include a section on dashboard live mode
- **AND** the section SHALL explain the backend-proxy architecture
- **AND** the section SHALL describe how the dashboard consumes live hotspot and signal APIs

#### Scenario: Roadmap reflects live dashboard contract

- **WHEN** `.devagent/docs/docs/roadmap.mdx` is read
- **THEN** the live dashboard milestone SHALL be marked as having a defined contract
- **AND** the milestone SHALL reference the live dashboard mode spec

### Requirement: End-to-end verification plan is documented

The contract SHALL include a verification plan covering live mode smoke testing, local mode regression, and SSE signal delivery. The plan SHALL extend the existing verification infrastructure under `scripts/verification/`.

#### Scenario: Live mode smoke test is defined

- **WHEN** the verification plan is read
- **THEN** it SHALL describe a test that starts `scryrs server`, starts `scryrs dashboard --server-url <URL> --repository-id <ID>`, and verifies `/api/meta` returns `mode: "live"`
- **AND** it SHALL verify `/api/hotspots` returns live entries
- **AND** it SHALL verify SSE signal delivery via `/api/signals`

#### Scenario: Local mode regression test is defined

- **WHEN** the verification plan is read
- **THEN** it SHALL describe that existing dashboard E2E tests (local artifact reads) continue to pass without live configuration
- **AND** it SHALL verify that `/api/meta` returns `mode: "local"` when no live config is present

#### Scenario: Verification README is updated

- **WHEN** `scripts/verification/README.md` is read
- **THEN** it SHALL document the dashboard live-mode smoke entrypoint
- **AND** it SHALL list prerequisites (running `scryrs server`, configured repository)
- **AND** it SHALL describe the behaviors under test