## ADDED Requirements

### Requirement: Clients can publish validated route manifests per repository

The live server SHALL expose an authenticated publication endpoint for a `RouteManifestDocument`. It SHALL validate JSON, `ROUTE_SCHEMA_VERSION`, route references, repository identity, and configured size limits before atomically replacing the repository's latest manifest.

#### Scenario: Valid manifest is published

- **GIVEN** an authenticated publisher for repository `repo-a`
- **AND** the request contains a valid `RouteManifestDocument`
- **WHEN** the client publishes the manifest
- **THEN** the server stores it as the latest manifest for `repo-a`
- **AND** the response includes schema and publication metadata

#### Scenario: Invalid manifest is rejected without replacement

- **GIVEN** repository `repo-a` already has a valid manifest
- **WHEN** a client publishes malformed JSON, an incompatible schema, or an invalid route reference
- **THEN** the server returns a client error
- **AND** the existing manifest remains unchanged

### Requirement: Live server exposes deterministic route explain

The live server SHALL expose `GET /v1/repositories/{repositoryId}/routes/explain?query=<text>` as a read-only endpoint over the repository's latest validated manifest. It SHALL use the existing deterministic `scryrs_runtime::explain_hints` behavior.

#### Scenario: Published manifest produces route hints

- **GIVEN** repository `repo-a` has a valid published manifest
- **WHEN** a client requests route explain with query `auth`
- **THEN** the response is a `RouteHintDocument`
- **AND** matching, ranking, reasons, evidence, and load targets match `scryrs_runtime::explain_hints`

#### Scenario: Missing manifest is explicit

- **GIVEN** repository `repo-a` has no published route manifest
- **WHEN** route explain is requested
- **THEN** the server returns `404 Not Found`
- **AND** the error instructs the client to publish a route manifest

#### Scenario: Empty query is rejected

- **WHEN** route explain is requested without a non-empty query
- **THEN** the server returns `400 Bad Request`
- **AND** no manifest matching operation runs

### Requirement: Dashboard proxies live route explain

In live mode, the dashboard SHALL proxy `/api/routes/explain?query=<text>` to the configured repository-scoped live route-explain endpoint. It SHALL not read `.scryrs/routes.json` or fall back to local artifacts.

#### Scenario: Live route search is proxied

- **WHEN** the browser searches `auth` in the live Routes view
- **THEN** the dashboard requests the live server route-explain endpoint
- **AND** returns a compatible `RouteHintDocument`

#### Scenario: Missing live manifest is actionable

- **WHEN** the live server reports that no manifest is published
- **THEN** the dashboard displays an actionable publication error

### Requirement: Live Routes navigation and search are available

When live mode is active, the dashboard SHALL expose Routes navigation and render the existing search/results interface against live route hints. Local mode SHALL continue using `.scryrs/routes.json`.

#### Scenario: Routes entry appears in live navigation

- **GIVEN** `/api/meta` returns `mode: "live"`
- **WHEN** navigation renders
- **THEN** a Routes entry is visible

#### Scenario: Live route results render existing fields

- **WHEN** a live route query returns hints
- **THEN** the view renders rank, label, target, load target, relevance, reason, and evidence
