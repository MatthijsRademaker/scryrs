## Context

The dashboard's Hotspots view (`HotspotsView.vue`) is the landing page and primary leaderboard surface. It loads a `HotspotsReport` once via `store.load()` on mount and renders top-3 hero cards (glass cards with flame intensity, event-type bars, outcome pulse) plus a sortable detail table. There is no auto-refresh, no live health indicator, and no motion beyond a one-time CSS `scry-in` entrance animation on the hero cards.

The Signals view (`SignalsView.vue` + `SignalRow.vue`) was recently redesigned with `motion-v` spring physics for data-driven arrival motion — the hardest part being layout FLIP reordering when rows enter from the top. The `live-signal-feed-motion` change established the motion language (data-driven flare + count-up + single soft settle), the store-owned lifecycle pattern (`start()`/`stop()`), and the reduced-motion strategy (`<MotionConfig reduced-motion="user">` at app level). That change explicitly deferred the Hotspots FLIP animation to a future thread.

The hotspot data model is snapshot-based: `GET /api/hotspots` returns a full cumulative `HotspotsReport` with `entries: HotspotEntry[]` — no per-entry delta field. This is a key difference from Signals, where each SSE event carries a `delta` representing a single event's score contribution. For Hotspots, motion must be derived from client-side diffing between consecutive poll responses.

The existing dashboard infrastructure already supports this work: `motion-v` is installed, `<MotionConfig reduced-motion="user">` wraps the app, `getHotspots()` exists in the API client, `useMetaStore.isLiveMode` distinguishes live from local, and `signals.test.ts` provides the exact fake-timer testing pattern.

## Goals / Non-Goals

**Goals:**
- Make the Hotspots view auto-refresh in live dashboard mode via store-owned polling against `/api/hotspots`.
- Apply restrained, data-driven motion to hero cards (rank-change FLIP layout, score-increase count-up, entrance for new entrants) using the existing `motion-v` library.
- Apply lighter CSS score-change highlights to the sortable detail table without FLIP layout reordering.
- Expose real-time refresh health: polling/updating/stale/error badge with last-updated timestamp and retry affordance.
- Preserve last successful report across poll failures instead of blanking the page.
- Add automated coverage for the polling store lifecycle.
- Reuse existing infrastructure: `motion-v`, `<MotionConfig>`, `getHotspots()`, `useMetaStore.isLiveMode`, `nextIgnitionDelayMs()`.

**Non-Goals:**
- No new backend endpoints, SSE contracts, or server changes.
- No changes to hotspot scoring, ranking semantics, or the meaning of score/heat.
- No redesign of the Signals page or merging of Hotspots + Signals.
- No polling or motion in local dashboard mode (local remains snapshot-based with manual retry).
- No dashboard chrome refactors or changes to non-Hotspots views.
- No new animation dependency — `motion-v` is already present.
- No animation of detail table rows with FLIP layout springs (table layout + user-sort would conflict).

## Decisions

### Decision 1: Store-owned polling lifecycle with in-flight guard and tab-visibility pause

**Choice**: Add `startPolling(intervalMs?: number)`, `stopPolling()`, `isPolling`, `lastUpdated`, and `staleError` to the hotspot store. Polling fetches `/api/hotspots` on a `setInterval` tick. If a fetch is in-flight when the next tick fires, the tick is skipped. Polling pauses when `document.hidden` becomes true and resumes with a fresh fetch when the tab becomes visible (500ms debounce). On poll failure, the last successful report is preserved and `staleError` is set; the view shows a staleness indicator with retry.

**Rationale**: The signals store's `start()`/`stop()` pattern and the acceptance criteria's "does not issue overlapping fetches" and "stops when the view unmounts" requirements dictate a store-owned lifecycle. The tab-visibility pause is a resource-efficiency measure consistent with modern dashboard practice. The in-flight guard prevents request stacking on slow connections.

**Alternatives considered**:
- *`usePolling` composable in the view*: simpler but scatters timer management and makes testing harder. Store-owned lifecycle mirrors signals and keeps test surface centralized.
- *AbortController per request*: cleaner cancellation but aborts the in-flight request, which means the response is discarded even if it arrives a moment later. An inflight flag skips the next tick but lets the current request finish — better for error resilience.

### Decision 2: Client-side delta computation between successive poll snapshots

**Choice**: The store retains the previous snapshot (`prevEntries`) and computes a diff on each successful poll. Entries are keyed by the composite `(subjectKind, subject)` key. Three sets are derived: `entered` (key in new but not old), `exited` (key in old but not new), and `changed` (key in both but score or rank differs). Only entries in `entered` or `changed` receive animation treatment; unchanged entries update silently.

**Rationale**: Without diffing, every poll would re-animate all entries, creating visual noise. The Signals SSE model avoids this because each signal arrives once. For polling replacement, only genuinely changed items should move. The composite key `(subjectKind, subject)` is the natural identity — used already in `:key` bindings.

**Alternatives considered**:
- *Server-side delta API*: requires a new backend contract, explicitly out of scope.
- *Animate everything every tick*: simple but visually noisy and breaks the restraint discipline.

### Decision 3: Split motion treatment between hero cards and detail table

