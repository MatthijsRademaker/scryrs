# live-dashboard-mode Specification

## Purpose
TBD - created by archiving change task-206a6986-5940-4824-a202-c7c759da4548. Update Purpose after archive.
## Requirements
### Requirement: Live dashboard mode is explicit and CLI-activated

The dashboard SHALL enter live mode only when `scryrs dashboard` is started with both `--server-url <URL>` and `--repository-id <ID>`. When both flags are absent, the dashboard SHALL continue to run in its existing local mode. Supplying only one of the two live flags SHALL fail startup with a clear configuration error. Live mode SHALL remain read-only and SHALL NOT merge server data with local `.scryrs` artifacts.

#### Scenario: Full live configuration activates live mode

- **GIVEN** `scryrs dashboard --server-url http://localhost:8081 --repository-id repo-a` is invoked
- **WHEN** the dashboard backend starts
- **THEN** it runs in live mode
- **AND** `/api/meta` returns `mode: "live"`
- **AND** `/api/meta` returns `repositoryId: "repo-a"`

#### Scenario: Local mode remains the default

- **GIVEN** `scryrs dashboard` is invoked without `--server-url` and `--repository-id`
- **WHEN** the dashboard backend starts
- **THEN** it runs in local mode
- **AND** `/api/meta` returns `mode: "local"`
- **AND** existing local hotspot, session, and event behavior remains available

#### Scenario: Partial live configuration fails loudly

- **GIVEN** `scryrs dashboard --server-url http://localhost:8081` is invoked without `--repository-id`
- **WHEN** startup validation runs
- **THEN** the command exits with a non-zero status
- **AND** stderr explains that both `--server-url` and `--repository-id` are required for live mode

#### Scenario: Live mode does not mix local artifacts with server state

- **GIVEN** the dashboard is running in live mode
- **AND** local `.scryrs` artifacts exist in the repository
- **WHEN** live dashboard API endpoints are queried
- **THEN** only live server data is used
- **AND** local `.scryrs` files are not merged or used as fallback

### Requirement: Live mode stays behind the dashboard's same-origin API surface

In live mode, the browser SHALL continue to use same-origin `/api/*` endpoints served by the dashboard backend. The backend SHALL expose live-aware `/api/meta`, `/api/hotspots`, and `/api/signals` endpoints. `GET /api/hotspots` SHALL proxy `GET /v1/repositories/{repository_id}/hotspots?window=cumulative` to the configured live server and normalize the response into a shape compatible with the current hotspot UI while preserving the live `cursor`. The dashboard SHALL add no mutation behavior.

#### Scenario: /api/hotspots proxies the live rankings endpoint

- **GIVEN** the dashboard is running in live mode for repository `repo-a`
- **WHEN** `GET /api/hotspots` is called
- **THEN** the backend requests `GET /v1/repositories/repo-a/hotspots?window=cumulative` from the configured server
- **AND** the response is normalized for the existing hotspot rendering path
- **AND** the live `cursor` value is preserved in the dashboard response

#### Scenario: /api/meta returns live dashboard context

- **GIVEN** the dashboard is running in live mode
- **WHEN** `GET /api/meta` is called
- **THEN** the response includes `mode: "live"`
- **AND** the response includes the configured `repositoryId`
- **AND** the response includes `repositoryPath` for dashboard-local context

#### Scenario: Upstream-unavailable and empty-live results are distinguished

- **GIVEN** the dashboard is running in live mode
- **WHEN** the configured live server is unreachable during `GET /api/hotspots`
- **THEN** the dashboard responds with `502 Bad Gateway`
- **AND** the response body describes the upstream failure

- **GIVEN** the configured live server responds successfully with no hotspot entries for the repository
- **WHEN** `GET /api/hotspots` is called
- **THEN** the dashboard responds with `200 OK`
- **AND** the response body contains an empty entries list rather than an upstream-unavailable error

#### Scenario: Local-only endpoints are unavailable in live mode

- **GIVEN** the dashboard is running in live mode
- **WHEN** `GET /api/sessions` or `GET /api/events` is called
- **THEN** the dashboard responds with `404 Not Found`
- **AND** the error body explains that the endpoint is unavailable in live mode

### Requirement: Live signal streaming replays history and appends new signals without buffering

In live mode, the dashboard SHALL expose `GET /api/signals` as a backend-proxied SSE endpoint for hotspot signals. The endpoint SHALL forward an optional `after` query parameter to `GET /v1/repositories/{repository_id}/signals` on the configured live server. The proxy SHALL stream replayed and live events through to the browser as they arrive; it SHALL NOT buffer the full upstream response before forwarding.

#### Scenario: after cursor is forwarded to the live server

- **GIVEN** the dashboard is running in live mode for repository `repo-a`
- **WHEN** a browser client connects to `GET /api/signals?after=42`
- **THEN** the dashboard opens an upstream connection to `GET /v1/repositories/repo-a/signals?after=42`
- **AND** only signals with `id > 42` are replayed before live tailing continues

