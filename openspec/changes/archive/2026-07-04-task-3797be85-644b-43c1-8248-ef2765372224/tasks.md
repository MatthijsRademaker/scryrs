## 1. Define the live dashboard mode contract

- [ ] 1.1 Publish `openspec/specs/live-dashboard-mode/spec.md` with requirements for mode activation, config namespace, backend proxy, SSE signals, frontend adaptation, shape normalization, error handling, and verification.

## 2. Design config resolution for live mode

- [ ] 2.1 Define `dashboard.server_url` and `dashboard.repository_id` fields in `scryrs.json` manifest schema.
- [ ] 2.2 Define `SCRYRS_DASHBOARD_SERVER_URL` and `SCRYRS_DASHBOARD_REPOSITORY_ID` environment variable contract.
- [ ] 2.3 Define `--server-url` and `--repository-id` CLI flags on `scryrs dashboard` with documented help text.
- [ ] 2.4 Document config precedence (CLI > env > manifest) and that absence of both fields means local mode.

## 3. Design backend source-mode abstraction

- [ ] 3.1 Define `SourceMode` enum (`Local` | `Live { server_url, repository_id }`) in dashboard config.
- [ ] 3.2 Design route dispatch in `server.rs` that selects file-backed or HTTP-proxied handlers based on mode.
- [ ] 3.3 Design `/api/meta` expansion to return `mode`, `repositoryId`, and `repositoryPath` (CWD-derived).

## 4. Design backend live API proxy and shape normalization

- [ ] 4.1 Design `/api/hotspots` proxy to `GET /v1/repositories/{id}/hotspots?window=cumulative` with `LiveHotspotsResponse`-to-`HotspotsReport` envelope normalization.
- [ ] 4.2 Define 502 response for upstream server unavailable and 200-with-empty for empty repository responses.
- [ ] 4.3 Design `/api/sessions`, `/api/sessions/:id`, and `/api/events` to return 404 with "unavailable in live mode" error body in live mode.

## 5. Design backend SSE proxy

- [ ] 5.1 Design `GET /api/signals` SSE endpoint that proxies `GET /v1/repositories/{id}/signals?after=<cursor>` with per-client upstream connections.
- [ ] 5.2 Define in-memory cursor state per connection with reset to `after=0` on backend restart.
- [ ] 5.3 Define 502 SSE client notification on upstream disconnection without silent retry.

## 6. Design frontend live-mode adaptation

- [ ] 6.1 Design mode-aware `client.ts` with `EventSource` for `/api/signals` and mode-conditional fetch behavior.
- [ ] 6.2 Design `DashboardShell.vue` conditional navigation: hide Sessions/Events in live mode, add Signals route.
- [ ] 6.3 Design Signals view with SSE-based signal timeline display.
- [ ] 6.4 Design unavailable stub view for direct navigation to Sessions/Events URLs in live mode.
- [ ] 6.5 Design footer copy update to reflect active mode (local vs live).
- [ ] 6.6 Design About view update to document live mode capabilities and known differences (raw subject paths).

## 7. Define verification plan

- [ ] 7.1 Design dashboard live-mode smoke step: start `scryrs server`, start `scryrs dashboard --server-url <URL> --repository-id <ID>`, verify `/api/meta` returns `mode: live`, verify `/api/hotspots` returns live entries, verify SSE signals via `/api/signals`.
- [ ] 7.2 Design local-mode regression check: existing dashboard E2E tests pass unchanged without live configuration.
- [ ] 7.3 Document the verification suite extension in `scripts/verification/README.md`.

## 8. Document the live dashboard mode

- [ ] 8.1 Update `.devagent/docs/docs/cli-v0-contract.md` to document live dashboard mode activation and behavior.
- [ ] 8.2 Update `.devagent/docs/docs/live-hotspots.md` to add dashboard live-mode section with configuration and proxy architecture.
- [ ] 8.3 Update `.devagent/docs/docs/roadmap.mdx` to mark live dashboard contract as defined.
- [ ] 8.4 Update `AboutView.vue` copy to reflect live mode capabilities.