**Choice**:
- **Hero cards** (top 3 by server rank): Full `motion-v` treatment — `Motion` components with `:layout="true"` for rank-change FLIP, `AnimatePresence` for enter/exit, score-increase count-up (rAF tween, capped at 800ms), and entrance spring for new entrants. The existing CSS `scry-in` class is replaced with motion-v in live mode (`cond:` guard).
- **Sortable detail table**: No `layout` prop (avoids table/FLIP conflicts). Rows are keyed by `(subjectKind, subject)` for identity. Score cells apply a brief CSS oklch transition highlight (~1s) when the value changes from the previous poll. User sort state (`sortKey`, `sortAsc`) is preserved across polls.

**Rationale**: The hero cards are styled `<div>` elements in a CSS grid — ideal for `motion-v` layout animation. The detail table uses native `<table>`/`<tbody>`/`<tr>` markup where `layout` FLIP can cause browser rendering instability (table layout engine fighting CSS transforms). Additionally, user-controlled sorting means row positions are not purely data-driven — animating layout when the sort column is not `rank` would be disorienting.

**Alternatives considered**:
- *Full FLIP on the table*: technically possible with `display: contents` or a virtualized list, but adds complexity disproportionate to the benefit and risks jank.
- *No table animation at all*: misses the opportunity for subtle score-change feedback that users find useful.

### Decision 4: Score count-up animation constraints

**Choice**: Count-up only runs on score *increase* for hero cards. Score decreases update silently (no count-down) to avoid misleading emphasis — a hot subject cooling down should not animate like progress. Duration is capped at 800ms regardless of score magnitude, using ease-out cubic easing. For first render, scores appear at their final value (no count-up on initial load).

**Rationale**: Count-down on score decrease would misleadingly emphasize a cooling subject. The 800ms cap prevents large score values (>1000) from exceeding acceptable settle time. First-render count-up would conflict with the initial page load and feel like stutter rather than live data.

### Decision 5: Live refresh badge states

**Choice**: A polling-specific state model distinct from Signals' SSE `connectionState`:
- `idle`: Not polling (local mode or stopped).
- `polling`: Interval active, waiting for next tick.
- `updating`: Fetch in-flight.
- `stale`: Last poll failed but last-good data is displayed.
- `error`: No data at all (initial load or all polls have failed).

The badge maps these to shadcn-vue `Badge` variants: `success` (polling with data), `info` (updating), `warning` (stale), `destructive` (error). A `lastUpdated` timestamp shows relative time since the last successful poll.

**Rationale**: The Signals view uses `connectionState` (connecting/connected/reconnecting/error) because SSE is a persistent connection. Polling has fundamentally different states — idle/interval-driven/fetch-in-flight/stale — that map poorly onto connection semantics. A distinct model is clearer.

## Risks / Trade-offs

- **[Polling load on backend]** → Default 15s interval + tab-visibility pause + in-flight guard keeps load low. The existing `GET /api/hotspots` proxy already serves each request independently; polling is a frequent caller, not a new protocol.
- **[Delta computation correctness on score decrease]** → Score decreases (from deduplication in the accumulator) are handled as calm updates — no count-down, no flare. The diff correctly identifies the entry as `changed` but the animation system treats score-decrease as a non-event.
- **[User sort + poll data race]** → When a user sorts by a non-rank column and a poll refreshes the data, `sortedEntries` recomputes reactively. Row positions may shift. This is acceptable because the table uses no FLIP layout — rows simply update their displayed values. The user's sort key and direction are preserved.
- **[scry-in CSS class removal in local mode]** → The `scry-in` class provides a one-time entrance stagger on initial load. In live mode it's replaced by motion-v. In local mode it is preserved as-is. The view conditionally applies `scry-in` only when `!meta.isLiveMode`.
- **[Hero card motion during first render]** → On the very first poll (or initial `load()`), all entries are "new" from the diff perspective but should not all ignite simultaneously. The store distinguishes "first load" from "subsequent poll" — first-load entries appear at their final position without entrance animation (matching the current behavior), while only genuinely new entrants on subsequent polls animate in.

## Traceability

| Source | Contribution |
|---|---|
| Task 7160e351-6c4c-43af-b84f-22018f28d96d | User request: apply live-signal-feed-motion treatment to Hotspots + polling |
| Dossier | Problem framing, goals/non-goals, affected areas, acceptance criteria |
| swarm-architect (round 1) | Store polling lifecycle, hero/table motion split, live badge, test coverage pattern |
| swarm-lead-dev (round 1) | Delta computation, motion split rationale, 15s default interval, tab-visibility, hotspots.test.ts |
| swarm-reviewer (round 1) | In-flight guard, diff-based animation, scry-in removal, onUnmounted cleanup, last-good-report |
| live-signal-feed-motion change | Motion language precedent, store lifecycle pattern, reduced-motion strategy, AnimatePresence + Motion usage |
| hotspots.ts | Current store lacks polling lifecycle/state |
| HotspotsView.vue | Current view one-shot loads, uses scry-in CSS, no live badge |
| signals.ts / signals.test.ts | Reference: store lifecycle and fake-timer test pattern |
| SignalRow.vue | Reference: motion-v layout, count-up, flare, reduced-motion collapse |
| App.vue | Confirms MotionConfig already wraps the app |
| client.ts | getHotspots() exists; no streaming helper for hotspots |
| server.rs | /api/hotspots is GET proxy; no SSE for hotspots |
| live-dashboard-mode spec | Requires /api/hotspots proxying, mode-aware copy, live-only Signals nav |
| dashboard-visual-design spec | Modified by live-signal-feed-motion to permit motion-v for data-bearing motion |