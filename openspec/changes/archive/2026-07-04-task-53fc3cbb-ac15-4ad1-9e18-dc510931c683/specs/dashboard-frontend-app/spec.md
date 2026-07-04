## ADDED Requirements

### Requirement: Dashboard SPA uses shadcn-vue stack

The dashboard frontend SHALL be implemented with Vue 3, Vite, strict TypeScript, Bun, Tailwind CSS v4, shadcn-vue, Reka UI primitives, lucide-vue icons, Vue Router, and Pinia. The frontend SHALL use shadcn-vue component composition and semantic tokens as primary UI architecture.

#### Scenario: Frontend package declares canonical stack

- **WHEN** `crates/scryrs-dashboard/frontend/package.json` is inspected
- **THEN** it SHALL declare Vue 3, Vite, TypeScript, Vue Router, Pinia, Tailwind CSS v4, shadcn-vue, Reka UI, lucide-vue, class-variance-authority, clsx, and tailwind-merge in dependencies or devDependencies
- **AND** it SHALL NOT declare another primary dashboard component framework

#### Scenario: Frontend package manager and shadcn-vue commands use Bun

- **WHEN** dashboard frontend scaffold and build integration are inspected
- **THEN** `crates/scryrs-dashboard/frontend/bun.lock` SHALL exist
- **AND** `crates/scryrs-dashboard/frontend/package-lock.json`, `pnpm-lock.yaml`, and `yarn.lock` SHALL NOT exist
- **AND** dependency installation SHALL use `bun install`
- **AND** production build SHALL use `bun run build`
- **AND** type checking SHALL use `bun run check`
- **AND** shadcn-vue CLI usage SHALL use `bunx --bun shadcn-vue@latest`

#### Scenario: shadcn-vue configuration uses shared aliases

- **WHEN** `crates/scryrs-dashboard/frontend/components.json` and Vite config are inspected
- **THEN** shadcn-vue SHALL be configured for Vite + TypeScript
- **AND** `@vitejs/plugin-vue` and `@tailwindcss/vite` SHALL be configured
- **AND** alias `@` SHALL resolve to frontend `src`
- **AND** shadcn-vue `ui` alias SHALL resolve under `@/shared/ui`
- **AND** shadcn-vue `utils` alias SHALL resolve under `@/shared/lib/utils`
- **AND** configured icon library SHALL be lucide

### Requirement: Dashboard frontend reuses donor infrastructure only within explicit boundary

The dashboard implementation SHALL use `~/repos/personal/TheGreatMigration/frontend/` as donor reference for reusable infrastructure and design-system patterns only. It SHALL adapt reusable files to scryrs context and SHALL NOT copy donor domain behavior or unrelated assets.

#### Scenario: Allowed donor assets stay infrastructure-focused

- **WHEN** donor-derived dashboard files are added
- **THEN** they MAY include Vite config, TypeScript config, `components.json`, global Tailwind/design-token CSS, `src/shared/ui/**`, `src/shared/lib/utils.ts`, and shared app-shell/sidebar patterns
- **AND** every copied file SHALL be adapted to scryrs names, routes, API contracts, and branding

#### Scenario: Rejected donor assets are absent

- **WHEN** dashboard frontend scaffold is inspected
- **THEN** it SHALL NOT include donor product pages or labels unrelated to hotspots, sessions, events, subjects, or dashboard docs
- **AND** it SHALL NOT include generated API client output or generator scripts
- **AND** it SHALL NOT include donor public images or brand assets
- **AND** it SHALL NOT include donor containerization files unless later change explicitly requires them

### Requirement: Dashboard API client is hand-written and typed

The dashboard frontend SHALL call local dashboard REST API through hand-written typed fetch functions. It SHALL NOT depend on generated client code.

#### Scenario: API client targets dashboard endpoints

- **WHEN** dashboard frontend API client is inspected
- **THEN** it SHALL expose typed functions for `GET /api/hotspots`, `GET /api/sessions`, and `GET /api/events`
- **AND** its TypeScript types SHALL match dashboard server response contracts
- **AND** non-2xx responses SHALL surface explicit typed errors to calling view or store

### Requirement: Dashboard SPA loads hotspot report data

The dashboard SHALL fetch and display hotspot report from `GET /api/hotspots` on load. Hotspot view SHALL be landing page of SPA.

#### Scenario: Landing page shows hotspot table

- **GIVEN** hotspot API returns non-empty `HotspotsReport`
- **WHEN** SPA loads at root path
- **THEN** it SHALL display table of hotspot entries with columns: Rank, Subject, Score, Session Count, Total Events, First Seen, Last Seen
- **AND** table SHALL be sortable by any column
- **AND** entries SHALL be ranked starting from 1

#### Scenario: Empty hotspot report shows empty state

- **GIVEN** hotspot API returns report with zero entries
- **WHEN** SPA loads
- **THEN** it SHALL display empty-state message indicating no hotspot data is available
- **AND** it SHALL NOT show empty table

#### Scenario: Hotspot API error shows error state

- **GIVEN** hotspot API returns `404` or `502`
- **WHEN** SPA loads
- **THEN** it SHALL display error message explaining data source could not be read
- **AND** it SHALL provide retry action that re-fetches data

### Requirement: Dashboard SPA provides per-subject drill-down

Clicking hotspot entry SHALL navigate to detail view for that subject, showing session history, event counts by type, and timeline.

#### Scenario: Subject detail view shows event breakdown by type

- **GIVEN** hotspot entry has evidence with multiple event types
- **WHEN** user clicks entry subject name
- **THEN** detail view SHALL display subject name and kind
- **AND** visualization or table SHALL show counts per event type (`FileOpened`, `SearchRun`, `SymbolInspected`, `CommandExecuted`, `DocRetrieved`, `EditMade`, `LookupErrored`)
- **AND** total event count and session count SHALL be displayed

