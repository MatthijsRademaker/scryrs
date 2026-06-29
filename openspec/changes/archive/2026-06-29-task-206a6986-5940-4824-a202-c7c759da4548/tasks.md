## 1. Add explicit live dashboard mode plumbing

- [x] 1.1 Add `--server-url` and `--repository-id` to `scryrs dashboard`, keep local mode as the default, and fail startup when only one live flag is provided.
- [x] 1.2 Extend dashboard configuration with a `SourceMode` split so the backend can dispatch between local and live data sources.
- [x] 1.3 Expand `/api/meta` to return dashboard mode and live repository identity when live mode is active.
- [x] 1.4 Add the live-mode backend HTTP/SSE proxy dependencies required for streaming proxy behavior in the Docker build environment.

## 2. Implement live backend APIs

- [x] 2.1 Proxy `/api/hotspots` to `GET /v1/repositories/{repository_id}/hotspots?window=cumulative` in live mode and normalize the response for the current hotspot UI.
- [x] 2.2 Distinguish live upstream-unavailable errors from valid empty live results in `/api/hotspots` responses.
- [x] 2.3 Add `/api/signals` as a streaming SSE proxy that forwards the optional `after` cursor and relays replayed plus live signals without buffering the full upstream response.
- [x] 2.4 Return clear live-mode unavailable errors from `/api/sessions`, `/api/sessions/:session_id`, and `/api/events`.
- [x] 2.5 Add live-mode backend tests that exercise the HTTP proxy code path against a mock or spawned upstream server.

## 3. Implement live-mode frontend behavior

- [x] 3.1 Extend the typed dashboard API client and meta store for `mode`, `repositoryId`, live hotspot responses, and signal stream support.
- [x] 3.2 Implement a signal stream store/composable that owns `EventSource` lifecycle, tracks `lastSeenId` in memory, reconnects with `?after=<lastSeenId>`, and avoids duplicate appends on replay.
- [x] 3.3 Add a live-mode Signals route/view using existing shadcn-vue primitives to display signal id, subject, kind, score, threshold/delta, timestamp, and connection state.
- [x] 3.4 Update Hotspots copy and subject rendering for live mode so the view no longer implies `.scryrs/hotspots.json` as the source.
- [x] 3.5 Make the shell mode-aware: show Signals in live mode, hide Sessions/Events from live navigation, render unavailable views for direct local-only routes, and update footer/About copy.

## 4. Preserve local mode and document the workflow

- [x] 4.1 Keep existing local dashboard tests passing without live configuration.
- [x] 4.2 Update `.devagent/docs/docs/cli-v0-contract.md`, `.devagent/docs/docs/live-hotspots.md`, `.devagent/docs/docs/roadmap.mdx`, and `scripts/verification/README.md` for live dashboard startup, rankings, signals, reconnect behavior, and local-vs-live differences.
- [x] 4.3 Document or add the live dashboard verification flow so operators can smoke-test rankings fetch plus signal replay/resume.
