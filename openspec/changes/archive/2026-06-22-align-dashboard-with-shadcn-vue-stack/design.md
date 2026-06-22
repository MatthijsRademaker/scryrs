## Context

Phase 3 Dashboard is already proposed as the next product surface after hotspot materialization. The active dashboard change currently defines the frontend as Vue 3 + Vite + Pinia + Vue Router + Chart.js, embedded into the Rust binary, but it also says "no component library" and uses custom CSS. That would force scryrs to build table, sidebar, card, empty-state, and chart primitives from scratch exactly where the project needs a reusable frontend foundation for later graph/route views.

`~/repos/personal/TheGreatMigration/frontend/` is a better frontend donor for this repository's dashboard direction. Its useful parts are stack and infrastructure: Vue 3, Vite, strict TypeScript, Vue Router, Pinia, Tailwind CSS v4 through `@tailwindcss/vite`, shadcn-vue, Reka UI, lucide icons, `components.json` aliases, `shared/ui`, `shared/lib/utils.ts`, app shell/sidebar patterns, and explicit shadcn-vue rules. Its moving-domain pages, generated API client, images, Dockerfile, and package-lock artifacts are not reusable product logic for scryrs.

The result should be a design-system-first dashboard scaffold under `crates/scryrs-dashboard/frontend/`, with Bun as the package manager and a curated donor import boundary. The Rust dashboard crate remains responsible for axum APIs and embedded static assets; this change only establishes the frontend stack and guidance contract before implementation.

## Goals / Non-Goals

**Goals:**

- Standardize Phase 3 dashboard frontend on Vue 3 + Vite + TypeScript + Bun + Tailwind CSS v4 + shadcn-vue + Reka UI + lucide-vue.
- Use TheGreatMigration as a donor reference for frontend infrastructure and design-system conventions.
- Establish a curated copy boundary so implementation imports reusable frontend foundation without dragging moving-domain application code into scryrs.
- Replace the custom-CSS/no-component-library direction with shadcn-vue-first UI composition.
- Keep API access simple: typed fetch functions against the local dashboard REST API, no generated OpenAPI client unless scryrs later emits an OpenAPI contract.
- Add project-local shadcn-vue guidance for agents building dashboard UI.
- Keep future Phase 4 graph/route views easy to add through route registration, navigation config, and reusable layout/components.

**Non-Goals:**

- No application-code copy of TheGreatMigration's moving pages, task/calendar/people domain, generated client, images, or tests.
- No hosted dashboard, authentication, write API, real-time updates, or graph/route visualization.
- No OpenAPI generator adoption in Phase 3.
- No duplicate component system beside shadcn-vue.
- No npm/package-lock fallback in the dashboard frontend package.
- No attempt to preserve the old custom-CSS-only dashboard frontend direction.

## Decisions

### Decision 1: Use TheGreatMigration as donor reference, not whole-repo transplant

**Chosen:** Curated copy/reference list.

Reusable donor assets:

- `frontend/package.json` dependency shape, adjusted to scryrs name and Bun scripts.
- `frontend/vite.config.ts` Vue + Tailwind v4 plugin setup.
- `frontend/tsconfig.json` strict TypeScript + `@/*` alias setup.
- `frontend/components.json` with shadcn-vue aliases adapted to `@/shared`, `@/shared/ui`, `@/shared/lib/utils`.
- `frontend/src/app/styles.css` design-token architecture, re-themed for scryrs.
- `frontend/src/shared/ui/**` shadcn-vue components needed by dashboard views.
- `frontend/src/shared/lib/utils.ts` `cn()` utility.
- App shell/sidebar patterns, adapted to scryrs navigation labels.
- `.agents/skills/shadcn-vue/**` guidance, adapted into this repo's `.pi/skills/` or equivalent rules surface.

Rejected donor assets:

- Moving-domain features: home, tasks, calendar, people, rooms, settings.
- Generated OpenAPI client and generator scripts.
- Public images and brand assets.
- Dockerfile unless a later change introduces standalone dashboard containerization.
- `package-lock.json` and npm-specific workflows.
- Tests whose assertions only describe TheGreatMigration behavior.

**Alternatives considered:**

- **Wholesale copy then strip:** Fast initially but likely leaves stale domain references, wrong dependencies, unused assets, and misleading tests. Rejected.
- **Fresh scaffold from `shadcn-vue init`:** Clean but ignores already-proven local conventions from donor repo. Rejected as slower and less grounded.

### Decision 2: Bun is the only dashboard frontend package manager

**Chosen:** Dashboard frontend uses `bun.lock`, `bun install`, `bun run build`, `bun run check`, and `bunx --bun shadcn-vue@latest`.

The donor repo contains both `bun.lock` and `package-lock.json`, but scryrs should not inherit that ambiguity. The dashboard build integration should fail loudly when Bun is unavailable and no prebuilt `dist/` exists, rather than silently falling back to npm.

**Alternatives considered:**

- **npm for compatibility:** Existing active dashboard design uses npm, but user intent is explicitly Bun and TheGreatMigration already has a Bun lock. Rejected.
- **Dual npm/Bun support:** Adds branches and lockfile drift. Rejected.

### Decision 3: shadcn-vue is the component system

**Chosen:** shadcn-vue components live as source under `src/shared/ui`, backed by Reka UI primitives, Tailwind CSS v4, semantic CSS variables, and lucide-vue icons.

Dashboard views should compose existing shadcn-vue primitives first: Sidebar, Card, Table, Badge, Button, Tabs, Select, Tooltip, Skeleton, Alert, Empty, Sheet/Dialog where useful. Domain-specific dashboard components should be thin wrappers over these primitives, not a second raw-HTML component system.

