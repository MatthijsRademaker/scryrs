## MODIFIED Requirements

### Requirement: Local-only endpoints are unavailable in live mode

The dashboard SHALL never mix local `.scryrs` artifacts with live server state. In live mode, sessions and events SHALL use their repository-scoped live server APIs, while proposal and route endpoints SHALL remain unavailable until their separate live contracts are implemented.

#### Scenario: Live sessions and events use server data

- **GIVEN** the dashboard is running in live mode
- **WHEN** `GET /api/sessions` or `GET /api/events` is called
- **THEN** the dashboard proxies the request to the configured live server
- **AND** local `.scryrs` files are not read

#### Scenario: Unimplemented live capabilities remain explicit

- **GIVEN** the dashboard is running in live mode
- **WHEN** `GET /api/routes/explain` or `GET /api/proposals` is called
- **THEN** the dashboard returns `404 Not Found`
- **AND** the error explains that the capability is unavailable in live mode

### Requirement: Live dashboard navigation and views are mode-aware

In live mode, the dashboard SHALL render live hotspot rankings, Signals, Sessions, and Events using server-backed data. Routes and Proposals SHALL remain hidden from live navigation until their live contracts are implemented. Direct navigation to unavailable routes SHALL show an explanatory view.

#### Scenario: Live navigation includes server-backed views

- **GIVEN** `/api/meta` returns `mode: "live"`
- **WHEN** the dashboard shell renders navigation
- **THEN** Hotspots, Signals, Sessions, Events, and About entries are visible
- **AND** Routes and Proposals entries are hidden

#### Scenario: Direct navigation to unavailable routes stays readable

- **GIVEN** the dashboard is in live mode
- **WHEN** the user navigates directly to `/routes` or `/proposals`
- **THEN** the page shows a clear unavailable message
- **AND** no local artifact fallback occurs
