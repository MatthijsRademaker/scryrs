## Why

Live swarm dashboards currently expose only hotspots and signals, even though the live server already persists every trace event with session identity. Operators cannot inspect session timelines or raw event evidence without switching to a local workspace dashboard, which defeats the purpose of centralized live observation.

## What Changes

- Add authenticated-by-repository live server APIs for session summaries, session detail, and paginated event queries.
- Query server-owned `server_trace_events` data without mixing local `.scryrs` artifacts into live mode.
- Add dashboard same-origin proxy endpoints for live sessions and events.
- Reuse existing frontend session and event views against live API responses.
- Add Sessions and Events to live navigation.
- Preserve explicit unavailable behavior only for capabilities that remain local-only.
- Add pagination, repository scoping, deterministic ordering, and upstream error handling.

## Capabilities

### New Capabilities

- `live-dashboard-sessions-events`: Server-backed sessions and event inspection for live dashboard mode.

### Modified Capabilities

- `live-dashboard-mode`: Sessions and Events become available through live server APIs instead of being live-mode unavailable.

## Impact

- `crates/scryrs-server`: new read-only repository-scoped session and event endpoints plus store queries and indexes.
- `crates/scryrs-dashboard/src/server.rs`: live proxy handlers and response normalization.
- `crates/scryrs-dashboard/frontend`: live-compatible session/event stores, views, navigation, and error states.
- Server HTTP contract and dashboard live-mode contract gain new endpoints.
- No proposal, route-manifest, or local artifact synchronization is included.
