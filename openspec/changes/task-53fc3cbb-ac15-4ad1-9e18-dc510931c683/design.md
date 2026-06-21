## Context

scryrs has two delivered phases: proxy capture (Phase 1) and hotspot materialization (Phase 2). Users can record traces and produce deterministic hotspot reports, but the output is JSON on stdout and flat files in `.scryrs/`. There is no visual browsing, no session timeline, no way to compare hotspot runs across sessions, no drill-down on individual subjects or events.

The proposal inserts a Dashboard phase between Phase 2 and the current Phase 3 (Graph and Route Manifests, pushed to Phase 4). This design covers the `scryrs dashboard` CLI command, the embedded HTTP server, the Vue.js SPA frontend, and how they compose with the existing crate architecture.

Constraints:

- Zero changes to existing CLI behavior (`record`, `hotspots`, `init`, `--help`, `--help-json`, `--version`).
- No new external dependencies for the existing crates.
- The dashboard crate must be a new workspace member, not woven into `scryrs-cli`.
- The SPA must be embeddable in the binary (Rust `include_dir!` or similar).
- The dashboard must gracefully handle missing or partial data (no `.scryrs/scryrs.db`, no `.scryrs/hotspots.json`, empty stores, unreadable files).

## Goals / Non-Goals

**Goals:**

- Ship `scryrs dashboard [--port <PORT>] [--no-open] [--bind <ADDR>]` that serves a Vue.js SPA and reads `.scryrs/*.json` + SQLite data.
- Provide visual browsing of hotspot report data: table view, per-subject drill-down, session timeline, event distribution chart.
- Build the frontend with component slots so graph/route views can be added in later phases without restructuring.
- Embed the built SPA as static assets in the Rust binary (single-file deploy, no external server needed).
- Serve data via a local REST API that the SPA fetches: `GET /api/hotspots`, `GET /api/sessions`, `GET /api/events`.
- Register `scryrs dashboard` as a subcommand of the existing `scryrs` binary.

**Non-Goals:**

- No hosted or multi-user dashboard — this is local-only.
- No authentication, no write API, no data mutation through the dashboard.
- No real-time or streaming updates — the dashboard reads the current state of artifact files.
- No graph visualization or route exploration — those belong in Phase 4+ when the data exists.
- No CI/CD or publishing pipeline for the dashboard as a standalone service.
- No behavioral changes to existing CLI commands (`record`, `hotspots`, `init`, `--help`, `--help-json`, `--version`) or any existing crate. The `scryrs-cli` crate will gain a `scryrs-dashboard` dependency and delegate the `dashboard` subcommand in the implementation PR.

## Decisions

### Decision 1: New `scryrs-dashboard` crate vs. adding to `scryrs-cli`

**Chosen**: New crate `crates/scryrs-dashboard/`.

The dashboard brings its own dependency tree (HTTP server, asset embedding, possibly a JSON deserialization library for its API types) and its own test surface. Adding it to `scryrs-cli` would couple CLI argument parsing with HTTP serving logic and make the binary larger even when users only call `record` or `hotspots`. A separate crate keeps the dashboard build optional in the workspace and makes the dependency boundary explicit.

`scryrs-cli` adds `scryrs-dashboard` as a dependency and delegates the `dashboard` subcommand to it.

### Decision 2: Embedded SPA via `rust-embed` or `include_dir!`

**Chosen**: `rust-embed` at build time.

The SPA is built with Vite, and the output `dist/` directory is compiled into the binary. `rust-embed` provides compile-time file embedding with path-based lookup and `gzip` compression support. This is the standard pattern in the Rust ecosystem (axum examples, `sqlx` migrations, etc.) with zero runtime dependencies beyond `mime_guess` for Content-Type headers.

The alternative — serving files from a known path on disk — would require the user to keep the SPA build output alongside the binary, which is fragile for a CLI tool distributed as a single binary.

### Decision 3: HTTP server framework

**Chosen**: `axum` on `tokio`.

`axum` is the current ecosystem standard in the Rust async HTTP space — used by the same team behind `tower` and `hyper`. For a local-only server, its ergonomics (extractors, middleware tower, shared state via `Arc`) fit well. The alternative `actix-web` is heavier and brings its own runtime. `warp` is less actively maintained.

The server has exactly three routes:

- `GET /` — serve the SPA's `index.html`
- `GET /assets/*path` — serve static JS/CSS assets
- `GET /api/*` — REST API endpoints backed by SQLite + artifact file reads

### Decision 4: API design — read from artifact files and SQLite

**Chosen**: The API reads `.scryrs/hotspots.json` (for the hotspot report) and opens `.scryrs/scryrs.db` via `rusqlite` (for session/event queries).

This avoids duplicating the scoring logic from `scryrs-core`. The dashboard is a _viewer_ — it does not recompute hotspots. It reuses the artifacts that `scryrs hotspots` produces.

Session and event queries are new but thin: they execute SQL against the existing SQLite schema. No new index or schema changes are needed — the store already has `events` and `sessions` tables with appropriate columns.

### Decision 5: CLI integration — `scryrs dashboard` subcommand

**Chosen**: `scryrs-cli` recognizes `dashboard` as a valid subcommand and delegates to `scryrs_dashboard::run(config)`.

The `--help-json` surface document must be updated to include the `dashboard` command with its flags. The existing help text is extended with a line for `dashboard`.

Flags:

