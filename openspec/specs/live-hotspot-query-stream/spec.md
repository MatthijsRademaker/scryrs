# live-hotspot-query-stream Specification

## Purpose
TBD - created by archiving change task-73818b5f-8a12-4cad-9d36-a70e256f1c45. Update Purpose after archive.
## Requirements
### Requirement: Live hotspot query returns accumulator-backed ranked entries

The system SHALL expose `GET /v1/repositories/{repository_id}/hotspots` on `scryrs server`. The endpoint SHALL return a `LiveHotspotsResponse` JSON envelope with ranked `HotspotEntry` values materialized directly from `hotspot_accumulators` rows. The response SHALL NOT contain filesystem-path fields (`repositoryPath`, `storePath`).

#### Scenario: Valid query returns ranked hotspot entries

- **GIVEN** the server has ingested subject-bearing events for `repository_id = "repo-a"`
- **WHEN** `GET /v1/repositories/repo-a/hotspots` is called
- **THEN** the response status is `200 OK`
- **AND** the response body is a JSON `LiveHotspotsResponse`
- **AND** `LiveHotspotsResponse.schemaVersion` is `LIVE_HOTSPOT_SCHEMA_VERSION`
- **AND** `LiveHotspotsResponse.repositoryId` is `"repo-a"`
- **AND** `LiveHotspotsResponse.entries` contains `HotspotEntry` items ranked by six-key tie-break: score DESC, sessionCount DESC, lastSeen DESC, subjectKind ASC, subject ASC, firstEvidenceId ASC
- **AND** `LiveHotspotsResponse.generatedAt` is the server timestamp when the response was computed
- **AND** `LiveHotspotsResponse.cursor` equals `generatedAt` (point-in-time snapshot marker, not a pagination cursor)

#### Scenario: Unknown repository returns empty entries

- **GIVEN** no events have been ingested for `repository_id = "unknown-repo"`
- **WHEN** `GET /v1/repositories/unknown-repo/hotspots` is called
- **THEN** the response status is `200 OK`
- **AND** `LiveHotspotsResponse.entries` is an empty array
- **AND** all other envelope fields are present

#### Scenario: Query response has no filesystem-path fields

- **GIVEN** the local `HotspotsReport` schema includes `repositoryPath` and `storePath`
- **WHEN** `LiveHotspotsResponse` is serialized for a hotspot query
- **THEN** the response does NOT contain `repositoryPath`
- **AND** the response does NOT contain `storePath`

#### Scenario: Window=cumulative is supported

- **GIVEN** the server has accumulators for `window = "cumulative"`
- **WHEN** `GET /v1/repositories/repo-a/hotspots?window=cumulative` is called
- **THEN** the response status is `200 OK`
- **AND** entries are returned from cumulative accumulator rows

### Requirement: Unsupported window values reject with explicit 400

The hotspot query endpoint SHALL validate the `window` query parameter. Only `window=cumulative` is supported. Any other window value SHALL return `400 Bad Request` with a descriptive error message listing supported windows. Omitting `window` SHALL default to `cumulative`.

#### Scenario: Unsupported window returns 400

- **GIVEN** the only implemented window model is `cumulative`
- **WHEN** `GET /v1/repositories/repo-a/hotspots?window=recent` is called
- **THEN** the response status is `400 Bad Request`
- **AND** the response body contains `error` describing that the window is unsupported
- **AND** the error message lists supported windows

#### Scenario: Missing window defaults to cumulative

- **GIVEN** the query does not include a `window` parameter
- **WHEN** `GET /v1/repositories/repo-a/hotspots` is called
- **THEN** the handler treats `window` as `"cumulative"`
- **AND** the response status is `200 OK`

### Requirement: Session-scoped hotspot filtering is deferred with explicit 400

The hotspot query endpoint SHALL reject the `session_id` query parameter with `400 Bad Request` and a clear deferral message. The current `hotspot_accumulators` schema stores only a distinct session set, not per-session scores. Returning per-session rankings would be incorrect.

