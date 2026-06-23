## Why

The scryrs dashboard is functionally correct but visually generic — a stock shadcn admin template (light canvas, slate sidebar, flat tables) wrapped around genuinely rich trace data (hotspot scores, event-type and outcome distributions, session timelines, evidence graphs). The data wants to be felt, not just read. The new scryrs logo — a glowing constellation viewed through a lens over a deep midnight canvas — already defines a complete visual language. This change extends that language across the whole dashboard to make it feel premium, futuristic, and unmistakably scryrs.

## What Changes

- Introduce a **dark-first visual design system** derived from the logo: deep midnight-navy canvas, frosted-glass surfaces (Apple-style vibrancy), a single electric cyan-blue accent, and a warm flame gradient reserved exclusively for hotspot heat.
- Establish a **"chrome stays calm, data glows" discipline**: navigation, surfaces, and structural elements remain near-monochrome; cyan glow, flame heat, and motion are reserved for the data itself.
- Replace the theme tokens in `src/app/styles.css` with the new dark-first palette (oklch), with light mode preserved as a secondary, calmer fallback.
- Restyle the **app shell** (`DashboardShell.vue`): glass navigation rail, logo lockup, active-item cyan edge-glow, midnight aurora canvas, placeholder slot for the user-provided logo asset.
- Transform each **view** from flat tables into the design language:
  - **Hotspots** (landing) → a glass "heat leaderboard" hero (flame intensity = score, inline event-type mini-bars, outcome pulse chips) above a restyled table.
  - **Sessions** → an activity ribbon/timeline with per-session event sparklines and live pulse for active sessions.
  - **Events** → a live monospace feed where new events fade-and-rise in with a brief cyan "scry" glow, color-coded by event type.
  - **Subject / Session detail** → constellation graph of related sessions/events plus glass stat tiles.
- Define **motion rules** (spring, ~200–400ms, ease-out, never bouncy) implemented with the already-installed `tw-animate-css` and reka-ui transition primitives.
- No new runtime dependencies and no changes to the API client, data contracts, or routes.

## Capabilities

### New Capabilities
- `dashboard-visual-design`: The scryrs dashboard's visual identity — dark-first token system, glass/material treatment, motion rules, the "chrome calm / data glows" discipline, and the per-view visual treatments for shell, hotspots, sessions, events, and detail views.

### Modified Capabilities
<!-- None. The dashboard-frontend-stack capability (Vue/Vite/Tailwind/Bun stack, donor reference) is unchanged; this change only adds the visual-design layer on top of it. -->

## Impact

- **Code**: `crates/scryrs-dashboard/frontend/src/app/styles.css` (tokens), `src/shared/ui/shell/DashboardShell.vue`, all `src/views/*.vue`, and shared UI primitives under `src/shared/ui/` (card, table, badge, button) restyled for the new materials. Possible small additions of presentational components (e.g. mini-bar, pulse chip, sparkline) under `src/shared/ui/`.
- **Assets**: A real logo asset is provided by the user and wired into the shell.
- **Dependencies**: None added — uses existing Tailwind v4, `tw-animate-css`, reka-ui, lucide-vue.
- **APIs / data**: Unchanged. No changes to `src/shared/api/client.ts`, routes, stores, or backend endpoints.
- **Themes**: Default theme becomes dark; light mode retained as a fallback.
