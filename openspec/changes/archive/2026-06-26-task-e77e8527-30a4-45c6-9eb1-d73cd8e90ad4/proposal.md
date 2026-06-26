## Why

The central server already ingests deduplicated trace events, maintains cumulative hotspot accumulators, and persists `HotspotSignal` rows, but live consumers still have no read APIs. Dashboard and CLI clients must scrape `.scryrs/hotspots.json`, which conflicts with the remote source-of-truth contract that says live hotspot queries belong to server-owned state.

This task exposes narrow, read-only server APIs for current hotspot rankings and ordered hotspot signals while keeping artifact export available as a separate export/cache path.

## What Changes

- Add two read-only server endpoints beside the existing ingest route: `GET /v1/repositories/{repository_id}/hotspots` and `GET /v1/repositories/{repository_id}/signals`.
- Materialize unfiltered cumulative hotspot queries from `hotspot_accumulators` and compute session-scoped queries from matching `server_trace_events` through the existing deterministic `score_hotspots` path rather than filtering global aggregates.
- Support `window=cumulative` and fail clearly for unsupported window values; support `session_id` on hotspot queries where scoped; return `LiveHotspotsResponse` with ranked `HotspotEntry` data and evidence references.
- Stream persisted and newly committed `HotspotSignal` records over SSE using deterministic `hotspot_signals.id` ordering, `after=<signal_id>` replay/resume semantics, and an internal broadcast notification channel outside the store mutex.
- Keep `.scryrs/hotspots.json` as an explicit export/cache path only; live query handlers must not read artifact files as the source of truth.
- Update server discovery surfaces for the expanded read-only API while keeping scope limited to live hotspot query and signal streaming behavior.

## Impact

- Dashboard and CLI consumers can observe server-authoritative hotspot state without scraping artifact files or polling for new signals.
- Existing `POST /v1/trace-events/batch` behavior remains intact except for internal notification needed to fan out newly created signals.
- The change stays additive: no websocket transport, no graph/proposal/route/runtime retrieval APIs, no dashboard mutation behavior, and no local/remote merge behavior.