#### Scenario: Initial connect replays persisted signals

- **GIVEN** the dashboard is running in live mode
- **WHEN** a browser client connects to `GET /api/signals?after=0`
- **THEN** persisted signals for the repository are replayed in id order
- **AND** newly committed signals continue on the same stream without polling gaps

#### Scenario: The dashboard surfaces stream failure instead of hiding it

- **GIVEN** a browser client is connected to `GET /api/signals`
- **WHEN** the upstream signal stream cannot be established or terminates unexpectedly
- **THEN** the dashboard does not silently retry upstream on behalf of the browser
- **AND** the browser can transition the UI into disconnected, reconnecting, or error state

### Requirement: The frontend owns cursor-based reconnection for the current page lifecycle

The frontend SHALL manage signal stream reconnect behavior explicitly rather than relying on native `EventSource` auto-reconnect. It SHALL track the last seen signal id in memory for the current page lifecycle, reconnect with `?after=<lastSeenId>` after disconnect, and avoid appending replayed duplicates twice. A full page refresh SHALL start from `after=0`.

#### Scenario: Reconnect resumes from the last seen signal id

- **GIVEN** the Signals view has already received signal id `57`
- **WHEN** the stream disconnects and the frontend reconnects
- **THEN** the frontend opens `/api/signals?after=57`
- **AND** previously received signals are not appended again

#### Scenario: Full page refresh restarts from full replay

- **GIVEN** the Signals view has already received one or more signals
- **WHEN** the user performs a full browser refresh
- **THEN** the next connection starts at `/api/signals?after=0`
- **AND** persisted signal history may be replayed again

#### Scenario: Connection state is explicit in the client model

- **GIVEN** the Signals view is active in live mode
- **WHEN** the stream is connecting, connected, interrupted, reconnecting, or terminally failed
- **THEN** the client model exposes those states so the UI can render them explicitly

### Requirement: Live dashboard navigation and views are mode-aware

In live mode, the dashboard SHALL render live hotspot rankings and a Signals timeline without implying local artifact files as the data source. The Signals view SHALL display `HotspotSignal` items with id, subject, kind, score, threshold or delta, and timestamp using the existing dashboard frontend stack and shadcn-vue primitives. Sessions and Events SHALL be hidden from live navigation, and direct navigation to those routes SHALL show an explanatory unavailable view instead of a blank or broken page. Footer and About copy SHALL reflect the active mode.

#### Scenario: Signals navigation is available only in live mode

- **GIVEN** `/api/meta` returns `mode: "live"`
- **WHEN** the dashboard shell renders navigation
- **THEN** a Signals navigation entry is visible
- **AND** Sessions and Events navigation entries are hidden

#### Scenario: Signals timeline renders replayed and live hotspot signals

- **GIVEN** the user opens the Signals view in live mode
- **WHEN** replayed or live `HotspotSignal` events are received
- **THEN** the timeline appends them in order
- **AND** each item shows id, subject, kind, score, threshold or delta, and timestamp

#### Scenario: Hotspots copy no longer implies local artifact input in live mode

- **GIVEN** the user opens the Hotspots view while the dashboard is in live mode
- **WHEN** the rankings table renders
- **THEN** the view does not describe `.scryrs/hotspots.json` as the source
- **AND** subjects are shown as received from the server by default

#### Scenario: Direct navigation to local-only routes stays readable

- **GIVEN** the dashboard is in live mode
- **WHEN** the user navigates directly to `/sessions` or `/events`
- **THEN** the page shows a clear read-only unavailable message
- **AND** the app does not render a blank page or an unhandled error state

#### Scenario: Connection state is visible in the Signals UI

- **GIVEN** the Signals view is active
- **WHEN** the stream transitions between connecting, connected, disconnected or reconnecting, and error
- **THEN** the UI shows the current connection state clearly

### Requirement: Local dashboard behavior remains intact and the live workflow is documented

The live dashboard change SHALL preserve existing local dashboard behavior and local dashboard test coverage. It SHALL also update project documentation to describe live dashboard startup, rankings, signal replay and reconnect behavior, and the difference between local and live mode.

#### Scenario: Existing local dashboard tests still pass

- **GIVEN** no live dashboard flags are provided
- **WHEN** the existing local dashboard backend and CLI tests are run
- **THEN** they continue to pass without requiring live server configuration

#### Scenario: Live dashboard docs are updated

- **WHEN** `.devagent/docs/docs/cli-v0-contract.md`, `.devagent/docs/docs/live-hotspots.md`, `.devagent/docs/docs/roadmap.mdx`, and `scripts/verification/README.md` are reviewed
- **THEN** they describe live dashboard startup and configuration
- **AND** they describe rankings fetch, signal timeline replay, reconnect cursor behavior, and local-vs-live workflow differences

#### Scenario: Live dashboard verification flow is documented

- **WHEN** the verification documentation is reviewed
- **THEN** it includes a live dashboard smoke path that starts the server and dashboard in live mode
- **AND** it verifies live rankings fetch plus signal replay/resume behavior

