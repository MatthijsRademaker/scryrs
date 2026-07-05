## 1. DOM test hooks on SignalRow

- [ ] 1.1 Add `data-signal-id` attribute to the root `Motion` element in `SignalRow.vue`, bound to `props.signal.id`
- [ ] 1.2 Add `data-signal-phase` attribute bound to `props.signal.live ? 'live' : 'replay'`
- [ ] 1.3 Add `data-motion-path` attribute bound to `reduced.value === true ? 'reduced' : 'full'`
- [ ] 1.4 Verify the dashboard frontend typechecks, unit tests pass, and the production build succeeds with the new attributes

## 2. Mock SSE server fixture

- [ ] 2.1 Create `scripts/verification/lib/mock-live-server.mjs` — a lightweight Node.js HTTP server
- [ ] 2.2 Implement `GET /v1/repositories/:repo/signals?after=N` endpoint speaking the scryrs SSE protocol
- [ ] 2.3 Emit replay signals (id 1-N) back-to-back with <100ms inter-signal gaps
- [ ] 2.4 Fall silent for ≥500ms (settle window) before switching to "live" phase
- [ ] 2.5 Emit one live signal (id N+1) after the settle window
- [ ] 2.6 Support scripted SSE disconnect: expose a control endpoint or signal-based trigger that terminates the SSE stream to induce reconnect
- [ ] 2.7 Post-disconnect, when the client reconnects with `?after=<lastSeenId>`, replay only signals with id > cursor, then continue live
- [ ] 2.8 Stub `GET /v1/repositories/:repo/hotspots` (needed for dashboard meta) returning a minimal valid response

## 3. Playwright browser smoke

- [ ] 3.1 Create `scripts/verification/live-dashboard-smoke.spec.ts` — a Playwright test file
- [ ] 3.2 Test: start mock SSE server, start `scryrs dashboard --no-open --server-url <mock> --repository-id repo-a`, navigate to `/signals`
- [ ] 3.3 Wait for replay rows to render, then assert all have `data-signal-phase="replay"`
- [ ] 3.4 Wait for the live signal, assert exactly one row has `data-signal-phase="live"`
- [ ] 3.5 Force disconnect via `page.route()` interception, wait for frontend reconnect, assert no duplicate signal ids appear in the rendered DOM
- [ ] 3.6 Re-run with `page.emulateMedia({ reducedMotion: 'reduce' })`, wait for live signal, assert `data-motion-path="reduced"` on the live row
- [ ] 3.7 Assert that replay rows in the reduced-motion run also have `data-motion-path="reduced"` (no inconsistency)
- [ ] 3.8 Tear down: stop dashboard and mock server, verify clean exit

## 4. Verification lane entrypoint

- [ ] 4.1 Create `scripts/verify-live-dashboard-smoke` — Docker-backed orchestration script
- [ ] 4.2 Build `scryrs` binary (with embedded frontend) in Rust Docker container (follow `scripts/verify-live-hotspots` pattern)
- [ ] 4.3 Copy binary to `.docker-fixtures/scryrs`
- [ ] 4.4 Run Playwright test in `mcr.microsoft.com/playwright` container, mounting the binary and fixture scripts
- [ ] 4.5 Pass `--shm-size=256m` to the Playwright container to prevent Chromium crashes
- [ ] 4.6 Make the script executable (`chmod +x`)
- [ ] 4.7 Exit non-zero on Playwright test failure, with clear lane header output

## 5. Documentation

- [ ] 5.1 Update `scripts/verification/README.md`: add "Live dashboard browser smoke" section documenting `scripts/verify-live-dashboard-smoke`
- [ ] 5.2 Document lane prerequisites: Docker/DinD, `mcr.microsoft.com/playwright` image, `--shm-size=256m`
- [ ] 5.3 Document lane posture: opt-in manual, NOT in `scripts/verify-production-suite`
- [ ] 5.4 Update the "Live dashboard manual smoke boundary" section to reference the new automated lane as the preferred path
- [ ] 5.5 Document the `data-*` attribute contract in the lane README so maintainers understand the stable assertion surface

## 6. Verification (self-check)

- [ ] 6.1 Run `scripts/verify-live-dashboard-smoke` end-to-end in the local Docker/DinD environment
- [ ] 6.2 Confirm replay rows render with `data-signal-phase="replay"`
- [ ] 6.3 Confirm live row renders with `data-signal-phase="live"`
- [ ] 6.4 Confirm reconnect produces zero duplicate rows
- [ ] 6.5 Confirm reduced-motion path produces `data-motion-path="reduced"`
- [ ] 6.6 Confirm frontend typecheck, unit tests, and production build remain green with the new `data-*` attributes