## 1. Server store hotspot query materialization

- [x] 1.1 Add production store methods that materialize cumulative unfiltered `LiveHotspotsResponse` data from `hotspot_accumulators`, preserving deterministic ranking, counts, session count, first/last seen, and evidence ordering.
- [x] 1.2 Add a session-scoped query path that filters `server_trace_events` by `repository_id` and `session_id` and recomputes rankings through `scryrs_core::scoring::score_hotspots` instead of filtering accumulator rows.
- [x] 1.3 Add a production signal replay query that returns persisted repository `HotspotSignal` rows with `id > after` ordered by `hotspot_signals.id ASC`.

## 2. Server routes and streaming infrastructure

- [x] 2.1 Add `GET /v1/repositories/{repository_id}/hotspots` with `window` and optional `session_id` query parsing, `window=cumulative` validation, and `LiveHotspotsResponse` serialization for known and unknown repositories.
- [x] 2.2 Add `GET /v1/repositories/{repository_id}/signals` as `text/event-stream`, replay persisted signals for `after=<signal_id>`, then tail newly committed repository signals in id order.
- [x] 2.3 Add a `tokio::sync::broadcast` notification channel in server app state outside the store mutex, plus the Tokio/Tokio Stream support needed for SSE fanout, and publish newly committed signals after ingest commits.
- [x] 2.4 Keep `POST /v1/trace-events/batch` externally unchanged aside from internal signal notification wiring.

## 3. Discovery and export boundaries

- [x] 3.1 Update CLI server help and README/server discovery text to mention the new read-only hotspot and signal endpoints.
- [x] 3.2 Preserve the existing local `scryrs hotspots` artifact export path and ensure live handlers do not read `.scryrs/hotspots.json` as live state.
- [x] 3.3 Verify no graph, proposal, route, runtime retrieval, websocket, dashboard mutation, or local/remote merge behavior is added.

## 4. Verification

- [x] 4.1 Add route and store tests for cumulative hotspot queries, unknown repositories, unsupported window rejection, and session-scoped query correctness.
- [x] 4.2 Add signal stream tests covering `after` replay semantics, `after=0` full replay, deterministic `hotspot_signals.id` ordering, and tail delivery of newly committed signals.
- [x] 4.3 Add regression coverage proving live query rankings match existing deterministic scoring semantics and that artifact export remains a separate path.
