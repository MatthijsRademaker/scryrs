## Context

scryrs has two delivered phases: proxy capture (Phase 1) and hotspot materialization (Phase 2). Users can record traces and produce deterministic hotspot reports, but output still lives in JSON on stdout and flat files in `.scryrs/`. There is no visual browsing, no session timeline, no subject drill-down, and no stable frontend foundation for later graph and route surfaces.

The proposal inserts Dashboard between Phase 2 and current Phase 3 (Graph and Route Manifests, pushed to Phase 4). This design covers `scryrs dashboard`, embedded HTTP server, and Vue.js SPA frontend.

`~/repos/personal/TheGreatMigration/frontend/` is donor reference for frontend infrastructure and design-system conventions, not donor product behavior. scryrs should copy or adapt reusable shell, aliases, token setup, and shadcn-vue component structure while rejecting donor domain views, generated clients, public assets, container files, and extra lockfiles.

Constraints:

- Zero changes to existing CLI behavior (`record`, `hotspots`, `init`, `--help`, `--help-json`, `--version`).
- No new external dependencies for existing crates.
- Dashboard crate must be new workspace member, not woven into `scryrs-cli`.
- SPA must be embeddable in binary.
- Dashboard must handle missing or partial data (`.scryrs/scryrs.db`, `.scryrs/hotspots.json`, empty stores, unreadable files).

## Goals / Non-Goals

**Goals:**

- Ship `scryrs dashboard [--port <PORT>] [--no-open] [--bind <ADDR>]` that serves Vue.js SPA and reads `.scryrs/*.json` + SQLite data.
- Provide visual browsing of hotspot report data: table view, subject drill-down, session timeline, and event distribution view.
- Build frontend on Vue 3, Vite, strict TypeScript, Bun, Tailwind CSS v4, shadcn-vue, Reka UI, lucide-vue, Vue Router, and Pinia.
- Keep frontend extensible through shared layout, typed fetch modules, and source-owned shadcn-vue components so later graph/route views add cleanly.
- Embed built SPA as static assets in Rust binary.
- Serve data via local REST API: `GET /api/hotspots`, `GET /api/sessions`, `GET /api/events`.
- Register `scryrs dashboard` as subcommand of existing `scryrs` binary.

**Non-Goals:**

- No hosted or multi-user dashboard.
- No authentication, write API, or data mutation.
- No real-time or streaming updates.
- No graph visualization or route exploration in Phase 3.
- No standalone dashboard deployment pipeline.
- No generated OpenAPI client adoption in Phase 3.
- No alternate frontend package-manager path or extra frontend lockfiles beside Bun.
- No second bespoke UI system beside shadcn-vue composition and semantic tokens.
- No behavioral changes to existing CLI commands or existing crates outside new dashboard integration.

## Decisions

### Decision 1: New `scryrs-dashboard` crate vs. adding to `scryrs-cli`

**Chosen**: New crate `crates/scryrs-dashboard/`.

Dashboard brings its own dependency tree (HTTP server, asset embedding, JSON types) and its own test surface. Keeping it separate makes boundary explicit and avoids mixing CLI parsing with HTTP serving logic.

### Decision 2: Embedded SPA via `rust-embed`

**Chosen**: `rust-embed` at build time.

Vite output under `frontend/dist/` is compiled into binary. This preserves single-binary distribution and avoids fragile filesystem coupling.

### Decision 3: HTTP server framework

**Chosen**: `axum` on `tokio`.

`axum` fits local-only server well and keeps handlers straightforward. Server exposes three route groups:

- `GET /` — serve SPA entrypoint
- `GET /assets/*path` — serve static assets
- `GET /api/*` — dashboard API endpoints backed by SQLite + artifact file reads

### Decision 4: API design — read artifact files and SQLite

**Chosen**: API reads `.scryrs/hotspots.json` for hotspot report and opens `.scryrs/scryrs.db` via `rusqlite` for session/event queries.

Dashboard is viewer, not scorer. It reuses artifacts produced by existing flows rather than recomputing them.

### Decision 5: CLI integration — `scryrs dashboard`

**Chosen**: `scryrs-cli` recognizes `dashboard` and delegates to `scryrs_dashboard::run(config)`.

