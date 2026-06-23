## 1. Foundation — tokens & theme

- [x] 1.1 Re-author `src/app/styles.css` oklch tokens: dark-first `:root` (midnight `--background`, raised glass surface, near-white `--foreground`, cyan `--primary`/`--accent`/`--ring`), with light moved to a secondary variant
- [x] 1.2 Add a `--flame` gradient token and per-state glow color tokens (accent-hot, ok, err)
- [x] 1.3 Verify dark theme renders as default with no stored preference; confirm light fallback stays legible
- [x] 1.4 Run Docker-backed type-check/build (`bun run check`, build) to confirm tokens compile

## 2. Shared primitives — glass & material

- [x] 2.1 Restyle `Card` (and Card sub-components) to frosted glass: translucent fill, `backdrop-blur`, hairline border, layered shadow, with a solid translucent fallback fill
- [x] 2.2 Restyle `Table` for the dark canvas (header, row hairlines, hover) using tokens
- [x] 2.3 Restyle `Badge` and `Button` to the new palette and focus/hover glow
- [x] 2.4 Add a reusable "scry glow" entrance utility/class (tw-animate-css) and standardize ~200–400ms ease-out motion; confirm no new dependency added

## 3. App shell

- [x] 3.1 Restyle `DashboardShell.vue` rail to frosted glass over the midnight canvas
- [x] 3.2 Add the logo lockup with `<img>` slot and scryrs text wordmark fallback (no broken-image when asset missing)
- [x] 3.3 Indicate active nav item with cyan edge-glow/lit indicator; keep inactive chrome near-monochrome
- [x] 3.4 Add a single ambient aurora/lens glow background to the canvas behind content

## 4. Data-viz presentational components

- [x] 4.1 Build a flame/heat indicator component (intensity prop → flame gradient)
- [x] 4.2 Build an event-type distribution mini-bar component (consumes `counts.eventType`)
- [x] 4.3 Build an outcome pulse chip component (consumes `counts.outcome`)
- [x] 4.4 Build an event sparkline component (inline SVG)
- [x] 4.5 Build a lightweight constellation graph component (inline SVG nodes + hairlines, glow by activity)

## 5. Hotspots view — heat leaderboard hero

- [x] 5.1 Render top-ranked subjects as glass heat cards; flame intensity = relative score
- [x] 5.2 Add inline event-type mini-bars and outcome pulse chips per card
- [x] 5.3 Restyle the detail table below the hero; preserve sorting, subject-detail links
- [x] 5.4 Verify empty and error-with-retry states render correctly

## 6. Sessions view — activity ribbon

- [x] 6.1 Render sessions as glass rows with an event sparkline each
- [x] 6.2 Indicate active (null `endedAt`) sessions with a cyan pulse marker
- [x] 6.3 Preserve start-time ordering and session-detail links; verify empty/error states

## 7. Events view — live scrying feed

- [x] 7.1 Render events as a monospace feed; color-code by event type
- [x] 7.2 Animate newly arriving events in (fade-and-rise + brief cyan scry glow)
- [x] 7.3 Preserve cursor/pagination loading and session filtering; verify empty/error states

## 8. Detail views — constellation

- [x] 8.1 Add constellation graph of related sessions/events to Subject and Session detail views
- [x] 8.2 Present key metrics as glass stat tiles
- [x] 8.3 Preserve underlying data, navigation links, and loading/empty/error states
- [x] 8.4 Restyle the About view to match the new identity

## 9. Verification & polish

- [x] 9.1 Confirm "chrome calm / data glows" discipline: flame only on hotspot heat, aurora at most once per view, cyan on chrome limited to active/interactive states
- [x] 9.2 Check contrast/legibility in both dark and light themes
- [x] 9.3 Run `scripts/precommit-run` (Docker-backed) and confirm type-check/build pass
- [x] 9.4 Run the dashboard locally and visually confirm each view against its spec scenarios
