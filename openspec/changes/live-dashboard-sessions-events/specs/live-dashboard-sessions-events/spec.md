## ADDED Requirements

### Requirement: Live server exposes repository-scoped session summaries

The live server SHALL expose `GET /v1/repositories/{repositoryId}/sessions` as a read-only endpoint over server-owned trace events. It SHALL return deterministic session summaries scoped to the requested repository, ordered by most recent activity, with a bounded `limit` and a cursor for subsequent pages.

#### Scenario: Session list returns repository-scoped summaries

- **WHEN** a client requests sessions for repository `repo-a`
- **THEN** the server returns only sessions whose events belong to `repo-a`
- **AND** each row includes session ID, start time, end time when known, event count, and source
- **AND** results are deterministically ordered

#### Scenario: Session list enforces bounded pagination

- **WHEN** a client supplies a limit above the supported maximum
- **THEN** the server clamps or rejects the value according to the documented API contract
- **AND** the server never executes an unbounded session query

### Requirement: Live server exposes repository-scoped session detail

The live server SHALL expose `GET /v1/repositories/{repositoryId}/sessions/{sessionId}` as a read-only endpoint returning the selected session summary and its ordered trace events. A session from another repository SHALL be indistinguishable from a missing session.

#### Scenario: Session detail returns ordered events

- **GIVEN** session `session-1` belongs to repository `repo-a`
- **WHEN** a client requests that session under `repo-a`
- **THEN** the response includes the session summary and events ordered by server event ID ascending
- **AND** each event includes ID, session ID, type, timestamp, subject fields, and parsed payload

#### Scenario: Cross-repository session is not disclosed

- **GIVEN** session `session-1` belongs to repository `repo-a`
- **WHEN** a client requests it under repository `repo-b`
- **THEN** the server returns not found
- **AND** no event or session metadata is disclosed

### Requirement: Live server exposes cursor-paginated events

The live server SHALL expose `GET /v1/repositories/{repositoryId}/events` as a read-only cursor-paginated endpoint over `server_trace_events`. It SHALL support bounded limits, an event-ID cursor, optional session filtering, stable ordering, and repository scoping.

#### Scenario: Event page returns stable cursor results

- **WHEN** a client requests events for `repo-a` with limit 50 and a cursor after event ID 100
- **THEN** the response contains only events for `repo-a` after ID 100
- **AND** the response includes a cursor when more results exist
- **AND** repeated requests against unchanged data return identical pages

#### Scenario: Event query supports session filtering

- **WHEN** a client requests events for `repo-a` with session ID `session-1`
- **THEN** every returned event belongs to both `repo-a` and `session-1`

### Requirement: Dashboard proxies live sessions and events through same-origin APIs

In live mode, the dashboard SHALL proxy `/api/sessions`, `/api/sessions/:sessionId`, and `/api/events` to the configured server endpoints. It SHALL preserve existing frontend DTO shapes, map upstream failures to explicit gateway errors, and never read local `.scryrs` databases as fallback.

#### Scenario: Live sessions request is proxied

- **WHEN** the browser requests `/api/sessions?limit=25` in live mode
- **THEN** the dashboard requests the matching repository-scoped server endpoint
- **AND** returns a response compatible with the existing Sessions view

#### Scenario: Live event upstream failure is visible

- **WHEN** the configured live server is unreachable during an events request
- **THEN** the dashboard returns `502 Bad Gateway`
- **AND** the response identifies the upstream failure

#### Scenario: Live mode does not fall back to local files

- **GIVEN** local `.scryrs/scryrs.db` exists beside the dashboard process
- **WHEN** a live sessions or events request is made
- **THEN** only live server data is used

### Requirement: Live navigation exposes sessions and events

When `/api/meta` reports live mode, the dashboard SHALL show Sessions and Events navigation entries and SHALL render their live data through the proxy APIs. Routes and Proposals SHALL remain unavailable until their separate live contracts are implemented.

#### Scenario: Live Sessions navigation renders data

- **WHEN** the user opens Sessions in live mode
- **THEN** the Sessions navigation entry is visible
- **AND** the view renders server-backed summaries or an explicit error/empty state

#### Scenario: Live Events navigation renders data

- **WHEN** the user opens Events in live mode
- **THEN** the Events navigation entry is visible
- **AND** the view renders server-backed paginated events or an explicit error/empty state
