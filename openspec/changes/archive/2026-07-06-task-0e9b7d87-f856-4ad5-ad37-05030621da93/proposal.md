## Why

Route explanations already exist as deterministic Rust/runtime logic over `.scryrs/routes.json`, exposed through `scryrs route explain --query <TEXT>`. Users can inspect hotspots, sessions, events, and live signals in the dashboard browser, yet must drop back to the CLI to see what future agents would load and why. This change exposes that same read-only explanation path inside the dashboard as a local-mode Route view.

## What Changes

- **Add `scryrs-runtime` as a dashboard dependency** — the dashboard backend reuses `scryrs_runtime::explain_hints` rather than duplicating deterministic ranking logic in TypeScript or the dashboard server.
- **Add `GET /api/routes/explain?query=<text>` endpoint** — a read-only, deterministic backend endpoint that loads `.scryrs/routes.json`, validates its schema version, and delegates matching to the existing Rust runtime. Returns a `RouteHintDocument` with relevance-populated hints, evidence citations, and query-match provenance in each hint's reason field.
- **Add Route navigation entry and view** — a new `/routes` route with a `RouteView.vue` component, registered in the Vue router and the local-mode navigation sidebar. The nav entry appears in local mode even when `.scryrs/routes.json` is missing, because the view provides remediation guidance.
- **Add typed frontend DTOs and API client** — hand-written TypeScript types mirroring `RouteHintDocument`/`RouteHintItem`, plus a `getRouteHints(query)` client function calling `GET /api/routes/explain`.
- **Add Pinia route hints store** — reactive loading/error/data state management for route explain queries.
- **Handle error and empty states** — missing artifact (404 with remediation to run `scryrs route <PATH>`), malformed/schema-mismatch artifact (502), live-mode unavailability (404 with descriptive message), zero-match results (non-error empty state), and empty-query rejection (400 backend, plus frontend gating).
- **Update CLI help surfaces** — `crates/scryrs-cli/src/dashboard.rs`, `help_json.rs`, `help_text.rs`, and their insta snapshots must document the new `GET /api/routes/explain` endpoint to prevent public API surface drift.
- **Add automated tests** — Rust API contract tests (success, missing-artifact, malformed-artifact, live-unavailable, empty-query rejection) in `crates/scryrs-dashboard/tests/api.rs`, plus TypeScript store/client tests for query building, error propagation, and nav gating.

## Impact

- **Affected specs**: New `dashboard-route-explain` spec; no existing spec modifications required.
- **Affected code**: `crates/scryrs-dashboard/src/server.rs` (new handler), `crates/scryrs-dashboard/Cargo.toml` (add `scryrs-runtime` dep), `crates/scryrs-dashboard/tests/api.rs` (new tests), `crates/scryrs-dashboard/frontend/src/router/index.ts` (new route), `crates/scryrs-dashboard/frontend/src/shared/lib/dashboard-mode.ts` (nav entry + live-mode gate), `crates/scryrs-dashboard/frontend/src/shared/api/client.ts` (new DTOs and fetcher), `crates/scryrs-dashboard/frontend/src/stores/routes.ts` (new store), `crates/scryrs-dashboard/frontend/src/views/RouteView.vue` (new view), `crates/scryrs-cli/src/dashboard.rs` (endpoint list), `crates/scryrs-cli/src/help_json.rs` (endpoint list), `crates/scryrs-cli/src/help_text.rs` (endpoint list).
- **No breaking changes**: Existing endpoints, views, navigation, and mode gating are unchanged. Live mode users see no new navigation items and no behavioral change.