#### Scenario: Session_id parameter returns 400 deferral

- **GIVEN** the hotspot query endpoint receives a `session_id` query parameter
- **WHEN** `GET /v1/repositories/repo-a/hotspots?session_id=s1` is called
- **THEN** the response status is `400 Bad Request`
- **AND** the response body contains `error` set to `"session-scoped hotspot queries are not yet supported; omit session_id or use the cumulative window without session filter."`

#### Scenario: Session_id combined with valid window still returns 400

- **GIVEN** the query includes both `window=cumulative` and `session_id=s1`
- **WHEN** `GET /v1/repositories/repo-a/hotspots?window=cumulative&session_id=s1` is called
- **THEN** the response status is `400 Bad Request`
- **AND** the deferral error is returned

### Requirement: Hotspot signal stream emits events in durable deterministic order

The system SHALL expose `GET /v1/repositories/{repository_id}/signals` as a Server-Sent Events (SSE) endpoint with `Content-Type: text/event-stream`. The endpoint SHALL emit persisted `HotspotSignal` records ordered by `hotspot_signals.id ASC` (the autoincrement primary key), not by the second-precision `created_at` timestamp. Each SSE event SHALL include `id:` set to the signal's row id and `data:` containing a JSON `HotspotSignalEvent`.

#### Scenario: SSE endpoint returns text/event-stream

- **GIVEN** a client connects to the SSE endpoint
- **WHEN** `GET /v1/repositories/repo-a/signals` is called
- **THEN** the response `Content-Type` header is `text/event-stream`
- **AND** the connection is a long-lived HTTP response

#### Scenario: Existing signals are emitted in id order on connect

- **GIVEN** the `hotspot_signals` table has signals with ids `[1, 3, 5]` for `repository_id = "repo-a"`
- **WHEN** a client connects to `GET /v1/repositories/repo-a/signals`
- **THEN** signals are emitted in order: id 1 first, then id 3, then id 5
- **AND** each SSE event carries `id: <signal.id>` and `data: <HotspotSignalEvent JSON>`

#### Scenario: New signals are emitted in insertion order

- **GIVEN** a connected SSE client has received all existing signals
- **WHEN** a new ingest batch produces a signal with a higher id for repo-a
- **THEN** the SSE stream emits the new signal next
- **AND** ordering is by `hotspot_signals.id ASC`, not by `created_at`

#### Scenario: Signal ordering is deterministic under same-second writes

- **GIVEN** two ingest batches arrive within the same wall-clock second producing signals for repo-a
- **WHEN** those signals are persisted
- **THEN** they receive distinct autoincrement `id` values
- **AND** the SSE stream emits them in `id ASC` order
- **AND** the order is deterministic regardless of `created_at` collisions

### Requirement: SSE replay via after parameter and Last-Event-ID

The signal stream endpoint SHALL support the `after` query parameter and `Last-Event-ID` replay. When `after=<signal_id>` is provided, the stream SHALL emit only signals with `id > <signal_id>`. The SSE `id:` field SHALL be set to the signal's row id, enabling standard `EventSource` reconnect.

#### Scenario: After parameter replays from specified position

- **GIVEN** signals with ids `[10, 11, 12, 13]` exist for repo-a
- **WHEN** `GET /v1/repositories/repo-a/signals?after=11` is called
- **THEN** the stream emits signals with ids 12 and 13
- **AND** `WHERE id > ?` semantics tolerate gaps in autoincrement ids

#### Scenario: After parameter with no newer signals waits for future data

- **GIVEN** the latest signal for repo-a has id 20
- **WHEN** `GET /v1/repositories/repo-a/signals?after=20` is called
- **THEN** no signals are emitted immediately
- **AND** the connection remains open waiting for signals with id > 20

#### Scenario: Missing after parameter emits all signals from start