- `--port`, `-p` (default `8080`): TCP port to bind
- `--bind`, `-b` (default `127.0.0.1`): bind address
- `--no-open` (flag): do not open browser automatically
- `--dev` (flag): serve from filesystem instead of embedded assets (for frontend development)

### Decision 6: Vue.js frontend stack

**Chosen**: Vue 3 + Vite + Pinia (state) + Vue Router (SPA routing).

The proposal specifies Vue.js, Vue 3 with the Composition API is the current stable. Vite is the canonical build tool. Pinia provides lightweight store management for hotspot/session/event data loaded from the API. Vue Router enables separate views (Hotspots, Sessions, Subjects, About).

No component library — the dashboard uses custom CSS (utility-first approach with CSS custom properties) to avoid adding 100+ kB of UI framework dependency. Charts are rendered with `chart.js` via `vue-chartjs` (lightweight, no D3 complexity for the initial bar/line/pie charts).

### Decision 7: Async/SQLite bridging strategy

**Chosen**: `tokio::task::spawn_blocking` for `rusqlite` query operations.

`rusqlite` is synchronous — calling it directly from an async axum handler would block the Tokio runtime thread. The dashboard uses `spawn_blocking` to offload SQLite reads (session list, event pagination) onto Tokio's blocking thread pool. This keeps axum handlers async while avoiding an additional SQLite dependency like `sqlx` or `libsql`.

Read operations are short-lived (<100ms for typical stores) and do not hold the connection across multiple requests. The server opens a new `rusqlite::Connection` per query (SQLite handles concurrent reads via file-level locking) rather than maintaining a long-lived connection pool — this is acceptable for a single-user local dashboard where concurrency is at most one browser client.

### Decision 8: Cargo feature-gating for the dashboard crate

**Chosen**: Deferred. The dashboard crate is always compiled as a workspace member.

Gating `scryrs-dashboard` behind a Cargo feature flag (e.g., `--features dashboard`) would let users who only need `record` and `hotspots` skip the axum/tokio dependency tree and the embedded SPA. This is a valid optimization but adds CI matrix complexity (feature-gated builds, feature-gated tests) and a user-facing configuration surface that doesn't exist today.

The initial implementation compiles the dashboard unconditionally. If binary size or compile time becomes a real user complaint, a `dashboard` feature flag can be added in a follow-up change with clear migration instructions. The crate separation in Decision 1 already isolates the dashboard dependency tree — adding a feature flag later is a one-line workspace config change plus conditional compilation in scryrs-cli.

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| **No graph/route data yet** — the dashboard launches showing only hotspot and event data, which may feel thin. | Explicitly document the dashboard as iteratively enriched. Graph and route views land when those phases deliver their artifacts. The component architecture is designed for addition, not modification. |
| **Port conflicts** — `--port 8080` may collide with local services. | Default to 8080 but let the user specify any port. The CLI exits with a clear error if the port is in use. Consider auto-increment as a future nicety. |
| **Large trace stores slow the API** — SQLite queries on stores with millions of events. | API endpoints paginate (cursor-based for events, configurable limit). The hotspot report is already bounded by scoring, which aggregates. Document that very large stores (100k+ events) may be slow on the first load. |
| **Embedded SPA adds ~1–2 MB to binary size.** | Acceptable for a CLI tool. The SPA is minified and gzip-compressed by `rust-embed`. Binary size impact is similar to adding one medium crate dependency. |
| **SPA, CLI, and API form a three-vs-two-crate maintenance surface.** | Manageable. The SPA is built separately with Vite and embedded at compile time. The API crate has no frontend coupling beyond serving static files. CI adds a `vite build` step before `cargo build`. |
| **User expects a hosted dashboard** — `scryrs dashboard` is local-only. | Document this clearly in the command help text and the roadmap. The Phase 3 contract explicitly defers hosted/multi-user to later phases or explicit follow-up work. |
| **Frontend build dependency (Node.js/npm) breaks `cargo build` for contributors without a JS toolchain** — `build.rs` runs `npm ci && npm run build` which requires Node.js and npm on the build machine. | The build script checks for `npm` availability first. If `npm` is not found and the `dist/` directory already exists (e.g., from a release tarball with pre-built assets), the build proceeds using embedded assets. If `npm` is missing AND `dist/` is absent, `build.rs` emits a clear `cargo:warning` instructing the developer to install Node.js or use `--dev` mode. CI always has Node.js available, so published binaries always include the embedded SPA. |
| **502 Bad Gateway is unconventional for local SQLite errors** — HTTP 502 typically indicates an upstream gateway failure, not a local file-read error. | The 502 status is intentional: the dashboard API is a gateway onto the `.scryrs/` store, and a corrupt store is an upstream data-source failure. This distinguishes data-source errors (502) from missing-data errors (404) so the SPA can show different recovery UX for each. If implementers find this confusing, fall back to 500 for corrupt stores. |

## Open Questions

1. **Should the API also serve raw trace event files (`.scryrs/*.jsonl`)?** Currently the spec only covers SQLite and hotspot JSON. Raw JSONL access would let the SPA do client-side analysis but could be large.
2. **Should the dashboard auto-refresh when artifact files change?** File-watching with `notify` crate adds async file system events. V1 could skip this and require a manual browser refresh.
3. **Should `scryrs dashboard` automatically detect the repository root from working directory, like `hotspots` does?** Yes — `scryrs dashboard` from within a scryrs-initialized project should find `.scryrs/` relative to the project root. This mirrors `hotspots` behavior.
