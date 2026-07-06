## Context

The dashboard's live Signals view (`SignalsView.vue`) renders a Server-Sent-Events stream of hotspot signals through a two-layer pipeline: (1) the Rust backend (`server.rs`) proxies `/api/signals` to the upstream live server, and (2) the Vue frontend (`stores/signals.ts`) manages reconnects with `?after=<lastSeenId>`, dedupes by signal id, and classifies signals as replay or live via a quiescence heuristic (`REPLAY_SETTLE_MS=350ms`).

The `SignalRow.vue` component renders each row with `motion-v` — replay rows fade in calmly, live rows ignite with a data-driven arrival motion (flare + count-up + spring entrance), and `useReducedMotion()` collapses ignition to a simple fade.

Today this is proven at two levels:

- **Store unit tests** (`signals.test.ts`): Covers reconnect cursor reuse, duplicate suppression, and live/replay classification — but at the state level, not the rendered DOM.
- **Backend API tests** (`api.rs`): Covers `/api/signals` proxy forwarding, `after` cursor passthrough, and SSE streaming — but at the HTTP level, not the browser.

The gap is the rendered browser behavior: do replay rows actually fade calmly? Does a live signal show `data-signal-phase="live"`? Does a reconnect avoid visible duplicates in the DOM? Does `prefers-reduced-motion` collapse the ignition path?

Project documentation (`scripts/verification/README.md`, `.devagent/docs/docs/production-suite.md`) explicitly states that live dashboard browser verification is manual and outside the automated production gate.

The existing Docker-backed verification pattern (`scripts/verify-live-hotspots`) provides a proven template: build the Rust binary, run a Node.js fixture in a Docker container, assert behavior. The new lane extends this pattern to browser automation.

## Goals / Non-Goals

**Goals:**

- Add a deterministic, headless, Docker-backed browser smoke for the live Signals view.
- Exercise replay, one new live signal, and reconnect-resume through the rendered dashboard.
- Assert replay/live distinction and reduced-motion path via stable DOM `data-*` attributes — no screenshots, no pixel checks, no animation-timing assertions.
- Verify reconnect cursor prevents duplicate visible rows in the DOM after a disconnect/reconnect cycle.
- Document the lane, its prerequisites, its opt-in posture, and the fact that it is NOT in `scripts/verify-production-suite`.

**Non-Goals:**

- Do not redesign the live feed UX or signal semantics.
- Do not replace or duplicate existing server/API verification lanes.
- Do not add brittle visual snapshot testing or pixel-based animation assertions.
- Do not include in the production suite — keep as an opt-in manual lane until flake data proves routine reliability.
- Do not change the SSE protocol, store logic, or reconnect cursor semantics.
- Do not change canonical OpenSpec files as part of this task.

## Decisions

### Decision 1: Deterministic mock SSE server over real `scryrs server`

**Choice**: Implement a lightweight Node.js mock HTTP server that speaks the scryrs SSE protocol at `GET /v1/repositories/{repo}/signals?after=N`. It emits replay signals back-to-back within <200ms, falls silent for ≥500ms (comfortably above `REPLAY_SETTLE_MS=350ms`), then emits one live signal. It supports scripted SSE disconnect for reconnect testing.

**Rationale**: The quiescence heuristic (`REPLAY_SETTLE_MS=350ms`) and reconnect duplicate-suppression assertion require precise control over stream timing and disconnect behavior. A real `scryrs server` cannot be scripted to "disconnect after exactly one live signal" or "force a reconnect with `after=N` and verify no DOM duplicates." A mock server running inside the same container as Playwright provides full deterministic control.

**Evidence**: `crates/scryrs-dashboard/tests/api.rs` already uses a `mock_live_signals` pattern (lines 127-157) with a 200ms inter-signal gap. This extends the same idea to a standalone HTTP+SSE server for browser-level verification.

**Trade-off**: The mock does not prove the real `scryrs server` binary's SSE behavior — but that is already covered by `scripts/verify-live-hotspots` and `api.rs`. The browser smoke proves the *rendered dashboard* behavior, which is the gap.

### Decision 2: Stable `data-*` DOM hooks as the assertion surface

**Choice**: Add three `data-*` attributes to the root `Motion` element in `SignalRow.vue`:
- `data-signal-id` — numeric signal id, for row selection and identity assertions
- `data-signal-phase="replay"` or `"live"` — reflects `signal.live` from the store
- `data-motion-path="full"` or `"reduced"` — reflects `!useReducedMotion()` (i.e., whether ignition is allowed)

**Rationale**: `SignalRow.vue` currently has zero test hooks. The `ignites` computed property and `useReducedMotion()` result are computed in `<script setup>` but not projected into the DOM. Adding these attributes is a minimal, non-breaking change (three attribute bindings) that creates a stable, intentional assertion surface. This mirrors the existing store unit test pattern (`signals.test.ts` uses mock `EventSource`) but elevates it to the rendered level.

**Evidence**: `SignalRow.vue` root `Motion` element has no `data-*` attributes. The `ignites` computed (line 10) and `reduced` ref (line 8) are available but not projected.

### Decision 3: Playwright route interception for reconnect testing

**Choice**: Force the disconnect/reconnect cycle via Playwright `page.route()` interception of the SSE endpoint, rather than stopping/restarting the mock server process.

