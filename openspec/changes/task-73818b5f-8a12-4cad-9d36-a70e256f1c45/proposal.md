## Why

The `scryrs server` central runtime already owns authoritative live hotspot state (accumulators, signals) and a `POST /v1/trace-events/batch` ingest endpoint, but there is no server-authoritative read API. The dashboard still reads `.scryrs/hotspots.json` from the filesystem, and signal reads rely on second-precision `created_at` ordering that is not a safe total-order key for streaming. This change adds two read-only server APIs that expose accumulator-backed hotspot rankings and an SSE signal stream, eliminating the need to scrape artifact files for live evidence observation.

## What Changes

- **New endpoint `GET /v1/repositories/{repository_id}/hotspots`** — returns `LiveHotspotsResponse` ranked from server-owned `hotspot_accumulators` using deterministic six-key tie-break matching `score_hotspots`. Accepts `window=cumulative` only; unsupported windows and session-scoped filtering are rejected with deterministic 400 errors.
- **New endpoint `GET /v1/repositories/{repository_id}/signals`** — serves `text/event-stream` SSE emitting `HotspotSignal` records ordered by `hotspot_signals.id ASC` (autoincrement PK). Supports `after=<signal_id>` replay and sets SSE `id:` field for `Last-Event-ID`. Each stream opens a separate read-only `rusqlite::Connection`.
- **New production store query methods** — `query_hotspots` materializes ranked `HotspotEntry` from accumulator rows using pre-stored score/counts/sessions/timestamps/evidence; `poll_signals` queries signals ordered by `id ASC` with cursor-based replay.
- **New SSE payload type** — `HotspotSignalEvent` wrapper in `scryrs-types` including server-side `id` alongside existing `HotspotSignal` fields.
- **New query parameter types** — `HotspotQueryParams` with validated `window` and explicit `session_id` deferral.
- **No changes to** dashboard, CLI, artifact export, graph/proposal/runtime APIs, ingest envelope, or local store.

## Impact

- **Affected crates:** `scryrs-server` (routes + store query methods + SSE dependencies), `scryrs-types` (new `HotspotSignalEvent`, `HotspotQueryParams`).
- **Affected specs:** `live-hotspot-server-contract` (updated to concrete signal payload and filter semantics), new `live-hotspot-query-stream` spec.
- **No migration risk:** New routes are additive; existing ingest and export paths are untouched.