## Why

The live Signals view is the most real-time surface in the dashboard, yet it is the least alive: signals render as rows appended to the bottom of a static table with no motion. A signal is a subject crossing its heat threshold — the most dramatic moment in the domain — and it currently arrives silently. We want the live feed to feel premium and physical (Apple-style: spring-based, restrained, weighty) so that an arriving signal is something you *feel*, with the magnitude of the motion driven by the data itself.

## What Changes

- Reframe the Signals view from a dense `<table>` into a **live, newest-first feed** of signal rows with breathing room.
- Introduce one signature **"arrival" motion** reused for every live signal: the row lands from the top, the feed below makes room with spring physics (layout FLIP), the heat indicator flares once, and the score counts up before settling to stillness.
- Make motion **data-driven**: the signal `delta` (how far it jumped past threshold) scales the flare/overshoot magnitude; `score` scales the resting flame intensity. No two arrivals animate identically unless the data is identical.
- Distinguish **replay batch from live tail**: signals replayed on connect fade in calmly (no flares — they are history), and only signals arriving after the stream is live get the full ignition treatment. Avoids a strobe on page load.
- Add a spring-physics animation library (`motion-v`) for high-fidelity springs, layout FLIP reordering, and number count-up. **BREAKING (spec-level)**: this relaxes the existing "no new animation dependency" constraint in `dashboard-visual-design`.
- Honor reduced-motion: defer the library's reduced-motion handling to the OS setting so it agrees with the existing global `prefers-reduced-motion` CSS kill-switch (single source of truth).
- Adjust the signals store to expose **newest-first ordering** and an **`isLive` flag** marking signals that arrived after the stream opened.

## Capabilities

### New Capabilities
- `live-signal-feed-motion`: The animated live Signals feed — newest-first layout, the data-driven arrival motion (enter, layout settle, flame flare, score count-up), the replay-vs-live distinction, and reduced-motion behavior.

### Modified Capabilities
- `dashboard-visual-design`: The "Motion is spring-based and restrained" requirement changes to permit a dedicated spring-physics library (`motion-v`) for data-bearing motion, while preserving the restraint discipline (a single soft settle, never cartoonish bounce; calm at rest; reduced-motion honored). The "no new animation dependency" scenario is replaced accordingly.

## Impact

- **Frontend code**: `crates/scryrs-dashboard/frontend/src/views/SignalsView.vue` (table → feed), `src/stores/signals.ts` (ordering + `isLive` flag), and likely a new feed-row component plus a small motion helper. The `FlameIndicator` heat token is reused for the flare.
- **Dependencies**: adds `motion-v` to `crates/scryrs-dashboard/frontend/package.json`.
- **Specs**: modifies `dashboard-visual-design`; adds `live-signal-feed-motion`.
- **Out of scope (future threads)**: FLIP-animated leaderboard on the Hotspots view, a unified signals+rankings live surface, and a globally reactive ambient aurora.
