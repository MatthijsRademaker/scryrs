## 1. Hotspot store polling lifecycle

- [ ] 1.1 Add `startPolling(intervalMs?: number)`, `stopPolling()`, `isPolling`, `lastUpdated`, `staleError`, `pollState` (idle/polling/updating/stale/error) to the hotspot store
- [ ] 1.2 Implement `setInterval`-based polling loop that calls `getHotspots()` and updates `report`, preserving last successful report on failure in a `lastGoodReport` ref
- [ ] 1.3 Add in-flight guard: skip the poll tick if `loading` is already true; set `loading` before fetch, clear in `finally`
- [ ] 1.4 Wire tab-visibility pause/resume: pause polling on `document.hidden === true`, resume with immediate fetch on visibility change (500ms debounce)
- [ ] 1.5 Add `onUnmounted` cleanup in `HotspotsView.vue`: call `store.stopPolling()` when the view unmounts
- [ ] 1.6 Wire polling start in `HotspotsView.vue`: call `store.startPolling()` in `onMounted` when `meta.isLiveMode` is true, after `meta.ensureLoaded()`

## 2. Client-side delta computation

- [ ] 2.1 Retain previous entries snapshot (`prevEntries`) in the store, keyed by `(subjectKind, subject)` composite key
- [ ] 2.2 Compute diff on each successful poll: derive `entered`, `exited`, and `changed` sets; expose as reactive refs
- [ ] 2.3 Flag first-load vs. subsequent poll: first load sets entries without marking them as animation candidates; only subsequent poll deltas trigger motion
- [ ] 2.4 Expose computed properties: `entriesWithDelta` (entry + delta type: 'entered' | 'changed' | 'unchanged'), `scoreIncreased` (boolean per entry), `rankChanged` (boolean per entry)

## 3. Hero card motion-v animation

- [ ] 3.1 Replace CSS `scry-in` class on hero cards with `motion-v` `Motion` components, conditionally: live mode uses motion-v, local mode preserves `scry-in`
- [ ] 3.2 Add `:layout="true"` to hero card `Motion` components for rank-change FLIP reordering
- [ ] 3.3 Wrap hero card section in `<AnimatePresence>` for enter/exit of new/dropped entries
- [ ] 3.4 Implement score count-up animation on hero cards: only on `scoreIncreased === true` for subsequent polls, ease-out cubic capped at 800ms, using rAF tween (reuse SignalRow pattern)
- [ ] 3.5 Implement entrance spring for new entrants (subsequent polls only): `initial={{ opacity: 0, y: -14, scale: 0.97 }}` with spring transition staggered via `nextIgnitionDelayMs()`
- [ ] 3.6 Map rank-change FLIP via `layout` prop: cards reorder with spring physics when their server rank changes between polls

## 4. Detail table score-change highlights

- [ ] 4.1 Add CSS class `score-flash` with a brief oklch transition (~1s) on the score cell when value changes from previous poll
- [ ] 4.2 Bind `score-flash` conditionally based on `scoreIncreased` delta flag (use a per-entry `:class` binding or a local flash key)
- [ ] 4.3 Preserve user sort state (`sortKey`, `sortAsc`) across polls — sort controls remain functional and stable
- [ ] 4.4 Suppress score flash in reduced-motion: use `prefers-reduced-motion` media query in the CSS class definition

## 5. Live refresh health UI

- [ ] 5.1 Add a live-status badge to the Hotspots view header showing current `pollState` (polling/updating/stale/error) with appropriate Badge variant
- [ ] 5.2 Add a `lastUpdated` relative timestamp display (e.g., "Updated 12s ago")
- [ ] 5.3 Add retry affordance for stale/error state: a "Retry" button that triggers a manual poll (reuse existing `Alert` + `Button` pattern from current error display)
- [ ] 5.4 Preserve current error `Alert` for initial load failures (no data at all) — distinct from stale poll state

## 6. Automated frontend coverage

- [ ] 6.1 Create `crates/scryrs-dashboard/frontend/src/stores/hotspots.test.ts` modeled after `signals.test.ts`
- [ ] 6.2 Test poll lifecycle: `startPolling()` creates interval, `stopPolling()` clears it, timer cleanup on store teardown
- [ ] 6.3 Test in-flight guard: tick skipped when `loading` is true, fetch proceeds when not loading
- [ ] 6.4 Test error preservation: last good report survives failed polls, `staleError` set, entries still accessible
- [ ] 6.5 Test delta computation: entering, exiting, score-changing, and rank-changing entries are correctly classified
- [ ] 6.6 Test tab-visibility pause/resume: polling pauses when `document.hidden` is true, resumes on visibility change
- [ ] 6.7 Test first-load vs subsequent poll distinction: first load does not flag entries as `entered`/`changed`

## 7. Verification

- [ ] 7.1 Frontend lane: `vue-tsc` typecheck passes, `vitest` all tests pass (including new hotspots.test.ts), `vite build` succeeds
- [ ] 7.2 Manual smoke: live dashboard → Hotspots view polls and updates without full refresh, hero cards animate on rank/score change, table highlights score changes, badge shows live state
- [ ] 7.3 Local mode regression: Hotspots view in local mode works identically to before (one-shot load, no polling, `scry-in` still works)
- [ ] 7.4 Reduced-motion verification: with OS reduced-motion enabled, hero card motion collapses to calm fade, no count-up or spring
- [ ] 7.5 Error/edge cases: poll failure preserves last-good data with stale badge and retry; back-to-back failures do not stack errors; navigating away and back restarts polling cleanly