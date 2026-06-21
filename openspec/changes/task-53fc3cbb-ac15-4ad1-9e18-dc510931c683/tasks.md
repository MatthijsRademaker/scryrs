## 1. Roadmap and Documentation Updates

- [ ] 1.1 Insert Phase 3 (Dashboard) into `.devagent/docs/docs/roadmap.mdx` between Phase 2 and current Phase 3, with defined deliverables and deferred items
- [ ] 1.2 Renumber downstream phases (4–8) to reflect the new insertion
- [ ] 1.3 Reconcile scope guardrail #5: remove or rewrite to differentiate local dashboard (planned) from hosted analytics (deferred)
- [ ] 1.4 Update the "Suggested Near-Term Milestones" table to include a dashboard milestone (e.g., "M3 - Local dashboard beta")
- [ ] 1.5 Update `.devagent/docs/docs/cli-v0-contract.md` to document the `scryrs dashboard` command surface, flags, and behavior

## 2. New Dashboard Crate Scaffolding

- [ ] 2.1 Create `crates/scryrs-dashboard/` directory with `Cargo.toml` declaring the crate name, version, and dependencies (`axum`, `tokio`, `tower-http`, `rust-embed`, `serde`, `serde_json`, `rusqlite`, `mime_guess`)
- [ ] 2.2 Register `scryrs-dashboard` as a workspace member in the root `Cargo.toml`
- [ ] 2.3 Create `crates/scryrs-dashboard/src/lib.rs` with the public `run(Config)` function and `Config` struct with fields for `port`, `bind_address`, `no_open`, `dev_mode`, and `repo_root`
- [ ] 2.4 Create `crates/scryrs-dashboard/src/server.rs` with the axum router setup, shared state (repo root path), and route registration

## 3. CLI Integration

- [ ] 3.1 Add `scryrs-dashboard` as a dependency of `scryrs-cli` in `crates/scryrs-cli/Cargo.toml`
- [ ] 3.2 Add `dashboard` arm to the argument parser in `crates/scryrs-cli/src/lib.rs` that parses `--port`, `--bind`, `--no-open`, `--dev` flags and delegates to `scryrs_dashboard::run()`
- [ ] 3.3 Update help text in `crates/scryrs-cli/src/lib.rs` to list `dashboard` in the commands section
- [ ] 3.4 Update the `--help-json` surface document in `crates/scryrs-cli/src/lib.rs` to include the `dashboard` command with its flags, types, and defaults
- [ ] 3.5 Add `scryrs dashboard --help` custom help output that describes all flags

## 4. HTTP Server and REST API

- [ ] 4.1 Implement the `GET /api/hotspots` endpoint that reads and returns `.scryrs/hotspots.json`
- [ ] 4.2 Implement the `GET /api/sessions` endpoint that queries `.scryrs/scryrs.db` for session metadata, ordered by start time descending with a configurable limit
- [ ] 4.3 Implement the `GET /api/events` endpoint with cursor-based pagination (query params: `limit`, `cursor`), returning events from `.scryrs/scryrs.db`
- [ ] 4.4 Implement static file serving: `GET /` → embedded `index.html`, `GET /assets/*path` → embedded asset files
- [ ] 4.5 Implement fallback routing: all non-API, non-asset paths serve `index.html` (for Vue Router push-state)
- [ ] 4.6 Implement error handling for missing `.scryrs/hotspots.json` (404), missing `.scryrs/scryrs.db` (404), and corrupt/unreadable SQLite store (502)
- [ ] 4.7 Implement startup behavior: determine repo root from current working directory (mirroring `hotspots` behavior), start server, print startup message to stderr, optionally open browser via `webbrowser` crate or `open` command

## 5. Vue.js Frontend Application

- [ ] 5.1 Scaffold the Vue 3 + Vite project under `crates/scryrs-dashboard/frontend/` with Vue Router, Pinia, and Chart.js (via vue-chartjs) dependencies
- [ ] 5.2 Create the shared layout: sidebar/navigation component with links to Hotspots, Sessions, Events, and About views
- [ ] 5.3 Create the API client module (`src/api/client.ts`) with typed fetch functions for `GET /api/hotspots`, `GET /api/sessions`, `GET /api/sessions/:id`, `GET /api/events`
- [ ] 5.4 Create the hotspot report view (`src/views/HotspotsView.vue`) with a sortable table of hotspot entries, handling empty and error states
- [ ] 5.5 Create the subject detail view (`src/views/SubjectDetailView.vue`) with per-event-type breakdown chart and session timeline
- [ ] 5.6 Create the sessions list view (`src/views/SessionsView.vue`) with recent sessions, navigation to session detail
- [ ] 5.7 Create the session detail view (`src/views/SessionDetailView.vue`) with scrollable event list and payload preview
- [ ] 5.8 Create the event distribution view (`src/views/EventsView.vue`) with a chart of events grouped by type, filterable by session
- [ ] 5.9 Create the about view (`src/views/AboutView.vue`) showing dashboard version, docs link, and data source info
- [ ] 5.10 Create the 404 not-found view (`src/views/NotFoundView.vue`) with a link back to the landing page
- [ ] 5.11 Create shared reusable components: `DataTable.vue` (sortable, column-configurable), `ChartCard.vue` (title, chart, optional filter), `StatusBadge.vue` (for event types and session states), `EmptyState.vue` (icon + message + action button)
- [ ] 5.12 Register all routes in the Vue Router configuration with lazy-loaded components
- [ ] 5.13 Implement Pinia stores: `useHotspotStore` (hotspot data + loading/error state), `useSessionStore` (session list + detail), `useEventStore` (events + distribution)
- [ ] 5.14 Style the application with custom CSS using design tokens (CSS custom properties for colors, spacing, typography) — no external UI component library

## 6. Build Integration and Embedding

- [ ] 6.1 Create a Rust build script (`crates/scryrs-dashboard/build.rs`) that runs `npm ci && npm run build` in the `frontend/` directory when the `dist/` output is missing or the frontend source has changed
- [ ] 6.2 Integrate `rust-embed` into the server code so built SPA assets are compiled into the binary at compile time
- [ ] 6.3 Ensure the `--dev` flag bypasses embedded assets and serves from the filesystem `frontend/dist/` directly
- [ ] 6.4 Add a CI step (or ensure the Makefile/scripts/check includes) that builds the frontend before `cargo build` for the dashboard crate

## 7. Testing

- [ ] 7.1 Write Rust unit tests for `Config` parsing defaults and validation
- [ ] 7.2 Write Rust integration tests for API endpoints using `axum::test` harness: test hotspot response shape, session query pagination, event cursor behavior, 404/502 error cases
- [ ] 7.3 Write end-to-end test that runs `scryrs dashboard` against a fixture `.scryrs/` directory with known hotspot/session data, starts the server, fetches from the API, and asserts response shapes
- [ ] 7.4 Write a smoke test verifying `scryrs dashboard --help` exits 0 and contains expected text
- [ ] 7.5 Write a smoke test verifying `scryrs --help-json` includes a `dashboard` command entry
