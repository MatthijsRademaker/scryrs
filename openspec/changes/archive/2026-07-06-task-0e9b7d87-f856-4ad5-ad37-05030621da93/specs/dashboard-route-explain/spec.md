# dashboard-route-explain Specification

## Purpose
Exposes the existing deterministic route explain logic (`scryrs_runtime::explain_hints`) as a read-only dashboard API endpoint and Vue view, enabling users to inspect what future agents would load and why without leaving the browser. Local-mode only; live mode shows an explicit unavailability card.

## ADDED Requirements
### Requirement: Dashboard backend exposes a route explain endpoint

The dashboard backend SHALL expose `GET /api/routes/explain?query=<text>` as a read-only, deterministic endpoint. The endpoint SHALL be available only in local mode and SHALL delegate route-hint matching to `scryrs_runtime::explain_hints` over `.scryrs/routes.json`.

#### Scenario: Successful query returns RouteHintDocument

- **GIVEN** the dashboard is running in local mode
- **AND** `.scryrs/routes.json` exists with valid `schemaVersion` matching `ROUTE_SCHEMA_VERSION`
- **WHEN** `GET /api/routes/explain?query=auth` is called
- **THEN** the response status is 200
- **AND** the response body is a valid `RouteHintDocument` JSON with populated `hints`
- **AND** each returned `RouteHintItem` has `rank`, `relevance`, `reason`, `evidence`, `target`, `loadTarget`, and `label` fields
- **AND** each `reason` includes the base template with load-target kind and appends `; query match on <fields>`

#### Scenario: Empty query is rejected with 400

- **GIVEN** the dashboard is running in local mode
- **WHEN** `GET /api/routes/explain?query=` (empty query) is called
- **THEN** the response status is 400
- **AND** the error body describes that a non-empty query is required

#### Scenario: Missing query parameter is rejected with 400

- **GIVEN** the dashboard is running in local mode
- **WHEN** `GET /api/routes/explain` (no query parameter) is called
- **THEN** the response status is 400
- **AND** the error body describes that a query parameter is required

#### Scenario: Zero-match query returns empty hints

- **GIVEN** the dashboard is running in local mode
- **AND** `.scryrs/routes.json` exists and is valid
- **WHEN** `GET /api/routes/explain?query=zzz_nonexistent` is called where no route entry matches
- **THEN** the response status is 200
- **AND** the response body is a valid `RouteHintDocument` with an empty `hints` array

#### Scenario: Live mode returns 404 unavailable

- **GIVEN** the dashboard is running in live mode
- **WHEN** `GET /api/routes/explain?query=auth` is called
- **THEN** the response status is 404
- **AND** the error body states route explain is unavailable in live mode

### Requirement: Route explain endpoint fails fast for missing or malformed artifacts

The backend SHALL return distinct HTTP error states for missing `.scryrs/routes.json`, invalid JSON, and schema version mismatch, following the existing dashboard error patterns.

#### Scenario: Missing route artifact returns 404 with remediation

- **GIVEN** the dashboard is running in local mode
- **AND** `.scryrs/routes.json` does not exist
- **WHEN** `GET /api/routes/explain?query=auth` is called
- **THEN** the response status is 404
- **AND** the error body contains a message indicating the route artifact is not found
- **AND** the message references running `scryrs route <PATH>` to generate the manifest

#### Scenario: Malformed route artifact returns 502

- **GIVEN** the dashboard is running in local mode
- **AND** `.scryrs/routes.json` contains invalid JSON
- **WHEN** `GET /api/routes/explain?query=auth` is called
- **THEN** the response status is 502
- **AND** the error body describes the parse failure

#### Scenario: Schema version mismatch returns 502

- **GIVEN** the dashboard is running in local mode
- **AND** `.scryrs/routes.json` has `schemaVersion` not equal to `ROUTE_SCHEMA_VERSION`
- **WHEN** `GET /api/routes/explain?query=auth` is called
- **THEN** the response status is 502
- **AND** the error body indicates a schema version mismatch

### Requirement: Route view renders in local mode with search input and results

The dashboard SHALL provide a Vue Route view at `/routes` that accepts a search query, calls `GET /api/routes/explain`, and renders matched hints with their full `RouteHintItem` fields.

