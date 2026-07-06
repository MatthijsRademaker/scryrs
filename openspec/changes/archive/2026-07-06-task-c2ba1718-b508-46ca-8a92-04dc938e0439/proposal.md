## Why

The dashboard live Signals view (replay/live classification, reconnect duplicate suppression, reduced-motion path) has zero browser-level verification. The repo proves correctness only at the store unit-test level (`signals.test.ts`) and backend API proxy level (`api.rs`). Project documentation explicitly states "Live dashboard browser verification is still manual in this change" (`scripts/verification/README.md`). This means replay-vs-live rendering, reconnect duplicate suppression via `?after=<lastSeenId>`, and the `useReducedMotion()` rendering path can regress silently even though the underlying SSE contract passes.

This task adds a deterministic, headless, Docker-backed browser smoke for the live Signals view — keeping it outside the production suite initially so flake data can accumulate before any promotion decision.

## What Changes

- **New verification lane**: `scripts/verify-live-dashboard-smoke` — a standalone, Docker-backed, Playwright-on-Chromium browser smoke for the live Signals view.
- **Deterministic mock SSE upstream**: A lightweight Node.js mock HTTP server that speaks the scryrs `/v1/repositories/{repo}/signals` SSE protocol, emitting replay signals back-to-back, then a quiet window, then one live signal, with scripted disconnect/reconnect support.
- **Stable DOM test hooks on `SignalRow.vue`**: Add `data-signal-id`, `data-signal-phase="replay|live"`, and `data-motion-path="full|reduced"` attributes — the intentional, stable assertion surface for replay/live distinction and reduced-motion verification.
- **Reconnect duplicate suppression assertion**: The smoke forces a disconnect via Playwright route interception, lets the frontend reconnect with `?after=<lastSeenId>`, and verifies no duplicate rows appear.
- **Reduced-motion path assertion**: The smoke re-runs with `page.emulateMedia({ reducedMotion: 'reduce' })` and verifies `data-motion-path="reduced"` on a live signal row.
- **Documentation update**: `scripts/verification/README.md` documents the new lane, its prerequisites (Playwright-capable Docker image), its opt-in posture, and the fact that it is NOT part of `scripts/verify-production-suite`.
- **Docker image selection**: The lane uses `mcr.microsoft.com/playwright` (Chromium-capable) rather than the current `node:22-alpine` or `node:22`, following the existing pattern where `scripts/verify-live-hotspots` already overrides the default image.

## Impact

- **Frontend production code (minimal)**: `SignalRow.vue` gains three `data-*` attributes. No store, API, protocol, or backend changes.
- **New fixture artifacts**: `scripts/verification/live-dashboard-smoke.spec.ts` (Playwright test), `scripts/verification/lib/mock-live-server.mjs` (mock SSE server).
- **New lane entrypoint**: `scripts/verify-live-dashboard-smoke` (orchestration script).
- **Docker**: New Playwright image used by the lane; no change to the default `scripts/.versions` image.
- **Documentation**: `scripts/verification/README.md` updated.
- **Specs**: New `dashboard-browser-verification` capability added.
- **Out of scope**: No change to `scripts/verify-production-suite`, no CI wiring, no visual snapshot tests, no animated-pixel assertions, no protocol or store logic changes.