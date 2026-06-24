## ADDED Requirements

### Requirement: `scryrs server` starts the central ingest runtime

The system SHALL expose `scryrs server` as the runtime surface for central trace ingest. The command SHALL start an HTTP server that serves `POST /v1/trace-events/batch`, SHALL default to bind `127.0.0.1`, port `8081`, and store path `.scryrs/server.db`, and SHALL allow explicit overrides for bind address, port, and store path.

#### Scenario: Default invocation starts the ingest server

- **WHEN** a user runs `scryrs server`
- **THEN** the server listens on `127.0.0.1:8081`
- **AND** it uses `.scryrs/server.db` as the server-owned SQLite store
- **AND** it exposes `POST /v1/trace-events/batch`

#### Scenario: Explicit flags override defaults

- **WHEN** a user runs `scryrs server --bind 0.0.0.0 --port 9091 --store /tmp/live.db`
- **THEN** the server listens on `0.0.0.0:9091`
- **AND** it uses `/tmp/live.db` as the server-owned SQLite store

#### Scenario: Discovery surfaces list the server command

- **WHEN** a user runs `scryrs --help` or `scryrs --help-json`
- **THEN** the `server` command is listed
- **AND** the command documents `bind`, `port`, and `store` configuration

### Requirement: The server owns a dedicated SQLite store with idempotent inserts

The server SHALL persist accepted events into a dedicated SQLite store/table owned by the server. The server store SHALL mirror the normalized trace-event columns needed by existing storage semantics, SHALL add `repository_id`, `workspace_id`, `agent_id`, `producer_event_id`, `client_timestamp`, and row-level `received_at`, and SHALL enforce first-writer-wins idempotency with a unique composite key on `(repository_id, workspace_id, agent_id, producer_event_id)`. This change SHALL NOT modify the existing local `trace_events` schema.

#### Scenario: First submission creates one stored row

- **GIVEN** no prior stored row exists for `(repo-a, ws-1, pi, evt-001)`
- **WHEN** the server accepts an event with that composite key
- **THEN** the server store contains one row for that key
- **AND** the row persists the accepted event, normalized columns, `client_timestamp`, and `received_at`

#### Scenario: Duplicate submission is idempotent

- **GIVEN** a stored row already exists for `(repo-a, ws-1, pi, evt-001)`
- **WHEN** the same composite key is submitted again
- **THEN** no second row is created
- **AND** the per-item result reports the event as idempotent
- **AND** the per-item result returns the original stored `received_at`

#### Scenario: Local record schema remains unchanged

- **WHEN** this change is implemented
- **THEN** `scryrs record --stdin` and `scryrs record --file` continue writing to the existing local `.scryrs/scryrs.db`
- **AND** the local `trace_events` table is not extended with server identity columns

### Requirement: Batch validation separates envelope failures from per-item rejections

The server SHALL validate ingest requests in two layers. Malformed top-level request bodies, unsupported `envelope_version`, or missing top-level identity fields SHALL fail with deterministic `400 Bad Request` diagnostics. For an otherwise valid envelope, the server SHALL evaluate each `events` entry independently, SHALL validate `client_timestamp` as RFC 3339, SHALL reuse `TraceEvent` validation for the inner event, and SHALL allow valid siblings to succeed alongside rejected siblings.

#### Scenario: Malformed top-level request fails with 400

- **WHEN** `POST /v1/trace-events/batch` receives malformed JSON, an unsupported `envelope_version`, or missing `repository_id`, `workspace_id`, or `agent_id`
- **THEN** the response status is `400 Bad Request`
- **AND** the response body contains deterministic diagnostics for the top-level failure

#### Scenario: Mixed batch accepts valid siblings and rejects invalid items

- **GIVEN** a structurally valid envelope containing one valid event, one schema-invalid `TraceEvent`, and one item missing per-event identity
- **WHEN** `POST /v1/trace-events/batch` is called
- **THEN** the response status is `200 OK`
- **AND** `accepted_count` is `1`
- **AND** `rejected_count` is `2`
- **AND** the response includes one per-item result for each request item in request order
- **AND** rejected items include deterministic indexed diagnostics

#### Scenario: Invalid client timestamp is rejected per item

- **GIVEN** a structurally valid envelope containing an event whose `client_timestamp` is not valid RFC 3339
- **WHEN** `POST /v1/trace-events/batch` is called
- **THEN** the event is rejected without aborting valid siblings
- **AND** the rejection diagnostic identifies the failing item deterministically

### Requirement: One server process owns writes to the central store

Clients SHALL submit central-ingest events through HTTP to `scryrs server`; they SHALL NOT receive or use a direct-write contract to the server-owned SQLite file.

#### Scenario: Concurrent duplicate submissions yield one stored row

- **GIVEN** two clients concurrently submit the same `(repository_id, workspace_id, agent_id, producer_event_id)` key to `POST /v1/trace-events/batch`
- **WHEN** both requests complete
- **THEN** exactly one per-item result reports `accepted`
- **AND** exactly one per-item result reports `idempotent`
- **AND** the server store contains one row for that key
- **AND** neither client writes the SQLite file directly
