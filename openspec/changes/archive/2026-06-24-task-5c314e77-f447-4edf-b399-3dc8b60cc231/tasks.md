## 1. Define ServerIngestEnvelope and related types in scryrs-types

- [x] 1.1 Add `ServerIngestEnvelope` struct with `envelope_version`, `repository_id`, `workspace_id`, `agent_id`, and `events: Vec<EnvelopeEvent>`
- [x] 1.2 Add `EnvelopeEvent` struct with `producer_event_id`, `client_timestamp`, and `event: TraceEvent`
- [x] 1.3 Add `BatchIngestResponse` struct with `received_count`, `duplicate_count`, `events: Vec<EventAck>`, `received_at`
- [x] 1.4 Add `EventAck` struct with `producer_event_id`, `status` ("accepted" | "idempotent"), `server_event_id` (optional), `received_at`
- [x] 1.5 Add `LiveHotspotsResponse` struct with `schemaVersion`, `repository_id`, `cursor`, `generatedAt`, `entries: Vec<HotspotEntry>` (reusing `HotspotEntry` from existing types)
- [x] 1.6 Add serde `Serialize`/`Deserialize` derives for all new types with JSON round-trip tests
- [x] 1.7 Add deduplication-key contract test validating that `(repository_id, workspace_id, agent_id, producer_event_id)` forms the 4-tuple uniqueness key

## 2. Write live-hotspot-server-contract OpenSpec capability spec

- [x] 2.1 Create `openspec/changes/task-5c314e77-f447-4edf-b399-3dc8b60cc231/specs/live-hotspot-server-contract/spec.md`
- [x] 2.2 Specify ServerIngestEnvelope schema requirements with all field semantics
- [x] 2.3 Specify deduplication contract: 4-tuple composite key, first-writer-wins, idempotent ack status, non-double-counting
- [x] 2.4 Specify clock semantics: `client_timestamp` vs `server received_at` separation
- [x] 2.5 Specify `POST /v1/trace-events/batch` endpoint shape (method, path, request/response bodies, status codes)
- [x] 2.6 Specify `GET /v1/repositories/{repository_id}/hotspots` endpoint shape (method, path, response envelope)
- [x] 2.7 Specify `GET /v1/repositories/{repository_id}/signals` SSE endpoint skeleton (media type `text/event-stream`, event framing)
- [x] 2.8 Specify local-only vs remote-live mode separation rules
- [x] 2.9 Specify repository_id derivation documentation (Git remote origin URL normalization, fallback rules)
- [x] 2.10 Define `LIVE_HOTSPOT_SCHEMA_VERSION` constant at `"1.0.0"` for the live query response, independent of existing `SCHEMA_VERSION` and `HOTSPOT_SCHEMA_VERSION`

## 3. Update trace-hook-contract documentation

- [x] 3.1 Add Remote Mode appendix to `.devagent/docs/docs/trace-hook-contract.md`
- [x] 3.2 Document ServerIngestEnvelope shape for harness integrators
- [x] 3.3 Document identity field semantics: repository_id, workspace_id, agent_id, producer_event_id
- [x] 3.4 Document remote ingestion mode as exclusive (skips local storage) with explicit configuration activation
- [x] 3.5 Document that inner TraceEvent and local `scryrs record --stdin` contract are unchanged

## 4. Verification

- [x] 4.1 Run `cargo test -p scryrs-types` to verify all new serde round-trip tests pass
- [x] 4.2 Run OpenSpec validation on the new spec
- [x] 4.3 Confirm no changes to existing specs (`trace-event-schema`, `trace-hook-contract`, `scryrs-record-endpoint`, `hotspot-report`)