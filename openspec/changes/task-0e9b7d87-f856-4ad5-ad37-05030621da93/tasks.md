## 1. Backend — Add `scryrs-runtime` dependency

- [ ] 1.1 Add `scryrs-runtime = { workspace = true }` to `crates/scryrs-dashboard/Cargo.toml` dependencies.
- [ ] 1.2 Verify `cargo check -p scryrs-dashboard` passes with the new dependency.

## 2. Backend — Implement `GET /api/routes/explain`

- [ ] 2.1 Write a manifest-loading helper in `crates/scryrs-dashboard/src/server.rs` (or a new `src/route_explain.rs` module) that: reads `.scryrs/routes.json`, parses JSON as `RouteManifestDocument`, validates `schemaVersion == ROUTE_SCHEMA_VERSION`, and returns the document or maps to `ApiError::missing()` (404) / `ApiError::bad_gateway()` (502). Document deliberate duplication over CLI `route_common.rs`.
- [ ] 2.2 Add a `RouteExplainQuery` struct with `query: String` for deserializing the `?query=` parameter.
- [ ] 2.3 Implement the `GET /api/routes/explain` handler: reject empty `?query=` with HTTP 400 ("query parameter must be non-empty"), reject live mode with 404 ("route explain unavailable in live mode"), load and validate manifest, call `scryrs_runtime::explain_hints(&manifest, &query)`, return `RouteHintDocument` as JSON.
- [ ] 2.4 Register `.route("/api/routes/explain", get(route_explain))` in the `router()` function.

## 3. Backend — Add API contract tests

- [ ] 3.1 Test `GET /api/routes/explain?query=auth` returns 200 with valid `RouteHintDocument` when `.scryrs/routes.json` fixture is present.
- [ ] 3.2 Test missing `.scryrs/routes.json` returns 404 with remediation message.
- [ ] 3.3 Test malformed JSON in `.scryrs/routes.json` returns 502.
- [ ] 3.4 Test schema version mismatch returns 502.
- [ ] 3.5 Test empty `?query=` returns 400 with descriptive error.
- [ ] 3.6 Test missing `?query` parameter returns 400.
- [ ] 3.7 Test live mode returns 404 ("route explain unavailable in live mode").
- [ ] 3.8 Test zero-match query returns 200 with empty hints array.

## 4. CLI — Update help surfaces

- [ ] 4.1 Add `GET /api/routes/explain (local mode only)` to REST API list in `crates/scryrs-cli/src/dashboard.rs` `write_dashboard_help()`.
- [ ] 4.2 Add `GET /api/routes/explain` to dashboard command output description in `crates/scryrs-cli/src/help_json.rs`.
- [ ] 4.3 Update `crates/scryrs-cli/src/help_text.rs` if it separately enumerates dashboard endpoints.
- [ ] 4.4 Regenerate insta snapshots (`cargo test -p scryrs-cli`) and accept updated help surface output.

## 5. Frontend — Add typed API DTOs and client

- [ ] 5.1 Add `RouteHintDocument`, `RouteHintItem`, `EvidenceLink`, `RouteLoadTarget` TypeScript interfaces to `crates/scryrs-dashboard/frontend/src/shared/api/client.ts`, matching the Rust `RouteHintDocument` camelCase wire contract.
- [ ] 5.2 Add `getRouteHints(query: string): Promise<RouteHintDocument>` client function using existing `fetchJson` pattern.

## 6. Frontend — Add route hints Pinia store

- [ ] 6.1 Create `crates/scryrs-dashboard/frontend/src/stores/routes.ts` with `useRouteStore` using `defineStore`, exposing reactive `hints`, `loading`, `error`, `query`, and async `search(query)` action.
- [ ] 6.2 Gate `search()` to skip API call when `query` is empty (frontend defense-in-depth).

## 7. Frontend — Add Route view

- [ ] 7.1 Create `crates/scryrs-dashboard/frontend/src/views/RouteView.vue` with: search input, loading/error/empty/results states, live-mode unavailable card, missing-artifact remediation guidance.
- [ ] 7.2 Render successful results as a table/list showing `rank`, `label`, `target`, `loadTarget.kind` + `reference`, `relevance` (when present), `reason`, and expanded evidence citations (plain-text `sourceKind` + `subject` + `rowIds`).
- [ ] 7.3 Show remediation guidance for missing artifact: "Route artifact not found. Run `scryrs route <PATH>` to generate the route manifest."
- [ ] 7.4 Show distinct error state for malformed/schema-mismatch artifact: "Route artifact is malformed or from an incompatible version. Run `scryrs route <PATH>` to regenerate."
- [ ] 7.5 Show "Unavailable in live mode" card via `routeUnavailableMessage()` per Sessions/Events pattern.

## 8. Frontend — Register route and navigation

- [ ] 8.1 Add `/routes` route with `name: "routes"` and lazy-loaded `RouteView.vue` in `crates/scryrs-dashboard/frontend/src/router/index.ts`.
- [ ] 8.2 Add Route nav item to `LOCAL_NAV` in `crates/scryrs-dashboard/frontend/src/shared/lib/dashboard-mode.ts`.
- [ ] 8.3 Add `"routes"` case to `routeUnavailableMessage()` in `dashboard-mode.ts` for live-mode direct URL access.

## 9. Frontend — Add tests

- [ ] 9.1 Create `crates/scryrs-dashboard/frontend/src/stores/routes.test.ts`: test store search dispatches API call, loading→data transition, error propagation, empty-query skip, non-empty query triggers fetch.
- [ ] 9.2 Add client-level test or extend store tests to verify query encoding and error handling from `getRouteHints`.
- [ ] 9.3 Add navigation/mode-gating test: Route nav item appears in `navigationForMode("local")`, absent in `navigationForMode("live")`.

## 10. Integration verification

- [ ] 10.1 Run `cargo test -p scryrs-dashboard` — all new and existing tests pass.
- [ ] 10.2 Run `cargo test -p scryrs-cli` — CLI help snapshots updated and passing.
- [ ] 10.3 Run `bun test` in `crates/scryrs-dashboard/frontend/` — all frontend tests pass.
- [ ] 10.4 Run `bun run check` in `crates/scryrs-dashboard/frontend/` — no TypeScript errors.