#### Scenario: Route view renders search input and prompt

- **GIVEN** the dashboard is running in local mode
- **WHEN** the user navigates to `/routes`
- **THEN** a search input is visible
- **AND** a prompt instructs the user to enter a query to search route hints
- **AND** the search button is disabled when the input is empty

#### Scenario: Successful search renders hint results

- **GIVEN** the dashboard is in local mode and `.scryrs/routes.json` contains entries
- **WHEN** the user enters `auth` and submits the search
- **THEN** matching route hints render with `rank`, `label`, `target`, `loadTarget.kind`, `relevance`, and `reason`
- **AND** evidence citations render as plain-text `sourceKind` / `subject` / `rowIds`

#### Scenario: Zero-match search shows empty state

- **GIVEN** a search returns zero results
- **WHEN** results are displayed
- **THEN** an empty state message is shown indicating no routes matched the query
- **AND** no error styling is applied

#### Scenario: Empty query does not trigger API call

- **GIVEN** the Route view is rendered
- **WHEN** the user attempts to submit with an empty search input
- **THEN** no API call is made
- **AND** the search button remains disabled

### Requirement: Route view shows actionable guidance for missing or malformed artifacts

The Route view SHALL display distinct, actionable messages when `.scryrs/routes.json` is missing, malformed, or has a schema version mismatch.

#### Scenario: Missing artifact shows remediation guidance

- **GIVEN** the dashboard is in local mode and `.scryrs/routes.json` does not exist
- **WHEN** the user navigates to `/routes`
- **THEN** a message is displayed stating the route artifact was not found
- **AND** the message instructs the user to run `scryrs route <PATH>` to generate the manifest

#### Scenario: Malformed or schema-mismatch artifact shows error state

- **GIVEN** `.scryrs/routes.json` is malformed or has an incompatible schema version
- **WHEN** the user performs a search
- **THEN** an error message is displayed indicating the artifact is malformed or from an incompatible version
- **AND** the message instructs the user to run `scryrs route <PATH>` to regenerate

### Requirement: Route view shows unavailable card in live mode

The Route view SHALL display an explicit "Unavailable in live mode" card when the dashboard is running in live mode, following the existing Sessions/Events pattern.

#### Scenario: Live mode shows unavailable card

- **GIVEN** the dashboard is running in live mode
- **WHEN** the user navigates to `/routes`
- **THEN** an "Unavailable in live mode" card is displayed
- **AND** the card describes that route explain requires local `.scryrs` artifacts

### Requirement: Route navigation is local-mode only

The Route navigation entry SHALL appear in local-mode navigation and SHALL NOT appear in live-mode navigation. The entry SHALL be present even when `.scryrs/routes.json` does not exist.

#### Scenario: Route nav entry appears in local mode

- **GIVEN** the dashboard is in local mode
- **WHEN** the navigation sidebar renders
- **THEN** a Route entry is visible

#### Scenario: Route nav entry does not appear in live mode

- **GIVEN** the dashboard is in live mode
- **WHEN** the navigation sidebar renders
- **THEN** no Route entry is visible

#### Scenario: Route nav entry appears when artifact is missing

- **GIVEN** the dashboard is in local mode
- **AND** `.scryrs/routes.json` does not exist
- **WHEN** the navigation sidebar renders
- **THEN** the Route entry is visible

### Requirement: Frontend data contract mirrors the RouteHintDocument wire contract

The dashboard frontend SHALL define hand-written TypeScript interfaces for `RouteHintDocument`, `RouteHintItem`, `EvidenceLink`, and `RouteLoadTarget` matching the Rust `scryrs_types` camelCase serialization. The frontend SHALL provide a typed `getRouteHints(query)` client function.

#### Scenario: TypeScript DTOs match the wire contract

- **WHEN** the frontend API client types are inspected
- **THEN** `RouteHintDocument` has `schemaVersion` (string) and `hints` (RouteHintItem[])
- **AND** `RouteHintItem` has `routeId`, `target`, `loadTarget?`, `label`, `rank`, `relevance?`, `reason`, and `evidence?` fields
- **AND** `EvidenceLink` has `sourceKind`, `subject`, `rowIds`, `docRef?`, `description?`, `score?`, and `metadata?` fields

