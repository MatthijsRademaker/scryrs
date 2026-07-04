## ADDED Requirements

### Requirement: scryrs dashboard accepts flags for port, bind address, and browser behavior

The `scryrs dashboard` command SHALL accept optional flags to configure the HTTP server. The default port SHALL be `8080`, the default bind address SHALL be `127.0.0.1`, and the user SHALL be able to suppress automatic browser opening.

#### Scenario: Default invocation starts server on port 8080

- **WHEN** a user runs `scryrs dashboard` with no flags
- **THEN** the HTTP server starts on `127.0.0.1:8080`
- **AND** the default browser opens to `http://127.0.0.1:8080`
- **AND** the server prints a startup message to stderr: "Dashboard available at <http://127.0.0.1:8080>"

#### Scenario: Custom port via --port flag

- **WHEN** a user runs `scryrs dashboard --port 9090`
- **THEN** the HTTP server starts on `127.0.0.1:9090`

#### Scenario: Custom bind address via --bind flag

- **WHEN** a user runs `scryrs dashboard --bind 0.0.0.0`
- **THEN** the HTTP server starts on `0.0.0.0:8080`

#### Scenario: Browser opening suppressed with --no-open

- **WHEN** a user runs `scryrs dashboard --no-open`
- **THEN** the HTTP server starts on `127.0.0.1:8080`
- **AND** the default browser is NOT opened

#### Scenario: Short flags are supported

- **WHEN** a user runs `scryrs dashboard -p 9090 -b 0.0.0.0`
- **THEN** the server starts on `0.0.0.0:9090`

#### Scenario: Port already in use exits with error

- **WHEN** a user runs `scryrs dashboard --port 8080` and port 8080 is already in use
- **THEN** the command exits with code 1
- **AND** stderr prints an error message indicating the port is unavailable

#### Scenario: Development mode serves from filesystem

- **WHEN** a user runs `scryrs dashboard --dev`
- **THEN** the server serves the SPA from `crates/scryrs-dashboard/frontend/dist/` relative to the repository root
- **AND** it does NOT use embedded assets
- **AND** the startup message includes "(dev mode)" in the output

### Requirement: scryrs dashboard serves REST API endpoints

The server SHALL expose REST API endpoints that read from the local `.scryrs/` store and artifact files. The SPA SHALL be a client-side application that fetches data exclusively through these endpoints.

#### Scenario: GET /api/hotspots returns the hotspot report

- **WHEN** a client sends `GET /api/hotspots`
- **THEN** the server responds with `200 OK` and the JSON content of `.scryrs/hotspots.json` as the body
- **AND** the Content-Type SHALL be `application/json`
- **AND** if `.scryrs/hotspots.json` does not exist, the server responds with `404 Not Found` and a JSON error body

#### Scenario: GET /api/sessions returns session metadata

- **WHEN** a client sends `GET /api/sessions`
- **THEN** the server queries `.scryrs/scryrs.db` for session start time, end time, event count, and source
- **AND** responds with `200 OK` and a JSON array of session objects
- **AND** each session object SHALL include `sessionId` (string), `startedAt` (ISO 8601), `endedAt` (ISO 8601 or null), `eventCount` (integer), and `source` (string)
- **AND** results are ordered by `startedAt DESC` with a default limit of 50

#### Scenario: GET /api/events returns events with cursor-based pagination

- **WHEN** a client sends `GET /api/events?limit=20&cursor=<opaque_token>`
- **THEN** the server queries `.scryrs/scryrs.db` for trace events
- **AND** responds with `200 OK` and a JSON object containing `events` (array) and `nextCursor` (string or null)
- **AND** each event SHALL include `eventId`, `eventType`, `timestamp`, `subjectKind`, `subject`, and `payload`
- **AND** if no cursor is provided, the response starts from the most recent events

#### Scenario: Missing or unreadable store returns 502 or 404

- **WHEN** `.scryrs/scryrs.db` does not exist
- **THEN** `GET /api/sessions` and `GET /api/events` SHALL respond with `404 Not Found`
- **AND** the error body SHALL be JSON with an `error` field explaining the store is missing
- **WHEN** the SQLite store is unreadable or corrupt
- **THEN** the endpoints SHALL respond with `502 Bad Gateway`
- **AND** the error body SHALL be JSON with an `error` field

### Requirement: scryrs dashboard serves the SPA static assets

The server SHALL serve the embedded Vue.js SPA as static files. All non-API requests SHALL serve the SPA (`index.html` for push-state routing).

#### Scenario: Root path serves the SPA

- **WHEN** a client sends `GET /`
- **THEN** the server responds with `200 OK` and the content of the SPA's `index.html`
- **AND** the Content-Type SHALL be `text/html`

#### Scenario: Asset paths serve static files

- **WHEN** a client sends `GET /assets/app-abc123.js`
- **THEN** the server responds with `200 OK`, the file content, and the correct Content-Type based on file extension
- **AND** if the asset does not exist, the server responds with `404 Not Found`

#### Scenario: Unknown paths fall through to SPA index.html

- **WHEN** a client sends `GET /sessions` or any non-API, non-asset path
- **THEN** the server responds with the SPA's `index.html` content (for Vue Router push-state)
- **AND** the Content-Type SHALL be `text/html`

### Requirement: scryrs dashboard is listed in help and --help-json

The `scryrs dashboard` command SHALL be discoverable through the existing CLI discovery surface.

#### Scenario: --help lists dashboard

- **WHEN** a user runs `scryrs --help`
- **THEN** the help text SHALL include `dashboard` in the available commands list
- **AND** the help text SHALL include a one-line description of the command

#### Scenario: --help-json includes dashboard

- **WHEN** a user runs `scryrs --help-json`
- **THEN** the JSON surface document SHALL include a `dashboard` entry under `commands`
- **AND** the entry SHALL list the supported flags (`port`, `bind`, `no-open`, `dev`) with their types and defaults

#### Scenario: scryrs dashboard --help prints command-specific help

- **WHEN** a user runs `scryrs dashboard --help`
- **THEN** the help text SHALL describe all available flags and their defaults
- **AND** the description SHALL state that the command starts a local dashboard server