#### Scenario: Subject detail view shows session timeline

- **GIVEN** hotspot entry has events from multiple sessions
- **WHEN** user views subject detail
- **THEN** timeline SHALL display each session as horizontal bar with event markers or equivalent semantic visualization
- **AND** each session SHALL show start time and source

### Requirement: Dashboard SPA provides session view

SPA SHALL have dedicated sessions view that lists recent sessions and allows navigation to session detail.

#### Scenario: Sessions view lists recent sessions

- **GIVEN** sessions API returns session data
- **WHEN** user navigates to `/sessions`
- **THEN** list of sessions SHALL be displayed ordered by start time, most recent first
- **AND** each session SHALL show truncated session ID, start time, end time or `Active`, event count, and source

#### Scenario: Session detail shows events within session

- **GIVEN** session ID
- **WHEN** user clicks session from list
- **THEN** SPA navigates to `/sessions/:sessionId`
- **AND** it SHALL display all events for that session in scrollable list
- **AND** each event SHALL show event type, timestamp, subject, and truncated payload preview

### Requirement: Dashboard SPA provides event distribution view

SPA SHALL have dedicated events view showing aggregate event distribution across captured traces.

#### Scenario: Event distribution shows design-system-compatible visualization by event type

- **WHEN** user navigates to `/events`
- **THEN** SPA SHALL display visualization or table showing event counts grouped by `event_type`
- **AND** colors, borders, labels, and typography SHALL come from semantic dashboard tokens
- **AND** loading, empty, and error states SHALL use shared shadcn-vue-compatible components

#### Scenario: Event distribution can be filtered by session

- **GIVEN** events view is loaded
- **WHEN** user selects session from dropdown filter
- **THEN** visualization SHALL update to show only events from that session
- **AND** clear-filter option SHALL be available

### Requirement: SPA navigation uses Vue Router with distinct routes

SPA SHALL use client-side routing with named routes for each view.

#### Scenario: Routes are defined for each view

- **WHEN** SPA is loaded
- **THEN** following routes SHALL be registered:
  - `/` — Hotspot report
  - `/subjects/:subjectKind/:subject` — Subject detail view
  - `/sessions` — Session list view
  - `/sessions/:sessionId` — Session detail view
  - `/events` — Event distribution view
  - `/about` — Dashboard version and documentation

#### Scenario: Unknown route shows 404 page

- **WHEN** user navigates to unrecognized path
- **THEN** SPA SHALL display `Page not found`
- **AND** it SHALL provide link back to landing page

### Requirement: SPA is built with Vue 3 and Vite

Frontend SHALL be implemented as Vue 3 SPA using Composition API, built with Vite, checked and built with Bun, and embedded in Rust binary at compile time.

#### Scenario: Production build outputs to frontend/dist/

- **WHEN** `bun run build` is executed in `crates/scryrs-dashboard/frontend/`
- **THEN** output SHALL be written to `crates/scryrs-dashboard/frontend/dist/`
- **AND** output SHALL include `index.html` and `assets/` directory with hashed JS and CSS files

#### Scenario: Build is embeddable in Rust binary

- **WHEN** `cargo build` compiles `scryrs-dashboard` crate
- **THEN** build script SHALL invoke Bun install and frontend build if `dist/` directory is missing or stale
- **AND** resulting `dist/` files SHALL be embedded using `rust-embed`

### Requirement: Dashboard styling and composition follow shadcn-vue rules

Dashboard styling SHALL use Tailwind CSS v4 semantic tokens and shadcn-vue conventions. Shared layout and feature views SHALL compose shadcn-vue primitives before using custom markup.

#### Scenario: Common UI primitives use shadcn-vue components

- **WHEN** dashboard view needs cards, tables, buttons, badges, alerts, empty states, skeletons, separators, tabs, select controls, tooltips, dialogs, or sheets
- **THEN** it SHALL use components from `@/shared/ui` when available
- **AND** missing primitives SHALL be installed through documented shadcn-vue CLI workflow rather than copied manually

#### Scenario: Styling uses semantic tokens and cn utility

- **WHEN** dashboard templates and shared components are inspected
- **THEN** status, event-kind, background, foreground, border, muted, destructive, warning, success, and info styling SHALL use semantic classes or CSS variables
- **AND** conditional merge-sensitive class names SHALL use `cn()` from `@/shared/lib/utils`
- **AND** normal dashboard styling SHALL NOT rely on raw color utility values

### Requirement: SPA architecture has extensible component slots for future phases

Component tree SHALL be structured so graph and route views (Phase 4+) can be added as new route components without modifying existing dashboard components.

#### Scenario: New views require only new route registration

- **GIVEN** later implementation adds graph visualization
- **WHEN** developer adds new route component under `src/views/GraphView.vue`
- **THEN** route SHALL be registered in router configuration
- **AND** navigation link SHALL be added in sidebar or nav component
- **AND** existing feature view components SHALL NOT require structural rewrites
- **AND** API client module SHALL allow adding new endpoint calls without changing existing calls

#### Scenario: Shared components are reusable

- **WHEN** new view needs data table, visualization card, or filter control
- **THEN** existing shared component library SHALL be used
- **AND** shared component SHALL accept props for customization

#### Scenario: Shared shell owns navigation and page frame

- **WHEN** dashboard layout files are inspected
- **THEN** sidebar, navigation, and page-frame concerns SHALL live in shared layout components
- **AND** feature views SHALL render inside that shell through Vue Router
