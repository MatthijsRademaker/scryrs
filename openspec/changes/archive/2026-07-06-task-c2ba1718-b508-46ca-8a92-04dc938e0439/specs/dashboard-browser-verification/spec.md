## ADDED Requirements

### Requirement: Live dashboard browser verification is an opt-in automated lane

The repository SHALL provide `scripts/verify-live-dashboard-smoke` as an opt-in, Docker-backed browser verification lane for the dashboard live Signals view. The lane SHALL NOT be part of `scripts/verify-production-suite`. The lane SHALL exit non-zero on test failure and SHALL follow the existing Docker verification pattern (build binary in Rust container, run fixture in a Node/Playwright container). `scripts/verification/README.md` SHALL document the lane, its prerequisites, and its exclusion from the production suite.

#### Scenario: Maintainer runs the browser smoke explicitly

- **GIVEN** Docker or DinD is available
- **WHEN** a maintainer runs `scripts/verify-live-dashboard-smoke`
- **THEN** the command runs the browser smoke headlessly in a Playwright-capable Docker container
- **AND** the lane header is clearly identified in output
- **AND** the command exits non-zero if the browser smoke fails

#### Scenario: Browser smoke is excluded from the production suite

- **WHEN** `scripts/verify-production-suite` runs
- **THEN** the browser smoke lane is NOT invoked
- **AND** the production suite lane count remains unchanged from its existing 9 lanes

### Requirement: SignalRow exposes stable DOM hooks for replay/live and motion-path state

The `SignalRow.vue` component SHALL expose three stable `data-*` attributes on its root rendered element:

- `data-signal-id` — the numeric signal id, for row identity selection
- `data-signal-phase` — `"replay"` when the signal is replayed history, `"live"` when the signal arrived after the stream opened
- `data-motion-path` — `"full"` when ignition motion is allowed, `"reduced"` when reduced motion is active

These attributes SHALL be the intentional, stable assertion surface for browser tests. Their presence or values SHALL NOT depend on animation timing, pixel state, or incidental rendering artifacts.

#### Scenario: Replay row has replay phase and reduced motion reflects OS preference

- **WHEN** a replayed signal row renders in the live Signals feed
- **THEN** the row root element has `data-signal-phase="replay"`
- **AND** the row root element has `data-motion-path` set to `"reduced"` when reduced motion is active or `"full"` when not

#### Scenario: Live row has live phase

- **WHEN** a live signal row renders in the live Signals feed
- **THEN** the row root element has `data-signal-phase="live"`

### Requirement: Browser smoke verifies replay/live rendering distinction

The browser smoke SHALL open the dashboard Signals route in live mode (`/signals`) against a deterministic mock SSE upstream, wait for replay rows to render, then wait for one live signal to arrive, and assert:

- Every rendered replay row has `data-signal-phase="replay"`
- Exactly one rendered row has `data-signal-phase="live"` after the live signal arrives
- The live signal row appears exactly once (no duplicate signal ids in the DOM)

#### Scenario: Replay rows render with replay phase markers

- **GIVEN** the mock SSE server emits N replay signals back-to-back
- **WHEN** the browser smoke navigates to `/signals` and waits for replay rows to render
- **THEN** all rendered signal rows have `data-signal-phase="replay"`
- **AND** the count of rendered rows equals the count of emitted replay signals

#### Scenario: Live signal is distinguished at the rendered level

- **GIVEN** the mock SSE server has completed the replay burst and emitted one live signal
- **WHEN** the browser smoke waits for the live signal to render
- **THEN** exactly one rendered row has `data-signal-phase="live"`
- **AND** that row's `data-signal-id` matches the live signal's id

### Requirement: Browser smoke verifies reconnect duplicate suppression at rendered level

The browser smoke SHALL force a disconnect (via Playwright route interception of the SSE endpoint), allow the frontend to reconnect with `?after=<lastSeenId>`, and assert that no duplicate signal ids appear in the rendered DOM after reconnection.

