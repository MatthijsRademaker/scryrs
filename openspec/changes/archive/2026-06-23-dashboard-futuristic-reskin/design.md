## Context

The scryrs dashboard (`crates/scryrs-dashboard/frontend/`) is a Vue 3 + Vite + Tailwind v4 + shadcn-vue (reka-mira style) SPA, served locally to view `.scryrs` trace artifacts. It has four primary views (Hotspots, Sessions, Events, About) plus Subject/Session detail views, an app shell with a slate sidebar, and shared UI primitives under `src/shared/ui/` (card, table, badge, button, alert, empty, select, shell). Theme tokens live in `src/app/styles.css` using oklch variables; the current palette is a light canvas with a dark slate sidebar and stone neutrals — a stock shadcn admin look.

The new scryrs logo establishes the target visual language: a deep midnight-navy field, a glowing cyan constellation seen through a lens over stacked documents, circuit-trace lines, and diamond spark accents. The trace data itself (hotspot scores, event-type/outcome distributions, session timelines, evidence graphs) is well suited to expressive visualization.

Constraints (from project rules and the proposal): no new runtime dependencies; no changes to the API client, routes, stores, or backend; the existing shadcn-vue + Bun + Tailwind v4 stack (`dashboard-frontend-stack` capability) stays. Verification runs through Docker-backed scripts; the host has no Node/Bun SDK for agents.

## Goals / Non-Goals

**Goals:**

- A dark-first, "calm Apple / data glows" visual system derived from the logo, applied cohesively across tokens, shell, and every view.
- Reserve glow, color emphasis, and motion for data; keep chrome near-monochrome.
- Reuse the existing stack and primitives — restyle rather than rebuild; add only small presentational components where needed.
- Preserve all existing behavior, data, routes, and view states (loading/empty/error).

**Non-Goals:**

- No backend, API, route, or store changes.
- No new charting/animation/runtime libraries.
- No full design-system component library rebuild; shadcn-vue primitives are restyled in place.
- Not aiming for a heavy neon "HUD" look — restraint is explicit (≈70% Apple calm, 30% futuristic accent).
- Light theme is a maintained fallback, not a second fully-bespoke art direction.

## Decisions

### Decision: Token-first reskin in `styles.css`, dark as default

Re-author the oklch token set in `src/app/styles.css` so the dark palette is the `:root` default and light becomes the `.light`/secondary variant (inverting today's dark-via-`.dark` convention). New/retuned tokens: midnight `--background`, raised `--card`/glass surface, near-white `--foreground`, cyan `--primary`/`--accent`/`--ring`, plus a `--flame` gradient token and per-state glow colors. Everything downstream consumes tokens, so most view restyling is Tailwind class work over correct tokens.

- **Why:** Centralizes the art direction; primitives and views already bind to these variables, so the blast radius of palette work is contained.
- **Alternatives considered:** A parallel theme file or a runtime theme provider — rejected as over-engineering for a single dark-first identity with a light fallback.

### Decision: Glass + depth via Tailwind utilities, not new components

Express frosted glass with translucent fills + `backdrop-blur` + hairline borders on the existing `Card`/shell primitives, and depth with layered shadows. Use a single ambient radial "aurora" behind hero regions (Hotspots hero, detail headers), implemented as a CSS background layer.

- **Why:** `backdrop-blur` is already used in the mobile header; Tailwind v4 + tokens cover this without dependencies.
- **Trade-off:** Backdrop blur has a GPU cost and varies across browsers; keep blurred surfaces bounded and provide a solid translucent fallback fill so a blur-less render still looks intentional.

### Decision: Motion with `tw-animate-css` + reka-ui only

Use the already-installed `tw-animate-css` for entrance/glow utilities and reka-ui transition primitives for state changes. Standardize on ~200–400ms ease-out, no bounce. Encapsulate the recurring "scry glow" entrance as a small shared utility/class so views stay consistent.

- **Why:** Satisfies the no-new-dependency constraint and keeps motion uniform.
- **Alternatives considered:** A spring/animation library (e.g. motion) — rejected; unnecessary for this restraint level.

### Decision: Small presentational components for data viz

Add a few stateless presentational components under `src/shared/ui/` as needed: a heat/flame indicator, an event-type distribution mini-bar, an outcome pulse chip, an event sparkline, and a lightweight constellation graph (inline SVG). These take already-available data (e.g. `HotspotEntry.counts`, `SessionDetail.events`) as props and render with tokens.

- **Why:** Keeps views declarative and the new visuals reusable/testable; avoids a charting dependency.
- **Alternatives considered:** A chart library — rejected for bundle weight and to keep the bespoke look; inline SVG + CSS is sufficient for sparklines, mini-bars, and a small node graph.

### Decision: Logo wired as an asset slot with text fallback

Render the provided logo in the shell lockup via a slot/`<img>` with the scryrs text wordmark as the fallback when the asset is missing or fails to load, so the broken-asset history can't regress into a broken-image icon.

- **Why:** The spec requires graceful degradation; decouples the visual work from the binary asset's availability.

## Risks / Trade-offs

- **Backdrop-blur performance/inconsistency across browsers** → Bound the number of blurred layers per view; ship a solid translucent fallback fill so a blur-less render is still coherent.
- **Glow/contrast hurting legibility (esp. light fallback)** → Treat glow as additive decoration only; keep text/background contrast within accessible range in both themes; gate glow-dependent affordances behind a non-glow cue (color/weight) too.
- **Scope creep into a full HUD redesign** → The "chrome calm / data glows" requirement is the guardrail; flame is contractually limited to hotspot heat and aurora to one-per-view.
- **Regressing existing view behavior while restyling** → Each view spec explicitly preserves loading/empty/error/sort/nav behavior; restyle markup without touching store calls or data flow.
- **Cannot build/preview on host (no Bun SDK)** → Verification runs via the Docker-backed scripts (`scripts/check`, `scripts/precommit-run`); type-check (`bun run check`) and build run there. Visual confirmation is via running the dashboard locally where Bun is available.

## Migration Plan

1. Re-author tokens in `styles.css` (dark default + light fallback); verify type-check/build still pass.
2. Restyle shared primitives (Card, Table, Badge, Button, shell) to the glass/token treatment.
3. Restyle the shell (rail, logo lockup + fallback, active glow, aurora canvas).
4. Build presentational components (flame indicator, mini-bar, pulse chip, sparkline, constellation) incrementally as each view needs them.
5. Transform views in order: Hotspots hero → Sessions ribbon → Events feed → detail constellations → About.
6. Verify behavior preservation per view and run Docker-backed checks.

Rollback: the change is presentational and dependency-free; reverting the diff restores the prior look with no data/route/store implications.

## Open Questions

- Should a user-facing theme toggle (dark/light) be exposed in the shell now, or is dark-default with light as an internal fallback sufficient for this change? (Leaning: defer the toggle; out of scope unless requested.)
- Final logo asset path/format and whether a monochrome rail variant is needed in addition to the full lockup.
