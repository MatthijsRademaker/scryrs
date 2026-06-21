## Why

scryrs can record traces and materialize hotspot reports, but consuming this data requires reading JSON on stdout. Developers exploring their trace corpus need visual browsing — sorting, filtering, drilling into subjects, comparing sessions — that a terminal JSON dump cannot provide. Adding a dashboard phase now, while the data surface is still small, lets us establish the `scryrs dashboard` CLI command and Vue.js frontend before graph/route data multiplies the visualization surface. This is the next logical step after Phase 2: make hotspot data visually browsable before building more data infrastructure on top.

## What Changes

1. **Roadmap update**: Insert a new **Phase 3 — Dashboard** between the delivered Phase 2 and the current Phase 3 (Graph and Route Manifests, pushed to Phase 4). Update all downstream phase numbers.

2. **New CLI command**: `scryrs dashboard` that starts a local HTTP server and opens a Vue.js dashboard SPA.

3. **Vue.js dashboard application**: An SPA served by the CLI that visualizes hotspot data, session timelines, event distributions, and per-subject drill-downs. Built with clean component separation so graph/route visualizations can be dropped in during later phases.

4. **Spec files**: Define the dashboard CLI contract (`cli-dashboard-command`), the dashboard frontend architecture (`dashboard-frontend-app`), and the Phase 3 goal definition on the roadmap (`dashboard-phase-goal`).

5. **No changes to existing CLI surface**: `scryrs record`, `scryrs hotspots`, `scryrs init`, `--help`, `--help-json`, `--version`, exit codes, and all existing behavior remain identical.

## Capabilities

### New Capabilities

- `dashboard-phase-goal`: Defines Phase 3 on the roadmap — what the dashboard phase delivers, what it explicitly defers, and how it relates to surrounding phases.
- `cli-dashboard-command`: The `scryrs dashboard` CLI surface — argument parsing, HTTP server lifecycle, artifact file serving contract.
- `dashboard-frontend-app`: The Vue.js SPA architecture — component tree, data loading from `.scryrs/*.json` files, visualization scope, and extensibility points for future graph/route views.

### Modified Capabilities

- (none — no existing capability has its requirements changed)

## Impact

- **Roadmap update**: `.devagent/docs/docs/roadmap.mdx` — Phase 3 becomes Dashboard, Phase 4 becomes Graph and Route Manifests, etc. Scope guardrail #5 is removed or updated since dashboards are now a planned phase.
- **New crate**: `crates/scryrs-dashboard/` — the `scryrs dashboard` CLI command and embedded SPA server.
- **New frontend code**: `crates/scryrs-dashboard/frontend/` — Vue.js SPA source, built and embedded into the binary as static assets.
- **No behavioral changes to existing crates**: `scryrs record`, `scryrs hotspots`, `scryrs init`, `--help`, `--help-json`, `--version`, exit codes, and all existing behavior remain identical. The `scryrs-cli` crate will gain a `scryrs-dashboard` dependency and delegate the `dashboard` subcommand in the implementation PR.
- **Docs change**: `cli-v0-contract.md` updated to document the `dashboard` command contract.