#### Scenario: Reconnect does not produce duplicate visible rows

- **GIVEN** the browser has already rendered replay and live signals
- **WHEN** the SSE stream is interrupted and the frontend reconnects with its last-seen cursor
- **THEN** the rendered DOM does not contain duplicate `data-signal-id` values
- **AND** the total count of unique signal ids in the DOM matches the expected total

### Requirement: Browser smoke verifies reduced-motion rendering path

The browser smoke SHALL re-run with `page.emulateMedia({ reducedMotion: 'reduce' })`, wait for a live signal to render, and assert that the live signal row has `data-motion-path="reduced"`. The assertion SHALL use the stable DOM attribute — not screenshots, pixel comparisons, or animation-timing checks.

#### Scenario: Reduced motion collapses the rendering path

- **GIVEN** Playwright emulates `prefers-reduced-motion: reduce`
- **WHEN** a live signal arrives and renders
- **THEN** the live signal row has `data-motion-path="reduced"`

#### Scenario: Reduced-motion assertion does not depend on pixel checks

- **WHEN** the reduced-motion browser smoke assertion runs
- **THEN** it reads `data-motion-path` from the DOM
- **AND** it does not use screenshot comparison, pixel analysis, or animation timing measurement

### Requirement: Deterministic mock SSE server provides controllable replay/live/disconnect timing

The verification lane SHALL include a lightweight Node.js mock HTTP server (`scripts/verification/lib/mock-live-server.mjs`) that:

- Speaks the scryrs SSE protocol at `GET /v1/repositories/{repository_id}/signals?after=N`
- Emits replay signals (id 1 through N) back-to-back with <100ms inter-signal gaps
- Falls silent for ≥500ms after the replay burst before emitting any live signal
- Supports a scripted SSE disconnect trigger for reconnect testing
- On reconnect with `?after=<cursor>`, replays only signals with id > cursor
- Stubs `GET /v1/repositories/{repository_id}/hotspots` with a minimal valid response for dashboard mode detection

#### Scenario: Replay burst completes within settle window

- **GIVEN** the mock server is configured with N replay signals
- **WHEN** a client connects to the SSE endpoint
- **THEN** all N replay signals are emitted back-to-back with <100ms between each
- **AND** the entire replay burst completes within `N * 100ms`

#### Scenario: Quiet window exceeds quiescence heuristic

- **GIVEN** the mock server has finished emitting all replay signals
- **WHEN** the stream continues
- **THEN** no signals are emitted for ≥500ms before the first live signal

#### Scenario: Scripted disconnect induces frontend reconnect

- **GIVEN** a client is connected to the SSE endpoint
- **WHEN** the mock server triggers a scripted disconnect (terminates the SSE response)
- **THEN** the client's native `EventSource.onerror` fires
- **AND** the mock server accepts a subsequent reconnect request with `?after=<cursor>`

### Requirement: Verification README documents the browser smoke lane

`scripts/verification/README.md` SHALL document:

- The `scripts/verify-live-dashboard-smoke` entrypoint and its purpose
- Runtime prerequisites: Docker/DinD, `mcr.microsoft.com/playwright` Docker image, `--shm-size=256m` for Chromium
- The lane's posture: opt-in manual, NOT part of `scripts/verify-production-suite`
- The stable `data-*` attribute contract used as the assertion surface
- That the existing "Live dashboard manual smoke boundary" section now has an automated alternative

#### Scenario: Lane documentation is complete and accurate

- **WHEN** `scripts/verification/README.md` is read after the change
- **THEN** the document describes `scripts/verify-live-dashboard-smoke`
- **AND** it lists Docker image and `--shm-size` prerequisites
- **AND** it states the lane is opt-in and excluded from the production suite
- **AND** it documents the `data-signal-id`, `data-signal-phase`, and `data-motion-path` attribute contract
