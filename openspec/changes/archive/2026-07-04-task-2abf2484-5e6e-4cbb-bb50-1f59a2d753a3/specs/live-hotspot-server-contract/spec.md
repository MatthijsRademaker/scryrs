## MODIFIED Requirements

### Requirement: Batch ingest endpoint accepts and acknowledges events

The system SHALL define a `POST /v1/trace-events/batch` endpoint for batch event ingestion. The endpoint SHALL accept a JSON `ServerIngestEnvelope` body and return a JSON `BatchIngestResponse` with deterministic batch counts plus per-item results in request order. `BatchIngestResponse` SHALL include additive `accepted_count` and `rejected_count` fields alongside the existing `received_count` and `duplicate_count`. Each per-item result SHALL include the request item's `index`, `status`, and either `received_at` or `error_reason`; `producer_event_id` SHALL be returned when available and MAY be omitted only for rejected items whose request element did not provide one.

#### Scenario: Successful batch ingest returns deterministic counts

- **GIVEN** a valid `ServerIngestEnvelope` with three valid events and no duplicates
- **WHEN** `POST /v1/trace-events/batch` is called
- **THEN** the response status is `200 OK`
- **AND** the response body is a JSON `BatchIngestResponse`
- **AND** `accepted_count` equals `3`
- **AND** `duplicate_count` equals `0`
- **AND** `rejected_count` equals `0`
- **AND** `received_count` equals `3`
- **AND** the response `events` array contains three per-item results in request order
- **AND** `BatchIngestResponse.received_at` is the server receipt timestamp

#### Scenario: Invalid inner event within envelope is rejected per item

- **GIVEN** a `ServerIngestEnvelope` containing one valid `TraceEvent` and one `TraceEvent` failing validation
- **WHEN** `POST /v1/trace-events/batch` is called
- **THEN** the valid event is accepted with status `accepted`
- **AND** the invalid event is rejected with an `error_reason`
- **AND** `accepted_count` reflects only accepted items
- **AND** `rejected_count` reflects the rejected item
- **AND** the rejected item's per-item result includes its request `index`

#### Scenario: Malformed request item without producer_event_id is still rejected deterministically

- **GIVEN** a structurally valid envelope containing an `events` item that cannot supply `producer_event_id`
- **WHEN** `POST /v1/trace-events/batch` is called
- **THEN** the response remains `200 OK`
- **AND** the item is rejected rather than aborting valid siblings
- **AND** the per-item result identifies the item by `index`
- **AND** the per-item result omits `producer_event_id`

### Requirement: Scope is limited to the server ingest foundation

This change SHALL implement only the central ingest foundation: the ingest server runtime, `POST /v1/trace-events/batch`, additive shared types required for deterministic counts and diagnostics, server-owned SQLite persistence, CLI discovery updates for `scryrs server`, and tests. It SHALL NOT implement live hotspot query endpoints, SSE signal streaming, authentication, hosted deployment, or automatic remote hook or `scryrs record` transport changes.

#### Scenario: No live query endpoint is implemented in this change

- **WHEN** this change is implemented
- **THEN** `GET /v1/repositories/{repository_id}/hotspots` is not part of the delivered scope
- **AND** live hotspot querying remains a follow-up task

#### Scenario: No signal streaming endpoint is implemented in this change

- **WHEN** this change is implemented
- **THEN** `GET /v1/repositories/{repository_id}/signals` is not part of the delivered scope
- **AND** SSE signal delivery remains a follow-up task

#### Scenario: Local-only ingest behavior remains the default

- **WHEN** this change is implemented
- **THEN** `scryrs record` continues to persist to local `.scryrs/scryrs.db`
- **AND** current hook behavior is unchanged
- **AND** central ingest is available only through the new server runtime
