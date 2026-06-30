## Context

The dashboard's live Signals view (`SignalsView.vue`) renders a Server-Sent-Events stream of hotspot signals into a static `<table>`, appending rows at the bottom with no motion. Signals carry `score`, `threshold`, and `delta` and arrive in two phases: a **replay batch** (persisted signals replayed from the `after` cursor on connect) followed by a **live tail** (new signals as they fire). The store (`stores/signals.ts`) pushes signals in arrival order and exposes no notion of "which signals are live."

The existing design system is disciplined: dark-first oklch tokens, a flame gradient reserved exclusively for hotspot heat, glow tokens as additive decoration, a `scry-in`/`scry-glow`/`pulse-dot` motion vocabulary, and a global `prefers-reduced-motion` kill-switch. The `dashboard-visual-design` spec currently forbids new animation dependencies. We are deliberately relaxing that one constraint to gain real spring physics.

The target feel was decided in exploration: **Apple-physical and restrained** — drama on arrival, stillness at rest — applied first to the Signals surface.

## Goals / Non-Goals

**Goals:**
- Reframe Signals from a table into a newest-first live feed.
- One signature, data-driven "arrival" motion: enter from top → feed makes room with weight → flame flares → score counts up → settles to stillness.
- Motion magnitude is a function of the data (`delta` → flare/overshoot, `score` → resting flame intensity).
- Replay batch is calm (no flares); only live-tail signals ignite.
- Reduced-motion is honored via a single source of truth shared with the existing CSS.

**Non-Goals:**
- No FLIP leaderboard on the Hotspots view (future thread).
- No unified signals+rankings surface (future thread).
- No globally reactive ambient aurora (future thread).
- No change to the SSE protocol, reconnect logic, or signal payload shape.
- No hero/now-playing element — every signal is equal; motion carries hierarchy.

## Decisions

### Decision: Adopt `motion-v` for spring physics

**Choice**: Add `motion-v` (the official Vue port of Motion / Framer Motion lineage) as the animation library for the feed.

**Rationale**: The hardest part of the effect — the displaced rows "making room with weight" — is layout FLIP with real spring physics. `motion-v`'s `layout` prop delivers this in one prop, and it also provides `AnimatePresence` for enter/exit and `useSpring`/`animate` for the number count-up.

**Alternatives considered**:
- *CSS-only + `<TransitionGroup>`*: zero dependencies, fits the existing token discipline, but FLIP springs are approximated and the count-up must be hand-rolled with rAF. Rejected because the user explicitly chose higher fidelity.
- *`@vueuse/motion`*: lighter and directive-based, but its layout-reorder story is weaker and number springs are less ergonomic. Rejected for the same fidelity reason.

### Decision: Single source of truth for reduced-motion

**Choice**: Wrap the feed (or app) in `<MotionConfig reducedMotion="user">` so the library defers to the OS `prefers-reduced-motion` setting — the same signal the existing global CSS kill-switch uses.

**Rationale**: Avoid two competing reduced-motion systems. When the user prefers reduced motion, arrival collapses to a simple opacity fade with no flare, no overshoot, no count-up.

### Decision: Distinguish replay batch from live tail in the store (quiescence heuristic)

**Choice**: Add a `live` boolean to each stored signal and expose newest-first ordering. Liveness is determined by a **stream-quiescence heuristic**, not `onopen`.

**Why not `onopen`**: The server (`scryrs-server/src/server.rs::get_signals_sse`) chains Phase 1 (replay of persisted signals with `id > after`) directly into Phase 2 (live broadcast) on the *same* SSE stream, with **no boundary marker** — both phases arrive as ordinary `onmessage` events carrying only `id` + `data`. `onopen` fires when the HTTP stream starts, *before* the replay burst, so it cannot separate replay from live. The proposal forbids protocol changes, so the boundary must be inferred client-side.

**How**: The replay batch arrives as a back-to-back burst at connect. The store arms a short settle timer (`REPLAY_SETTLE_MS`, ~350ms) on open and re-arms it on each incoming signal while still catching up. Signals received before the stream goes quiet are marked `live: false` (replay → calm fade); once the stream has been quiet for the settle window, the phase flips to `live` and subsequent signals are marked `live: true` (→ ignite). This derives liveness from stream *timing*, never from signal *content*, satisfying the spec. On reconnect the same logic re-runs, so catch-up after a drop is also calm.

**Trade-off**: A genuine live signal arriving within `REPLAY_SETTLE_MS` of the replay burst is misclassified as replay (it fades instead of igniting once). This is rare and the failure mode is restraint-friendly.

### Decision: Data-driven motion mapping

**Choice**:
- `delta` (jump past threshold) → flare magnitude and spring overshoot/settle duration. A small delta is a gentle ripple; a large delta is a bright, longer flare.
- `score` (absolute heat) → resting flame intensity, reusing `FlameIndicator`'s existing intensity scaling.

**Rationale**: This is the anti-decoration discipline — a big animation literally means a big heat event. It keeps the motion "suited to the application" rather than ornamental.

## Risks / Trade-offs

- **[New dependency contradicts the existing visual-design spec]** → This change explicitly modifies `dashboard-visual-design` to permit `motion-v`; the restraint discipline (single soft settle, calm at rest, no cartoonish bounce) is preserved in the modified requirement.
- **[Replay strobe on connect]** → Mitigated by the `isLive` flag: replayed history fades in calmly; only live-tail signals ignite.
- **[Spring overshoot reading as "bouncy/cartoonish"]** → Constrain to a single soft settle (slightly underdamped, no oscillation); tune so chrome-level transitions still honor the ~200–400ms ease-out budget and only data-bearing arrival motion runs longer.
- **[Bundle size / dependency drift]** → Accepted trade-off for fidelity; scoped to the dashboard frontend only.
- **[Burst of live signals causing many simultaneous ignitions]** → Stagger or cap concurrent ignitions so a rapid live burst settles gracefully rather than strobing.

## Migration Plan

1. Add `motion-v` to the dashboard frontend `package.json`.
2. Extend `stores/signals.ts` with newest-first ordering + `isLive`.
3. Build the feed-row component and the data-driven arrival motion.
4. Replace the table in `SignalsView.vue` with the feed.
5. Wire reduced-motion via `<MotionConfig>`.

Rollback is a straight revert of the frontend changes and the dependency; no backend, protocol, or data-model changes are involved.

## Open Questions

- Exact spring constants (response/damping) and the `delta → magnitude` curve — to be tuned during implementation against real signal data.
- Whether `<MotionConfig>` wraps the whole app or only the Signals feed (prefer app-level for consistency if other views later adopt the library).
