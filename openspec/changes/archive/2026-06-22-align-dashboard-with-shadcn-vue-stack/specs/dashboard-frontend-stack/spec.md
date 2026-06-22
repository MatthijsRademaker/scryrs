## ADDED Requirements

### Requirement: Dashboard frontend uses the shadcn-vue stack
The Phase 3 dashboard frontend SHALL be implemented with Vue 3, Vite, strict TypeScript, Bun, Tailwind CSS v4, shadcn-vue, Reka UI primitives, lucide-vue icons, Vue Router, and Pinia. The dashboard frontend SHALL NOT use a custom-CSS-only component approach as its primary UI architecture.

#### Scenario: Frontend package declares the canonical stack
- **WHEN** `crates/scryrs-dashboard/frontend/package.json` is inspected
- **THEN** it declares Vue 3, Vite, TypeScript, Vue Router, Pinia, Tailwind CSS v4, shadcn-vue, Reka UI, lucide-vue, class-variance-authority, clsx, and tailwind-merge dependencies or devDependencies
- **AND** it does not declare an alternate UI component framework as the primary dashboard component system

#### Scenario: Vite uses Vue and Tailwind v4 plugins
- **WHEN** `crates/scryrs-dashboard/frontend/vite.config.ts` is inspected
- **THEN** it configures `@vitejs/plugin-vue`
- **AND** it configures `@tailwindcss/vite`
- **AND** it defines the `@` alias to the frontend `src` directory

#### Scenario: shadcn-vue configuration uses shared aliases
- **WHEN** `crates/scryrs-dashboard/frontend/components.json` is inspected
- **THEN** it configures shadcn-vue for a Vite + TypeScript project
- **AND** the `ui` alias resolves under `@/shared/ui`
- **AND** the `utils` alias resolves under `@/shared/lib/utils`
- **AND** the configured icon library is lucide

### Requirement: Dashboard frontend uses Bun exclusively
The dashboard frontend SHALL use Bun as its package manager and script runner. The dashboard frontend SHALL commit a Bun lockfile and SHALL NOT commit npm, pnpm, or yarn lockfiles for the dashboard package.

#### Scenario: Dashboard frontend has Bun lockfile only
- **WHEN** the dashboard frontend scaffold is present
- **THEN** `crates/scryrs-dashboard/frontend/bun.lock` exists
- **AND** `crates/scryrs-dashboard/frontend/package-lock.json` does not exist
- **AND** `crates/scryrs-dashboard/frontend/pnpm-lock.yaml` does not exist
- **AND** `crates/scryrs-dashboard/frontend/yarn.lock` does not exist

#### Scenario: Dashboard build commands use Bun
- **WHEN** dashboard build integration is inspected
- **THEN** dependency installation uses `bun install`
- **AND** production build uses `bun run build`
- **AND** type checking uses `bun run check`
- **AND** shadcn-vue CLI invocations use `bunx --bun shadcn-vue@latest`

### Requirement: TheGreatMigration is a curated donor reference
The dashboard implementation SHALL use `~/repos/personal/TheGreatMigration/frontend/` as a donor reference for infrastructure and design-system patterns only. It SHALL copy or adapt reusable stack files and shared UI primitives, but SHALL NOT copy moving-domain application code into scryrs.

#### Scenario: Allowed donor assets are reusable frontend foundation
- **WHEN** donor-derived frontend files are added to scryrs
- **THEN** they may include Vite config, TypeScript config, `components.json`, global Tailwind/design-token CSS, `shared/ui` primitives, `shared/lib/utils.ts`, and app-shell/sidebar patterns
- **AND** every copied file is adapted to scryrs names, routes, assets, and API contracts

#### Scenario: Rejected donor assets are not copied
- **WHEN** the dashboard frontend scaffold is inspected
- **THEN** it does not include TheGreatMigration moving-domain directories for tasks, calendar, people, rooms, settings, or moving-home views
- **AND** it does not include TheGreatMigration generated OpenAPI client output
- **AND** it does not include TheGreatMigration public brand images
- **AND** it does not include TheGreatMigration Dockerfile unless a separate containerization change explicitly requires it

#### Scenario: No stale donor branding remains
- **WHEN** dashboard UI source, routes, tests, and public assets are searched
- **THEN** they do not contain user-facing TheGreatMigration, moving, room, helper, task-backlog, or schedule-board domain labels except in comments that explicitly identify donor provenance during migration

### Requirement: Dashboard API client is hand-written and typed
The Phase 3 dashboard frontend SHALL call the local dashboard REST API through hand-written typed fetch functions. It SHALL NOT copy TheGreatMigration's generated OpenAPI client or generator scripts.

#### Scenario: API client targets scryrs dashboard endpoints
- **WHEN** the dashboard frontend API client is inspected
- **THEN** it exposes typed functions for `GET /api/hotspots`, `GET /api/sessions`, and `GET /api/events`
- **AND** its TypeScript types match the dashboard server response contracts
- **AND** it handles non-2xx responses by surfacing explicit typed errors to the calling view or store

#### Scenario: Donor generated client is absent
- **WHEN** the dashboard frontend tree is inspected
- **THEN** it does not contain TheGreatMigration `src/client/**` generated API files
- **AND** it does not contain OpenAPI generation scripts unless a later scryrs OpenAPI capability is explicitly added

### Requirement: Charting fits the design system
The dashboard SHALL present event distribution and timeline visualizations using components that fit the shadcn-vue/Tailwind design system. Chart.js SHALL NOT be a mandatory dependency for Phase 3 unless implementation explicitly wraps it behind shadcn-vue-style components and semantic tokens.

#### Scenario: Event distribution remains available
- **WHEN** a user navigates to the dashboard events view
- **THEN** the UI displays event counts grouped by event type
- **AND** the visualization uses semantic dashboard tokens for colors and typography
- **AND** loading, empty, and error states use shared shadcn-vue-compatible components

#### Scenario: Chart dependency is justified if introduced
- **WHEN** a charting package such as Chart.js is added to the dashboard frontend
- **THEN** the implementation wraps it behind reusable dashboard chart components
- **AND** chart colors are mapped from semantic CSS variables
- **AND** the package is not used directly from view components in a way that bypasses the design system

### Requirement: Dashboard layout remains extensible for later graph and route views
The dashboard frontend SHALL keep route registration, navigation, layout, API client modules, and shared UI components separated so future graph and route views can be added without rewriting existing hotspot/session/event views.

#### Scenario: Future view requires isolated additions
- **WHEN** a future graph view is added
- **THEN** implementation only needs to add a route component, register a route, add a navigation item, and add any new API client function
- **AND** existing hotspot, session, event, and subject view components do not need structural rewrites

#### Scenario: Shared shell owns navigation and page frame
- **WHEN** dashboard layout files are inspected
- **THEN** sidebar/navigation and page frame concerns live in shared layout components
- **AND** feature views render inside that shell through Vue Router