- **GIVEN** signals with ids `[1, 2, 3]` exist for repo-a
- **WHEN** `GET /v1/repositories/repo-a/signals` is called
- **THEN** all three signals are emitted in id order

### Requirement: SSE connection is isolated from ingest mutex

The SSE stream handler SHALL open a separate read-only `rusqlite::Connection` to the store file path per connected client. It SHALL NOT share or hold the `Arc<Mutex<ServerStore>>` across async await points for the stream lifetime. The read-only connection SHALL leverage SQLite WAL mode for concurrent reads.

#### Scenario: SSE stream does not block ingest operations

- **GIVEN** an active SSE stream is polling for new signals
- **WHEN** a new `POST /v1/trace-events/batch` request arrives
- **THEN** the ingest request completes without blocking
- **AND** the SSE stream observes the new signal on its next poll cycle

#### Scenario: Store path is available for read-only connection

- **GIVEN** the server is running with store at a known path
- **WHEN** an SSE handler needs to open a read-only connection
- **THEN** the store file path is accessible from server configuration
- **AND** the connection is opened with read-only flags

### Requirement: SSE heartbeat prevents proxy timeout

The SSE stream handler SHALL emit keepalive comment lines at regular intervals when no signal data is available, preventing intermediate proxies from closing idle connections.

#### Scenario: Heartbeat sent during idle periods

- **GIVEN** an SSE stream has emitted all existing signals and no new signals arrive
- **WHEN** a configurable keepalive interval elapses with no signal data
- **THEN** a keepalive comment is sent
- **AND** the connection remains open

#### Scenario: Heartbeat does not interrupt signal emission

- **GIVEN** signals are arriving frequently
- **WHEN** the stream is actively emitting signal events
- **THEN** extra keepalive comments are not interleaved between closely-spaced signal events

### Requirement: HotspotSignalEvent payload includes server-side signal id

The SSE `data:` payload SHALL use a `HotspotSignalEvent` type defined in `scryrs-types` that wraps all existing `HotspotSignal` fields plus a server-side `id: i64` field. The `id` field SHALL correspond to `hotspot_signals.id` and match the SSE `id:` field value.

#### Scenario: HotspotSignalEvent carries id and all HotspotSignal fields

- **GIVEN** a `HotspotSignalEvent` is serialized for an SSE data payload
- **WHEN** the JSON is inspected
- **THEN** it includes `id` matching the signal's autoincrement row id
- **AND** it includes `repositoryId`, `subjectKind`, `subject`, `score`, `delta`, `window`, `threshold`, `evidenceRowIds`, `createdAt`

#### Scenario: HotspotSignalEvent round-trips through JSON

- **GIVEN** a `HotspotSignalEvent` with all fields populated
- **WHEN** it is serialized to JSON and deserialized back
- **THEN** the reconstructed struct equals the original
- **AND** the `id` field is preserved

### Requirement: New types and routes are additive

All new types and routes SHALL be additive. No existing types, routes, or specs SHALL be modified except for the deferred signal payload placeholder in `live-hotspot-server-contract` being replaced with a concrete reference to `HotspotSignalEvent`.

#### Scenario: Existing ingest endpoint is unaffected

- **GIVEN** the query and signal stream routes are registered
- **WHEN** `POST /v1/trace-events/batch` is called
- **THEN** the ingest behavior is identical to before the new routes were added
- **AND** existing ingest tests continue to pass

#### Scenario: Existing types are unchanged

- **WHEN** new types are added to `scryrs-types`
- **THEN** `HotspotSignal`, `LiveHotspotsResponse`, `HotspotEntry`, `ServerIngestEnvelope`, and all other existing structs retain their current fields
- **AND** existing serde round-trip tests continue to pass

#### Scenario: Artifact export path remains separate

- **GIVEN** the live query and stream APIs are active
- **WHEN** local `scryrs hotspots` is invoked
- **THEN** `.scryrs/hotspots.json` is produced from the local store as before
- **AND** the live query code path never reads `.scryrs/hotspots.json`

