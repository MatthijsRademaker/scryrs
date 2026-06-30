## Why

The live Signals surface now feels premium and continuously alive — rows enter from the top with spring-driven layout reordering, the flame flares once, and the score counts up before settling to stillness. But the Hotspots landing page (the primary leaderboard) still renders a one-time snapshot from `store.load()` and stays frozen until manual retry or page refresh. That creates a jarring split inside the live dashboard: Signals animate and tail updates, while the ranking leaderboard neither auto-refreshes nor shows rank changes in motion.

The `live-signal-feed-motion` change explicitly deferred a "FLIP-animated leaderboard on the Hotspots view" as a future thread. This is that follow-on: make Hotspots feel live, reuse the same restrained motion system, and do it without changing hotspot scoring semantics or introducing a new server push contract.

## What Changes

- **Extend the Pinia hotspot store** with a polling lifecycle (`startPolling` / `stopPolling`, in-flight guard, tab-visibility pause/resume, `lastUpdated` timestamp, `staleError` state) that fetches `/api/hotspots` on a configurable interval (default 15s) while preserving backward-compatible `load()` for local mode. Polling runs only in live dashboard mode.
- **Add client-side delta computation** between successive poll snapshots so that only entries with genuine rank, score, or membership changes receive motion — not all entries on every tick. The store retains the previous snapshot and derives `entered`, `changed`, and `unchanged` sets keyed by `(subjectKind, subject)`.
- **Apply restrained motion-v animation to the hero cards** (top 3 entries by server rank): rank-change FLIP layout via `layout` prop, score-increase count-up (upward only, capped at 800ms), and entrance motion for new entrants. The existing `scry-in` CSS entrance class is replaced with motion-v in live mode to avoid double-animation.
- **Apply lighter CSS-based score-change highlights to the sortable detail table** without FLIP layout reordering (which would conflict with table layout and user-controlled sorting). Score cells highlight briefly on change; reduced-motion suppresses the highlight.
- **Expose live refresh health** on the Hotspots view: a polling/updating/stale/error badge plus a last-updated relative timestamp, following the Signals connection badge pattern. The last successful report is preserved when a poll fails, with a retry affordance.
- **Add automated frontend coverage** in `hotspots.test.ts` modeled after `signals.test.ts`: fake-timer-based tests covering poll lifecycle, overlapping-fetch guard, error/last-good-report preservation, delta computation, tab-visibility pause/resume, and cleanup on unmount.

## Impact

- **Frontend code**: `crates/scryrs-dashboard/frontend/src/stores/hotspots.ts` (polling lifecycle + delta diff), `src/views/HotspotsView.vue` (motion-v hero cards + live badge + table highlights), and new `src/stores/hotspots.test.ts`.
- **Dependencies**: None new — `motion-v` is already installed, `getHotspots()` already exists in the API client, and `<MotionConfig reduced-motion="user">` already wraps the app.
- **Backend**: None — the existing `GET /api/hotspots` proxy is the sole transport; no new SSE endpoint or server contract.
- **Specs**: Adds `live-hotspot-polling-motion` as a new capability. Does not modify existing canonical specs.
- **Out of scope**: New backend streaming contracts, changes to hotspot scoring/ranking semantics, merged Signals+Hotspots surface, local-mode polling against `.scryrs/hotspots.json`, and animation of non-ranking detail table rows with FLIP springs.