## Why

Live Hotspot Foundation 01 — Define server ingest and identity contract

Feature: live hotspot server contract
As multi-agent scryrs operator
I want a versioned server ingest and identity contract
So that multiple agents can report trace evidence to one shared hotspot authority without relying on local artifact files

### Scenario: Event ingest identifies repository and producer

Given an agent records a TraceEvent from a containerized workspace
When it submits the event to the server ingest API
Then the request includes stable repository identity, workspace identity, agent identity, and producer event id
And the server can group evidence without trusting container-local absolute paths

### Scenario: Duplicate submissions are harmless

Given a CLI retry sends the same producer event more than once
When the server processes the batch
Then the event is stored once
And hotspot scores do not double count the duplicate

### Scenario: Local artifact semantics stay separate

Given server ingest is configured
When events are accepted by the server
Then server state is the source of truth for live hotspot queries
And `.scryrs/hotspots.json` remains an export/cache artifact, not the live state store

## What Changes

This change defines the versioned remote ingest contract and live-state semantics for scryrs Phase 4 (Live Hotspot Server and Signals). It does not implement the server, does not modify dashboard or streaming behavior, and preserves the existing local-only pipeline unchanged. All changes are contract artifacts, Rust type additions in `scryrs-types`, and spec/doc/test updates only.

### New Rust types in `crates/scryrs-types`

- **`ServerIngestEnvelope`** — a versioned JSON batch wrapper carrying `envelope_version`, `repository_id`, `workspace_id`, `agent_id`, and an `events` array of `EnvelopeEvent` items.
- **`EnvelopeEvent`** — pairs a `producer_event_id`, `client_timestamp`, and inner `event: TraceEvent` for each submitted event.
- **`BatchIngestResponse`** — JSON acknowledgment returned by `POST /v1/trace-events/batch`, with `received_count`, `duplicate_count`, per-event `EventAck` entries, and server `received_at` timestamp.
- **`LiveHotspotsResponse`** — a new live-query envelope (separate from `HotspotsReport`) for `GET /v1/repositories/{repository_id}/hotspots`, carrying `schemaVersion`, `repository_id`, `cursor`, `generatedAt`, and `entries` (reusing `HotspotEntry` shape).

### New OpenSpec spec

- **`live-hotspot-server-contract`** — full capability specification covering the `ServerIngestEnvelope` schema, HTTP endpoint shapes (`POST /v1/trace-events/batch`, `GET /v1/repositories/{repository_id}/hotspots`, `GET /v1/repositories/{repository_id}/signals`), deduplication contract, clock-semantic rules, workspace-id documentation, and source-of-truth rules for local-only vs remote-live modes.

### Updated documentation

- **`.devagent/docs/docs/trace-hook-contract.md`** — add a Remote Mode appendix documenting the `ServerIngestEnvelope` shape, identity field semantics, and producer expectations for remote transport (no changes to local `scryrs record --stdin` contract).

### Test additions

- **`crates/scryrs-types/tests/`** — serialization round-trip tests for `ServerIngestEnvelope`, `EnvelopeEvent`, `BatchIngestResponse`, and `LiveHotspotsResponse`.
- **`crates/scryrs-types/tests/`** — deduplication-key coverage tests validating the 4-tuple uniqueness contract.

## Impact

### Affected crates

- `crates/scryrs-types` — adds new structs (`ServerIngestEnvelope`, `EnvelopeEvent`, `BatchIngestResponse`, `LiveHotspotsResponse`, `EventAck`) and serde round-trip tests. Inner `TraceEvent` and existing `HotspotsReport` are unchanged.
- No changes to `scryrs-cli`, `scryrs-core`, `scryrs-dashboard`, or hook source files.

### Backward compatibility

- Full backward compatibility with the existing local-only pipeline. All nine existing hook implementations continue to produce bare `TraceEvent` records and invoke `scryrs record --stdin` without change.
- `HotspotsReport` and `.scryrs/hotspots.json` artifact semantics are preserved for local-only mode.
- The remote contract is independent of the local contract; no existing spec is modified.

### Migration path

- Future implementation tasks will add remote transport to `scryrs record` (via explicit configuration only), build the server ingest handler against this contract, and expand the trace-hook-contract doc with remote-mode producer guidance.
- No migration is required from existing users; local-only remains the default.