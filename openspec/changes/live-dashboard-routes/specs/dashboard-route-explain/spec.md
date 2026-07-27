## MODIFIED Requirements

### Requirement: Dashboard backend exposes a route explain endpoint

The dashboard backend SHALL expose `GET /api/routes/explain?query=<text>` as a read-only deterministic endpoint. In local mode it SHALL use `.scryrs/routes.json`; in live mode it SHALL proxy the configured repository's latest published route manifest through the live server. Both modes SHALL return the same `RouteHintDocument` contract and deterministic `scryrs_runtime::explain_hints` results.

#### Scenario: Local query uses local manifest

- **GIVEN** the dashboard is running in local mode with a valid `.scryrs/routes.json`
- **WHEN** route explain is requested
- **THEN** the response is generated from the local manifest

#### Scenario: Live query uses published manifest

- **GIVEN** the dashboard is running in live mode for repository `repo-a`
- **WHEN** route explain is requested
- **THEN** the dashboard proxies the request to the live server
- **AND** it does not inspect local route artifacts

### Requirement: Route view shows unavailable card in live mode

The Route view SHALL render a functional search interface in both local and live modes. When live mode has no published manifest or the upstream route service fails, it SHALL render an actionable error state rather than an unconditional unavailable card.

#### Scenario: Live route view is usable

- **GIVEN** the dashboard is running in live mode
- **AND** a route manifest is published for its repository
- **WHEN** the user opens `/routes`
- **THEN** the search input is available
- **AND** submitting a query requests live route hints

#### Scenario: Live route view explains missing publication

- **GIVEN** the dashboard is running in live mode
- **AND** no route manifest is published
- **WHEN** the user searches
- **THEN** the view explains that a manifest must be published

### Requirement: Route navigation is local-mode only

The Route navigation entry SHALL appear in both local and live navigation. The data source SHALL be selected by dashboard mode: local `.scryrs/routes.json` in local mode and the repository's published manifest through the live server in live mode.

#### Scenario: Route nav appears in local mode

- **GIVEN** the dashboard is in local mode
- **WHEN** navigation renders
- **THEN** a Route entry is visible

#### Scenario: Route nav appears in live mode

- **GIVEN** the dashboard is in live mode
- **WHEN** navigation renders
- **THEN** a Route entry is visible