**Alternatives considered:**

- **Custom CSS only:** Current active design says this, but it creates bespoke primitives and higher maintenance. Rejected.
- **Another UI kit:** Inconsistent with donor stack and shadcn-vue skill guidance. Rejected.

### Decision 4: Tailwind CSS v4 plus semantic design tokens

**Chosen:** `src/app/styles.css` is the single global style entrypoint. It imports Tailwind v4, registers semantic color/spacing/typography tokens through `@theme inline`, defines light/dark variables, and exposes dashboard-specific status/chart tokens.

The dashboard should use semantic utilities (`bg-background`, `text-muted-foreground`, `border-border`, `bg-destructive-soft`, etc.) instead of raw colors. If scryrs needs event-family colors, add semantic variables such as `--event-file`, `--event-search`, and map them through `@theme inline`.

**Alternatives considered:**

- **Tailwind config v3 style:** Not aligned with donor repo and shadcn-vue v4 setup. Rejected.
- **Raw CSS modules:** Would bypass shadcn-vue conventions. Rejected.

### Decision 5: Typed hand-written REST client first

**Chosen:** `src/shared/api/client.ts` exposes typed functions for `/api/hotspots`, `/api/sessions`, and `/api/events` using `fetch` and local TypeScript interfaces derived from Rust response contracts.

TheGreatMigration's OpenAPI-generated client should not be copied. scryrs does not currently emit an OpenAPI document for the local dashboard API, and generator setup would add complexity before the API stabilizes.

**Alternatives considered:**

- **Copy donor generated client:** Wrong domain and wrong source-of-truth. Rejected.
- **Add OpenAPI generator now:** Premature; can be a later explicit change if server emits a contract. Rejected.

### Decision 6: Charting must fit the design system

**Chosen:** Phase 3 charting should use shadcn-vue-compatible chart components if available and documented, or simple semantic CSS/SVG chart primitives for bar/timeline distributions. Chart.js is not mandatory unless later evaluation proves it is the simplest compatible option.

This keeps chart colors, typography, card layout, empty/loading states, and interaction patterns consistent with shadcn-vue. For Phase 3 scope, event distribution can be a semantic bar chart/table if that delivers clearer, testable UI with fewer dependencies.

**Alternatives considered:**

- **Keep Chart.js requirement:** It matches active roadmap text, but not the requested donor stack. Rejected as default.
- **D3/visx:** Too much surface for Phase 3. Rejected.

### Decision 7: Agent guidance is part of the frontend contract

**Chosen:** Add or adapt project-local shadcn-vue guidance from TheGreatMigration, including CLI usage, styling, composition, form, icon, theming, and component update rules.

Agents implementing dashboard UI must load this guidance before adding or changing shadcn-vue components. This prevents common failures: raw colors, raw spacing stacks, missing `DialogTitle`, icon sizing overrides, ungrouped select/dropdown items, and custom markup where shadcn-vue components already exist.

**Alternatives considered:**

- **Rely on memory or upstream docs only:** Error-prone and slower. Rejected.
- **Copy guidance verbatim forever:** Donor-specific paths and examples may drift. Adaptation required.

## Risks / Trade-offs

| Risk | Mitigation |
| --- | --- |
| Copying donor frontend infrastructure may import hidden moving-domain assumptions. | Use explicit allowlist/denylist and review every copied file for project name, route, image, API, and domain references. |
| shadcn-vue components increase source volume. | Accept source-owned components as design-system foundation; copy/add only components used by dashboard views. |
| Bun requirement can block contributors without Bun installed. | Build script should fail loudly with clear install guidance unless `dist/` already exists for release embedding. |
| Chart.js removal may surprise readers of current roadmap. | Update active dashboard artifacts to say charting must be design-system-compatible; event distribution remains required, Chart.js does not. |
| Tailwind/shadcn-vue guidance can become stale. | Pin `shadcn-vue` version in `package.json`, use `bunx --bun shadcn-vue@latest info`, and refresh guidance only through explicit maintenance changes. |
| Active dashboard change already exists with conflicting tasks. | Treat this change as superseding the frontend portions before `/opsx-apply` on Phase 3; implementation must update old tasks or apply this proposal first. |

## Migration Plan

1. Update the active Phase 3 dashboard proposal/design/spec/tasks to replace custom CSS/no-component-library/npm/Chart.js defaults with this stack contract.
2. Scaffold `crates/scryrs-dashboard/frontend/` from curated donor pieces and remove all donor domain references before first build.
3. Add/adapt shadcn-vue dashboard guidance into project-local agent instructions.
4. Install frontend dependencies with Bun and commit only `bun.lock`.
5. Build and check the frontend with `bun run check` and `bun run build` before embedding in Rust.
6. If the stack proves wrong before release, rollback by reverting the dashboard frontend scaffold and restoring the previous active dashboard design; no existing shipped CLI behavior is affected because dashboard is not yet implemented.

## Open Questions

1. Should shadcn-vue dashboard guidance live under `.pi/skills/shadcn-vue/`, `.pi/rules/`, or both?
2. Should Phase 3 use shadcn-vue chart components, simple CSS/SVG primitives, or Chart.js wrapped in shadcn-style cards?
3. Should the dashboard include dark-mode toggle in Phase 3, or only ship token-ready dark mode without UI toggle?
4. Which shadcn-vue preset should be canonical: donor `reka-mira` or a scryrs-specific preset chosen during implementation?
