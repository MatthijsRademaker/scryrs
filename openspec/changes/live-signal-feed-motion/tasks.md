## 1. Dependency & motion foundation

- [x] 1.1 Add `motion-v` to `crates/scryrs-dashboard/frontend/package.json` and install it
- [x] 1.2 Wrap the dashboard (app-level) in `<MotionConfig reducedMotion="user">` so the library defers to the OS reduced-motion setting, agreeing with the existing global `prefers-reduced-motion` CSS kill-switch
- [x] 1.3 Verify reduced-motion is governed by a single shared source of truth (no competing second system)

## 2. Signals store: live vs. replay

- [x] 2.1 Mark each stored signal with whether it arrived live or as part of the replay batch — derived from a stream-quiescence heuristic (timing, not signal content). NOTE: the design's `onopen` boundary was corrected; the SSE protocol has no replay/live marker, so liveness is inferred from when the catch-up burst falls quiet (see updated design.md)
- [x] 2.2 Expose newest-first ordering suitable for the feed (`feed` getter)
- [x] 2.3 Update/extend `stores/signals.test.ts` to cover the live/replay flag and ordering

## 3. Feed redesign

- [x] 3.1 Replace the `<table>` in `SignalsView.vue` with a newest-first feed layout, preserving connection badge, error/reconnect affordance, and empty state
- [x] 3.2 Create a signal feed-row component (`SignalRow.vue`) surfacing subject, kind, score, threshold, and delta with breathing room
- [x] 3.3 Reuse `FlameIndicator` for the per-row resting heat, intensity scaled by `score`

## 4. Data-driven arrival motion

- [x] 4.1 Implement the signature arrival motion on live-tail rows: enter from top + `AnimatePresence`
- [x] 4.2 Use `motion-v` `layout` so rows below settle to make room with spring physics (single soft settle, no oscillating bounce)
- [x] 4.3 Implement the one-shot heat flare on arrival, magnitude scaled by `delta`
- [x] 4.4 Implement the score count-up to its value on arrival (eased rAF tween; motion-v springs reserved for the physical entrance/layout/flare)
- [x] 4.5 Map `delta` → flare/overshoot magnitude and settle (initial spring/flare constants set; final tuning against real signal data flagged for the live pass)
- [x] 4.6 Render replay-batch signals with a calm fade-in (no flare, no count-up); ensure only post-connect live signals ignite
- [x] 4.7 Stagger or cap concurrent ignitions so a rapid live burst settles gracefully rather than strobing (`shared/lib/ignition.ts`)
- [x] 4.8 Implement the reduced-motion path: collapse arrival to a simple opacity fade (no flare, overshoot, layout spring, or count-up)

## 5. Verification

- [x] 5.4 Frontend lane verified green: `vue-tsc` typecheck, `vitest` (16 tests incl. new live/replay + ordering), and the production `vite build` all pass. NOTE: the full `scripts/precommit-run` Docker suite is Rust-only and unrelated to this frontend change — the dashboard E2E tests assert the `/api/signals` proxy/cursor at the API layer, not the rendered markup, so the table→feed change cannot affect them.
- [ ] 5.1 Manually verify in a live environment: connect (replay fades calmly) → live signal ignites once → feed settles to stillness *(requires a running scryrs-server emitting live signals + a browser; logic is unit-tested but visual confirmation is pending)*
- [ ] 5.2 Visually verify high-delta vs. low-delta signals animate with visibly different magnitude *(pending live + browser pass)*
- [ ] 5.3 Visually verify reduced-motion mode collapses the arrival to a fade *(pending browser pass with OS reduced-motion enabled)*