Flags:

- `--port`, `-p` (default `8080`)
- `--bind`, `-b` (default `127.0.0.1`)
- `--no-open`
- `--dev`

### Decision 6: Frontend stack is Bun + Tailwind v4 + shadcn-vue

**Chosen**: Vue 3 + Vite + strict TypeScript + Bun + Tailwind CSS v4 + shadcn-vue + Reka UI + lucide-vue + Vue Router + Pinia.

Vue 3 with Composition API remains base. Vite is build tool. Bun is only frontend package manager and script runner. Tailwind CSS v4 and shadcn-vue provide semantic token and component foundation. Vue Router keeps route surfaces explicit. Pinia manages hotspot/session/event state.

Dashboard views should compose shared shadcn-vue primitives under `src/shared/ui` and global semantic styling from `src/app/styles.css`. Shell, cards, tables, badges, alerts, empty states, skeletons, filters, dialogs, and sidebar layout should come from source-owned shadcn-vue components or thin wrappers over them.

Reusable donor assets:

- `package.json` dependency shape, adapted to scryrs scripts and Bun commands
- `vite.config.ts` with Vue plugin, `@tailwindcss/vite`, and `@` alias
- `tsconfig.json` strict TypeScript + alias setup
- `components.json` adapted to `@/shared`, `@/shared/ui`, `@/shared/lib/utils`
- `src/app/styles.css` semantic token setup, rethemed for scryrs
- `src/shared/ui/**` and `src/shared/lib/utils.ts`
- shared app-shell and sidebar patterns adapted to hotspot/session/event routes

Rejected donor assets:

- donor product pages or labels unrelated to scryrs dashboard
- generated API clients or generator scripts
- public images or brand assets
- Dockerfile for standalone deployment
- `package-lock.json`, `pnpm-lock.yaml`, or `yarn.lock`

API access stays hand-written and typed. `src/shared/api/client.ts` exposes fetch helpers for `GET /api/hotspots`, `GET /api/sessions`, and `GET /api/events`, with local TypeScript interfaces derived from Rust response contracts.

Event distribution and timeline visuals must fit semantic token system. Phase 3 may use simple semantic table/SVG/bar primitives or wrapped design-system-compatible chart components. Visual consistency matters more than any specific chart package.

### Decision 7: Async/SQLite bridging strategy

**Chosen**: `tokio::task::spawn_blocking` for `rusqlite` query operations.

`rusqlite` is synchronous. Blocking reads move onto Tokio blocking pool to keep HTTP handlers async without adding heavier database stack.

### Decision 8: Cargo feature-gating for dashboard crate

**Chosen**: Deferred.

Initial implementation compiles dashboard unconditionally as workspace member. Feature-gating can come later if binary size or compile time becomes real issue.

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| No graph/route data yet may make dashboard feel thin. | Document Phase 3 as iterative baseline and keep route/layout architecture easy to extend. |
| Port conflicts on default `8080`. | Keep configurable `--port` and fail with clear bind error. |
| Large trace stores may slow SQLite-backed endpoints. | Paginate events, keep queries narrow, and document first-load cost on large stores. |
| Embedded SPA increases binary size. | Accept cost; minified assets are reasonable for CLI distribution. |
| SPA, CLI, and API add maintenance surface. | Keep crate boundaries explicit and frontend responsibilities narrow. |
| Users may expect hosted dashboard. | Document local-only scope in help text and roadmap. |
| Bun dependency can block contributors without frontend toolchain when `dist/` is stale or absent. | Build script checks for Bun and fails loudly with install guidance when rebuild is required. Prebuilt `dist/` remains valid fallback for release artifacts. |
| Corrupt local store mapped to 502 may surprise readers. | Document API as gateway onto `.scryrs/` data source and keep frontend recovery states explicit. |

## Open Questions

1. Should API also serve raw trace event files (`.scryrs/*.jsonl`), or is SQLite + hotspot JSON enough for Phase 3?
2. Should dashboard auto-refresh when artifact files change, or is manual browser refresh enough for V1?
3. Should Phase 3 visuals start with semantic table/SVG primitives or wrapped chart components from start?
4. Should `scryrs dashboard` detect repository root from working directory exactly like `hotspots`?
