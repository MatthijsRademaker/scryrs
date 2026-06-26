## MODIFIED Requirements

### Requirement: Live hotspot query returns server-authoritative state

The system SHALL define a read-only `GET /v1/repositories/{repository_id}/hotspots` endpoint that returns live hotspot state from server-owned state rather than artifact files. The response SHALL use a JSON `LiveHotspotsResponse` envelope and SHALL support `window` plus optional `session_id` query parameters. For this task, `window` SHALL accept only `"cumulative"`; any other value SHALL return `400 Bad Request` with a descriptive error. When `session_id` is omitted, the server SHALL materialize cumulative rankings from repository live state. When `session_id` is provided, the server SHALL recompute rankings from matching `server_trace_events` rows using the existing deterministic hotspot scoring path and SHALL NOT derive session-scoped results by filtering aggregate accumulator rows. The response SHALL return all ranked entries for the requested scope and SHALL keep `cursor` as an opaque response field for future use.

#### Scenario: Unfiltered cumulative query returns current hotspot rankings

- **GIVEN** the server has ingested subject-bearing events for `repository_id = "repo-a"`
- **WHEN** `GET /v1/repositories/repo-a/hotspots?window=cumulative` is called
- **THEN** the response status is `200 OK`
- **AND** the response body is a JSON `LiveHotspotsResponse`
- **AND** `LiveHotspotsResponse.schemaVersion` is the live hotspot schema version
- **AND** `LiveHotspotsResponse.repositoryId` is `"repo-a"`
- **AND** `LiveHotspotsResponse.generatedAt` is the server time when the response was computed
- **AND** `LiveHotspotsResponse.entries` contains ranked `HotspotEntry` items with evidence row IDs from server state

#### Scenario: Unsupported window is rejected clearly

- **GIVEN** the live hotspot API supports only the cumulative window in this foundation
- **WHEN** `GET /v1/repositories/repo-a/hotspots?window=recent` is called
- **THEN** the response status is `400 Bad Request`
- **AND** the response body describes that only `window=cumulative` is supported
- **AND** the server does not guess or default the request to another window

#### Scenario: Session-scoped query recomputes rankings from matching events only

- **GIVEN** the server has accepted events for repository `repo-a` across sessions `s1` and `s2`
- **WHEN** `GET /v1/repositories/repo-a/hotspots?window=cumulative&session_id=s1` is called
- **THEN** the response status is `200 OK`
- **AND** the ranked entries reflect only events from session `s1`
- **AND** the ranking uses the existing deterministic hotspot scoring semantics
- **AND** the response is not produced by filtering repository-level accumulator totals by session membership

#### Scenario: Unfiltered live query uses the same deterministic scoring semantics as local hotspots

- **GIVEN** the same accepted server event set that would produce a local hotspot report for `repo-a`
- **WHEN** `GET /v1/repositories/repo-a/hotspots?window=cumulative` is compared to that deterministic scoring result
- **THEN** scores, rankings, counts, session counts, first seen, last seen, and evidence ordering match for the overlapping cumulative semantics
- **AND** only the live response envelope differs from the local artifact envelope

#### Scenario: Unknown repository returns an empty live envelope

- **GIVEN** no events have been ingested for `repository_id = "unknown-repo"`
- **WHEN** `GET /v1/repositories/unknown-repo/hotspots?window=cumulative` is called
- **THEN** the response status is `200 OK`
- **AND** `LiveHotspotsResponse.entries` is an empty array
- **AND** all other envelope fields are present

#### Scenario: Live query response does not contain local filesystem fields

- **GIVEN** the local `HotspotsReport` schema includes `repositoryPath` and `storePath`
- **WHEN** `LiveHotspotsResponse` is serialized for the live query API
- **THEN** the response does NOT contain `repositoryPath`
- **AND** the response does NOT contain `storePath`

### Requirement: SSE signal stream delivers hotspot delta events

The system SHALL define a read-only `GET /v1/repositories/{repository_id}/signals` endpoint for one-way hotspot signal streaming. The transport SHALL be Server-Sent Events (SSE) with media type `text/event-stream`. Each SSE message SHALL use the persisted `hotspot_signals.id` as the SSE `id` and SHALL serialize a `HotspotSignal` JSON payload in the `data` field. The endpoint SHALL support an optional `after` query parameter interpreted as a signal-row cursor. On connection, the server SHALL replay persisted signals for the repository with `id > after` ordered by `id ASC`, then tail newly committed repository signals in the same deterministic order without requiring artifact polling. `after=0` SHALL replay all persisted signals for the repository.

#### Scenario: SSE endpoint returns text/event-stream with HotspotSignal payloads

- **GIVEN** a client connects to `GET /v1/repositories/repo-a/signals`
- **WHEN** the server responds
- **THEN** the `Content-Type` header is `text/event-stream`
- **AND** the connection is a long-lived HTTP response
- **AND** each delivered SSE message includes an `id:` field carrying the signal row id
- **AND** each delivered SSE message includes a `data:` field carrying serialized `HotspotSignal` JSON

#### Scenario: Cursor replay returns only signals after the supplied id

- **GIVEN** repository `repo-a` has persisted signals with ids `5`, `6`, and `7`
- **WHEN** a client connects to `GET /v1/repositories/repo-a/signals?after=5`
- **THEN** the replay stream starts with signals `6` and `7`
- **AND** signal `5` is not replayed again
- **AND** the replay order is `id ASC`

#### Scenario: after=0 replays the full persisted signal history for a repository

- **GIVEN** repository `repo-a` has one or more persisted hotspot signals
- **WHEN** a client connects to `GET /v1/repositories/repo-a/signals?after=0`
- **THEN** the server replays all persisted signals for `repo-a`
- **AND** the replay order is `hotspot_signals.id ASC`

#### Scenario: Newly committed repository signals are streamed in deterministic order

- **GIVEN** a client is already connected to `GET /v1/repositories/repo-a/signals`
- **WHEN** new `HotspotSignal` records are committed for `repo-a`
- **THEN** the client receives those new signals without polling artifact files
- **AND** delivery order follows persisted `hotspot_signals.id ASC`

### Requirement: Scope is limited to contract definition and types

This change SHALL implement only the read-only live hotspot query and signal streaming behavior on top of the existing ingest and accumulator foundations. It SHALL preserve `.scryrs/hotspots.json` as an explicit export/cache path and SHALL NOT use that artifact as the live query source of truth. It SHALL NOT add websocket transport, graph, proposal, route, or runtime retrieval APIs, dashboard mutation behavior, or local/remote merge behavior.

#### Scenario: Artifact export remains available but separate from live query state

- **GIVEN** a user explicitly runs or requests hotspot export
- **WHEN** `.scryrs/hotspots.json` is produced from local or server-owned state
- **THEN** the artifact remains available as a portable report file
- **AND** `GET /v1/repositories/{repository_id}/hotspots` does not read that artifact as its source of truth

#### Scenario: Existing ingest behavior remains the only write surface

- **WHEN** this change is implemented
- **THEN** `POST /v1/trace-events/batch` remains the only server write API in scope
- **AND** any internal signal notification added for SSE does not change the public ingest contract

#### Scenario: No non-hotspot retrieval or mutation APIs are added

- **WHEN** this change is implemented
- **THEN** no graph, proposal, route, or runtime retrieval API is added
- **AND** no websocket transport is added
- **AND** no dashboard mutation behavior is added