## 1. Roadmap and Documentation Updates

- [x] 1.1 Insert Phase 3 (Dashboard) into `.devagent/docs/docs/roadmap.mdx` between Phase 2 and current Phase 3, with defined deliverables and deferred items
- [x] 1.2 Renumber downstream phases (4–8) to reflect new insertion
- [x] 1.3 Reconcile scope guardrail #5: drop or rewrite to distinguish local dashboard (planned) from hosted analytics (deferred)
- [x] 1.4 Update suggested near-term milestones table to include dashboard milestone
- [x] 1.5 Update `.devagent/docs/docs/cli-v0-contract.md` to document `scryrs dashboard` command surface, flags, and behavior

## 2. New Dashboard Crate Scaffolding

- [x] 2.1 Create `crates/scryrs-dashboard/` with `Cargo.toml` declaring crate name, version, and dependencies (`axum`, `tokio`, `tower-http`, `rust-embed`, `serde`, `serde_json`, `rusqlite`, `mime_guess`)
- [x] 2.2 Register `scryrs-dashboard` as workspace member in root `Cargo.toml`
- [x] 2.3 Create `crates/scryrs-dashboard/src/lib.rs` with public `run(Config)` function and `Config` struct with `port`, `bind_address`, `no_open`, `dev_mode`, and `repo_root`
- [x] 2.4 Create `crates/scryrs-dashboard/src/server.rs` with axum router setup, shared state, and route registration

## 3. CLI Integration

- [x] 3.1 Add `scryrs-dashboard` as dependency of `scryrs-cli` in `crates/scryrs-cli/Cargo.toml`
- [x] 3.2 Add `dashboard` arm to parser in `crates/scryrs-cli/src/lib.rs` that parses `--port`, `--bind`, `--no-open`, `--dev` and delegates to `scryrs_dashboard::run()`
- [x] 3.3 Update help text in `crates/scryrs-cli/src/lib.rs` to list `dashboard` in commands section
- [x] 3.4 Update `--help-json` surface document in `crates/scryrs-cli/src/lib.rs` to include `dashboard` command with flags, types, and defaults
- [x] 3.5 Add `scryrs dashboard --help` custom help output that describes all flags

## 4. HTTP Server and REST API

- [x] 4.1 Implement `GET /api/hotspots` endpoint that reads and returns `.scryrs/hotspots.json`
- [x] 4.2 Implement `GET /api/sessions` endpoint that queries `.scryrs/scryrs.db` for session metadata, ordered by start time descending with configurable limit
- [x] 4.3 Implement `GET /api/events` endpoint with cursor-based pagination (`limit`, `cursor`) returning events from `.scryrs/scryrs.db`
- [x] 4.4 Implement static file serving: `GET /` → embedded `index.html`, `GET /assets/*path` → embedded asset files
- [x] 4.5 Implement fallback routing: all non-API, non-asset paths serve `index.html`
- [x] 4.6 Implement error handling for missing `.scryrs/hotspots.json` (404), missing `.scryrs/scryrs.db` (404), and corrupt or unreadable SQLite store (502)
- [x] 4.7 Implement startup behavior: determine repo root from current working directory, start server, print startup message to stderr, optionally open browser

## 5. Vue.js Frontend Application

- [x] 5.1 Scaffold dashboard frontend under `crates/scryrs-dashboard/frontend/` with Vue 3, Vite, strict TypeScript, Bun, Tailwind CSS v4, shadcn-vue, Reka UI, lucide-vue, Vue Router, and Pinia
- [x] 5.2 Create shared layout shell with shadcn-vue sidebar/navigation components linking to Hotspots, Sessions, Events, and About views
- [x] 5.3 Create hand-written typed API client module (`src/shared/api/client.ts`) with fetch functions for `GET /api/hotspots`, `GET /api/sessions`, `GET /api/sessions/:id`, and `GET /api/events`; do not introduce generated client code
- [x] 5.4 Create hotspot report view (`src/views/HotspotsView.vue`) with sortable table of hotspot entries, shared loading/empty/error states, and semantic token styling
- [x] 5.5 Create subject detail view (`src/views/SubjectDetailView.vue`) with per-event-type breakdown visualization compatible with dashboard design system and session timeline
- [x] 5.6 Create sessions list view (`src/views/SessionsView.vue`) with recent sessions and navigation to session detail
- [x] 5.7 Create session detail view (`src/views/SessionDetailView.vue`) with scrollable event list and payload preview
- [x] 5.8 Create event distribution view (`src/views/EventsView.vue`) with semantic event distribution visualization, session filter, and shared state components
- [x] 5.9 Create about view (`src/views/AboutView.vue`) showing dashboard version, docs link, and data source info
- [x] 5.10 Create 404 not-found view (`src/views/NotFoundView.vue`) with link back to landing page
- [x] 5.11 Import or adapt reusable donor infrastructure from `~/repos/personal/TheGreatMigration/frontend/`: `components.json`, `src/shared/ui/**`, `src/shared/lib/utils.ts`, `src/app/styles.css`, and shared shell patterns; reject donor domain pages, generated client files, images, Dockerfile, and extra lockfiles; drop stale donor labels during import
- [x] 5.12 Register all routes in Vue Router configuration with lazy-loaded components
- [x] 5.13 Implement Pinia stores: `useHotspotStore` (hotspot data + loading/error state), `useSessionStore` (session list + detail), `useEventStore` (events + distribution)
- [x] 5.14 Create and maintain `components.json`, `src/shared/ui/**`, `src/shared/lib/utils.ts`, and `src/app/styles.css`; style dashboard with Tailwind CSS v4 semantic tokens and shadcn-vue component composition

## 6. Build Integration and Embedding

- [x] 6.1 Create Rust build script (`crates/scryrs-dashboard/build.rs`) that runs Bun install and Bun build in `frontend/` when `dist/` output is missing or frontend source has changed, and fails loudly when Bun is required but unavailable
- [x] 6.2 Integrate `rust-embed` into server code so built SPA assets are compiled into binary at compile time
- [x] 6.3 Ensure `--dev` flag bypasses embedded assets and serves from filesystem `frontend/dist/` directly
- [x] 6.4 Add CI step (or ensure Makefile/scripts/check includes) that runs frontend Bun checks and build before `cargo build` for dashboard crate, and commit only `bun.lock` for frontend package

## 7. Testing

- [x] 7.1 Write Rust unit tests for `Config` parsing defaults and validation
- [x] 7.2 Write Rust integration tests for API endpoints using `axum::test` harness: hotspot response shape, session query pagination, event cursor behavior, and 404/502 error cases
- [x] 7.3 Write end-to-end test that runs `scryrs dashboard` against fixture `.scryrs/` directory, starts server, fetches API, and asserts response shapes
- [x] 7.4 Write smoke test verifying `scryrs dashboard --help` exits 0 and contains expected text
- [x] 7.5 Write smoke test verifying `scryrs --help-json` includes `dashboard` command entry
