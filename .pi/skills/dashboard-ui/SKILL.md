---
name: dashboard-ui
description: "Recipe for dashboard frontend work — views, routes, stores, icons, and data flow in crates/scryrs-dashboard/frontend/. Use when: the task mentions crates/scryrs-dashboard/frontend, a dashboard view/page/chart, Vue components, Pinia stores, or dashboard icons. Do not use for: shadcn-vue CLI usage, components.json, or semantic-token theming rules (shadcn-vue skill), or generic visual-design aesthetics (frontend-design skill)."
metadata:
  curated: true
  sources:
    - crates/scryrs-dashboard/frontend/src/**
    - crates/scryrs-dashboard/frontend/components.json
    - crates/scryrs-dashboard/frontend/package.json
  last-verified: 2026-07-04
  verified-commit: 8f8ca6cdcee59fb31b4a12f5ab8c48787f2c345c
---

# Dashboard UI Work

Vue 3.5 + Vite + Tailwind v4 + Pinia, Bun-only, `@` aliases to `crates/scryrs-dashboard/frontend/src`. All paths below are under `crates/scryrs-dashboard/frontend/`: views in `crates/scryrs-dashboard/frontend/src/views/`, primitives in `crates/scryrs-dashboard/frontend/src/shared/ui/` (barrel-exported via its `index.ts`), API client + DTO types in `crates/scryrs-dashboard/frontend/src/shared/api/client.ts`, reusable logic in `crates/scryrs-dashboard/frontend/src/shared/lib/`, stores in `crates/scryrs-dashboard/frontend/src/stores/`.

## Read first

1. `crates/scryrs-dashboard/frontend/src/router/index.ts` — all routes, lazy-import pattern, route names (used for active-nav matching).
2. `crates/scryrs-dashboard/frontend/src/shared/lib/dashboard-mode.ts` — `navigationForMode()`: the sidebar nav is mode-aware (`local` vs `live`); most UI branches on `meta.mode`.

## Steps

1. Check what already exists before building: primitives (`ls crates/scryrs-dashboard/frontend/src/shared/ui/`), viz components (`ls crates/scryrs-dashboard/frontend/src/shared/ui/viz/` — ConstellationGraph, EventSparkline, EventTypeBar, FlameIndicator, OutcomePulse), helpers in `crates/scryrs-dashboard/frontend/src/shared/lib/`. Compose from these; do not hand-roll duplicates.
   Verify: you can name which existing primitive/store each part of your change reuses.
2. New view: SFC under `crates/scryrs-dashboard/frontend/src/views/`, route object in `crates/scryrs-dashboard/frontend/src/router/index.ts` with lazy `component: () => import("@/views/...")`, and — if it belongs in the sidebar — an entry in `navigationForMode()` in `crates/scryrs-dashboard/frontend/src/shared/lib/dashboard-mode.ts` for the right mode(s).
   Verify: `grep -n "<ViewName>" crates/scryrs-dashboard/frontend/src/router/index.ts crates/scryrs-dashboard/frontend/src/shared/lib/dashboard-mode.ts` — hits where expected.
3. New shared primitive: folder + `index.ts` barrel under `crates/scryrs-dashboard/frontend/src/shared/ui/<name>/`, then re-export from `crates/scryrs-dashboard/frontend/src/shared/ui/index.ts`. (The stray `crates/scryrs-dashboard/frontend/src/components/HeroCard.vue` is the exception, not the pattern.)
   Verify: `grep -n "<name>" crates/scryrs-dashboard/frontend/src/shared/ui/index.ts` — export present.
4. New icon: create `Icon<Name>.vue` in `crates/scryrs-dashboard/frontend/src/shared/ui/icon/` as a hand-written inline `<svg>` (24×24, `stroke="currentColor"`, `aria-hidden="true"` — copy lucide path data inline), export from `icon/index.ts`. **Never import from `lucide-vue`** — it is the Vue 2 package; it sits unused in package.json as dead weight.
   Verify: `grep -rn "lucide" crates/scryrs-dashboard/frontend/src/` — zero hits, still.
5. Data fetching goes through `crates/scryrs-dashboard/frontend/src/shared/api/client.ts` and a Pinia store in `crates/scryrs-dashboard/frontend/src/stores/` (`useMetaStore` in `meta.ts` owns `mode`/`repositoryId`). Respect `DashboardMode` — don't hardcode endpoints or assume local mode.
   Verify: `grep -n "mode" <your changed store/view files>` — mode handled, not assumed.
6. Typecheck/build. `bun run check` (vue-tsc) and `bun run build` from the frontend dir need **Bun on the host** — the frontend step of `scripts/check` is NOT dockerized. If you have no Bun/Node, your only in-repo build verification is the full `docker build -t scryrs-server .` (frontend compiles via the dashboard crate's `build.rs`); otherwise rely on the static Verify greps above and say so in your outcome report.
   Verify: `bun run check` exit 0 from `crates/scryrs-dashboard/frontend/`, or the docker build succeeds, or you explicitly reported static-only verification.

## Easy-to-miss details

- **Dark mode is dark-FIRST.** `:root` in `crates/scryrs-dashboard/frontend/src/app/styles.css` holds the dark palette; light is opt-in via `@custom-variant light`. Never add manual `dark:` overrides and never assume a light default.
- **Semantic tokens go beyond stock shadcn**: `--success*`, `--warning*`, `--info*` (with `-soft` variants), `--sidebar*`, `--flame-*`, `--chart-1..4` in `crates/scryrs-dashboard/frontend/src/app/styles.css`. Use these, not raw colors.
- **`@/shared/composables` is a lie**: components.json declares the alias but the composables directory does not exist under `crates/scryrs-dashboard/frontend/src/shared/`. Reusable logic lives in `crates/scryrs-dashboard/frontend/src/shared/lib/`.
- **TS strictness will bite lazy code**: `noUnusedLocals`/`noUnusedParameters`/`strict` are on; `bun run check` fails on an unused variable.
- **DTO interfaces in `client.ts` are hand-mirrored from Rust** (`HotspotEntry`, `HotspotsReport`, …), not generated. If your task also touches `scryrs-types`, the mirror must be updated by hand — see graph-core-changes.
- Dev proxy: `vite.config.ts` proxies `/api` to `API_PROXY_URL` (default `http://localhost:8080`); fetches assume same-origin `/api`.

## Self-check before reporting done

- [ ] Zero `lucide` imports (`grep -rn "lucide" crates/scryrs-dashboard/frontend/src/`)?
- [ ] New view registered in router AND (if sidebar-worthy) in `navigationForMode()` for the correct mode(s)?
- [ ] Only semantic tokens used — no raw hex/oklch colors, no `dark:` overrides?
- [ ] Typecheck/build ran (Bun or docker build), or static-only verification explicitly reported?