#### Scenario: getRouteHints calls the route explain endpoint

- **WHEN** `getRouteHints("auth")` is called
- **THEN** a GET request is made to `/api/routes/explain?query=auth`
- **AND** non-2xx responses surface typed `ApiError` instances

### Requirement: Route explain endpoint does not mutate artifacts

The `GET /api/routes/explain` handler SHALL be a read-only operation. It SHALL NOT create, modify, or delete `.scryrs/routes.json` or any other filesystem artifact.

#### Scenario: Route artifact is unchanged after explain request

- **GIVEN** `.scryrs/routes.json` exists with known content
- **WHEN** `GET /api/routes/explain?query=auth` is called
- **THEN** `.scryrs/routes.json` is byte-identical to its pre-request content

### Requirement: CLI help surfaces document the route explain endpoint

The CLI help output (`scryrs dashboard --help`), machine-readable surface (`--help-json`), and help text SHALL include `GET /api/routes/explain` in the dashboard REST API endpoint list.

#### Scenario: Dashboard help includes route explain endpoint

- **WHEN** `scryrs dashboard --help` is run
- **THEN** the REST API section includes `GET /api/routes/explain (local mode only)`

#### Scenario: Help-json dashboard output includes route explain

- **WHEN** `scryrs --help-json` is run and the dashboard command output is inspected
- **THEN** the REST API description includes `GET /api/routes/explain`

### Requirement: Frontend tests cover route store and client behavior

The dashboard frontend SHALL have Vitest-based tests covering the route hints store loading, error, and data states, plus client query building and error propagation.

#### Scenario: Store search dispatches API call

- **GIVEN** a mocked `getRouteHints` that returns a valid `RouteHintDocument`
- **WHEN** the route store `search("auth")` action is called
- **THEN** `loading` transitions from false → true → false
- **AND** `hints` is populated with the returned hints
- **AND** `error` is null

#### Scenario: Store handles API error

- **GIVEN** a mocked `getRouteHints` that throws `ApiError`
- **WHEN** the route store `search("auth")` action is called
- **THEN** `loading` transitions from false → true → false
- **AND** `error` is set to the error message
- **AND** `hints` is an empty array

#### Scenario: Store skips API call for empty query

- **WHEN** the route store `search("")` action is called
- **THEN** no API call is made
- **AND** `hints` remains unchanged

#### Scenario: Navigation gating returns correct items per mode

- **GIVEN** `navigationForMode("local")` is called
- **THEN** the returned nav items include a Route entry
- **GIVEN** `navigationForMode("live")` is called
- **THEN** the returned nav items do not include a Route entry

### Requirement: Backend reuses scryrs-runtime without duplicating explain logic

The dashboard backend SHALL depend on `scryrs-runtime` and call `scryrs_runtime::explain_hints`. It SHALL NOT reimplement case-insensitive substring matching, tiered ranking, or result ordering.

#### Scenario: Dashboard handler delegates to explain_hints

- **WHEN** the route explain handler processes a query
- **THEN** it calls `scryrs_runtime::explain_hints(&manifest, &query)`
- **AND** returns the resulting `RouteHintDocument` directly

### Requirement: Evidence citations render as plain text in the Route view

Evidence links in the Route view SHALL render as human-readable plain-text citations showing `sourceKind`, `subject`, and `rowIds`. They SHALL NOT be clickable hyperlinks.

#### Scenario: Evidence renders as plain text

- **GIVEN** a route hint with an evidence link having `sourceKind: "localTraceRow"`, `subject: "auth_handler"`, `rowIds: [1, 2]`
- **WHEN** the Route view renders the hint
- **THEN** the evidence citation displays as plain text
- **AND** it is not wrapped in an anchor tag

### Requirement: Route explain endpoint is deterministic

The `GET /api/routes/explain` endpoint SHALL produce byte-identical output for identical inputs (same `.scryrs/routes.json` and same query). Output SHALL NOT include wall-clock timestamps, random identifiers, or iteration-order-dependent content.

#### Scenario: Repeated requests produce identical output

- **GIVEN** the same `.scryrs/routes.json` artifact
- **WHEN** `GET /api/routes/explain?query=auth` is called twice
- **THEN** both responses are byte-identical
