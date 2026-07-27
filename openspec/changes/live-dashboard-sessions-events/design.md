## Context

Live mode currently proxies only cumulative hotspots and the signal SSE stream. The server already persists raw trace events in `server_trace_events`, while the local dashboard derives session summaries and event pages from `.scryrs/scryrs.db`. The live dashboard must expose equivalent read models without reading local artifacts or changing the ingest contract.

## Goals / Non-Goals

**Goals:**

- Add repository-scoped, read-only server APIs for sessions and events.
- Preserve existing dashboard DTOs and same-origin browser API paths.
- Support bounded pagination and stable cursors.
- Keep local mode behavior unchanged.

**Non-Goals:**

- Route manifests, proposals, or review actions.
- Cross-repository queries.
- Arbitrary SQL or event mutation APIs.
- Long-term archival or retention policy changes.

## Decisions

1. **Query server-owned event rows directly.** Add store methods over `server_trace_events` rather than reconstructing sessions from hotspot accumulators. This preserves event payloads and exact event ordering. A separate analytics database was rejected because the required source already exists.

2. **Use repository-scoped REST endpoints.** Add `GET /v1/repositories/{repository_id}/sessions`, `GET /v1/repositories/{repository_id}/sessions/{session_id}`, and `GET /v1/repositories/{repository_id}/events`. Repository path scoping matches existing hotspots/signals APIs and prevents accidental cross-repository reads.

3. **Use numeric event IDs as cursors.** Events are ordered by server row ID, with `after`/`before` semantics defined in the contract. Offset pagination was rejected because inserts shift pages during active ingestion.

4. **Keep dashboard as same-origin proxy.** Browser calls remain `/api/sessions` and `/api/events`; the dashboard backend adds upstream URL construction, status mapping, and response validation. Direct browser access to the server API was rejected to avoid CORS and duplicated server URL configuration.

5. **Normalize server DTOs at the dashboard boundary.** Server responses use explicit JSON DTOs compatible with current TypeScript interfaces. Payload JSON is returned as parsed JSON, with malformed stored payloads represented as `null` and surfaced through diagnostics rather than crashing the whole page.

## Risks / Trade-offs

- **[Large event tables]** Unbounded queries could overload the server → enforce limits, indexes on `(repository_id, id)` and `(repository_id, session_id, id)`, and cursor pagination.
- **[Ingest/read contention]** SQLite reads may contend with writes → use the existing connection strategy and read-only query methods; verify under concurrent ingest tests.
- **[Schema drift]** Dashboard and server DTOs can diverge → add contract fixtures and integration tests for every endpoint.
- **[Historical payload corruption]** Existing rows may contain invalid JSON → return null payloads with explicit per-row diagnostic metadata or documented null semantics.

## Migration Plan

1. Add additive server read endpoints and indexes.
2. Add dashboard proxy endpoints and live frontend data paths behind mode checks.
3. Deploy server and dashboard together; old dashboards continue using hotspots/signals only.
4. Enable live navigation after endpoint smoke tests pass.
5. Roll back by deploying the prior dashboard image; additive server tables/indexes remain harmless.

## Open Questions

- Should session list query parameter be `limit` only, or include `after` for incremental polling?
- Should live Events reuse the current cursor field name (`cursor`) or expose an explicit `after` cursor?
- What retention limit is required before server-side pagination is insufficient?