**Rationale**: Route interception is deterministic — it does not depend on process lifecycle management, port re-binding, or timing uncertainty. The frontend's native `EventSource.onerror` handler fires when the intercepted SSE stream is cut, triggering the store's `connect(lastSeenId)` reconnection logic with the `?after=<lastSeenId>` cursor. This proves the full reconnect path at the rendered DOM level.

**Trade-off**: Route interception tests a mocked network boundary rather than a real connection drop. However, the real server transport layer is already proven by `api.rs` and `scripts/verify-live-hotspots`. The browser smoke's job is to prove the frontend *handles* a disconnect correctly — route interception does that deterministically.

### Decision 4: Playwright Docker image (`mcr.microsoft.com/playwright`)

**Choice**: Use `mcr.microsoft.com/playwright` as the Docker image for the verification lane rather than installing Playwright atop the existing `node:22` image at runtime.

**Rationale**: The prebuilt image includes Chromium and all system dependencies (libnss3, libatk-bridge, libcups, etc.). It is faster, more reproducible, and avoids version drift between manual install and CI. The lane documents the image pinning, matching the existing pattern where `scripts/verify-live-hotspots` already overrides `scripts/.versions` default.

**Evidence**: `scripts/.versions` defaults to `node:22-alpine` (musl, incompatible with Playwright). `scripts/verify-live-hotspots` already overrides to `node:22` for glibc compatibility. The browser smoke similarly needs its own image override.

### Decision 5: Opt-in manual lane, excluded from production suite

**Choice**: The new lane (`scripts/verify-live-dashboard-smoke`) is a standalone, manual/opt-in verification lane. It is NOT added to `scripts/verify-production-suite`.

**Rationale**: Unanimous across all three refinement agents. Browser automation in Docker/DinD has inherent flake risk (shared-memory sizing, timing sensitivity, CI resource starvation). The project's stated boundary in `production-suite.md` and `scripts/verification/README.md` explicitly excludes live dashboard browser automation. This lane ships a runnable, documented opt-in path that can gather flake data before any promotion discussion.

**Evidence**: `.devagent/docs/docs/production-suite.md` line ~14: "Live dashboard browser automation stays out of the automated production gate." The production suite currently lists 9 lanes without browser coverage. Adding a browser lane would raise the image requirement from Node to ~1.5GB+ Chromium-capable.

## Risks / Trade-offs

- **`REPLAY_SETTLE_MS` timing sensitivity**: Under Docker-in-Docker or resource-constrained CI, timing can stretch. The mock must emit replay signals with tight inter-signal gaps (<100ms) and a ≥500ms quiet window before the live signal, giving generous margin above the 350ms heuristic. Document this margin in the fixture so maintainers know the timing budget.
- **Playwright `/dev/shm` sizing**: Chromium can crash or hang on page load without sufficient shared memory. The lane must document `--shm-size=256m` or use `--disable-dev-shm-usage` in Playwright launch args.
- **Reconnect assertion is the highest-risk path**: Route interception is reliable, but the mock must produce known signals both before and after the interruption so the test can prove "same signal id, zero visible duplicates." The fixture must predefine a fixed set of signal ids.
- **Reduced-motion assertion depends on the new `data-motion-path` attribute**: Without it, the test can only assert absence of animation artifacts (negative assertion), which is fragile. The `data-motion-path` hook makes it a positive assertion.
- **Dashboard binary compilation**: The smoke needs the dashboard built with embedded frontend assets (production build). The lane must either build from source in Rust Docker (consistent with `scripts/verify-live-hotspots`) or use a prebuilt binary. Building from source is preferred for consistency but adds lane runtime.

## Traceability

All decisions, goals, and non-goals are grounded in the refinement evidence:

| Claim | Source |
|---|---|
| Browser lane gap exists | `scripts/verification/README.md` "Live dashboard browser verification is still manual" |
| Store unit tests prove state, not rendered DOM | `signals.test.ts` covers cursor reuse, dedupe, replay/live marking |
| `SignalRow.vue` has zero test hooks | `SignalRow.vue` grep confirms no `data-testid`/`data-*` attributes |
| `REPLAY_SETTLE_MS=350ms` is the quiescence heuristic | `stores/signals.ts` line defining `REPLAY_SETTLE_MS` |
| `useReducedMotion()` controls ignition | `SignalRow.vue` line 10: `const ignites = computed(() => props.signal.live && reduced.value !== true)` |
| `motion-v` `MotionConfig` centralizes reduced motion | `App.vue`: `<MotionConfig reducedMotion="user">` |
| Backend proxy test pattern exists | `api.rs` `mock_live_signals` (lines 127-157) |
| Docker verification pattern exists | `scripts/verify-live-hotspots` builds binary + runs Node fixture in Docker |
| Production suite excludes browser automation | `.devagent/docs/docs/production-suite.md` line ~14 |
| Default `node:22-alpine` incompatible with Playwright | `scripts/.versions` default image |
| Mock SSE server approach favored 2-of-3 agents | Architect (round 1), Reviewer (round 1) — mock; Lead Dev (round 1) — real server |
| DOM hook naming from Reviewer decision | Reviewer round 1: `data-signal-phase`, `data-motion-path` |
| Route interception for reconnect from Reviewer | Reviewer round 1: "Playwright route interception or mock-server restart" |
| Lane excluded from production suite — unanimous | Architect round 1, Lead Dev round 1, Reviewer round 1 |