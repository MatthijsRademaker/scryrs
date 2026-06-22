## Why

The planned Phase 3 dashboard currently specifies Vue/Vite/Pinia but rejects a component library and assumes npm + custom CSS. That conflicts with the desired donor stack from `~/repos/personal/TheGreatMigration/`, where dashboard UI is built around Vue 3, Vite, Bun, Tailwind CSS v4, shadcn-vue, Reka UI primitives, lucide icons, and explicit shadcn-vue usage rules.

This change aligns the Phase 3 dashboard frontend contract before implementation starts, so the dashboard is built on a coherent design-system stack instead of first shipping custom CSS and later replacing it.

## What Changes

- Replace the Phase 3 dashboard frontend stack with Vue 3 + Vite + TypeScript + Bun + Tailwind CSS v4 + shadcn-vue + Reka UI + lucide-vue.
- Adopt TheGreatMigration as a donor reference for frontend infrastructure and design-system conventions, not as a wholesale application transplant.
- Add a curated copy strategy: copy reusable app-shell, `components.json`, Tailwind/design-token setup, `shared/ui`, and shadcn-vue skill/rules; reject moving-domain views, generated OpenAPI client, images, Dockerfile, and package-lock artifacts.
- Replace the current "no component library" direction with a shadcn-vue-first component architecture.
- Use Bun for frontend dependency install, lockfile, checks, and build integration.
- Prefer typed hand-written REST API client calls for the Phase 3 dashboard API instead of importing TheGreatMigration's generated OpenAPI client.
- Reconsider Chart.js as a required dependency: charts must fit the shadcn-vue/Tailwind design system and may use shadcn-vue-compatible chart components or simple semantic chart primitives in Phase 3.
- Capture project-local shadcn-vue agent guidance so future dashboard UI work follows the same composition, styling, form, and icon rules.

## Capabilities

### New Capabilities

- `dashboard-frontend-stack`: Defines the Phase 3 dashboard frontend technology stack, package-manager contract, donor-repo reuse boundary, design-system requirements, and charting direction.
- `shadcn-vue-dashboard-guidance`: Defines the local guidance and rules agents must follow when adding or modifying shadcn-vue dashboard UI components.

### Modified Capabilities

- (none — the dashboard frontend capability is not archived in `openspec/specs/`; this change establishes the stack contract before implementation.)

## Impact

- **Active dashboard change artifacts**: Supersedes the custom-CSS/no-component-library parts of `openspec/changes/task-53fc3cbb-ac15-4ad1-9e18-dc510931c683/design.md`, `specs/dashboard-frontend-app/spec.md`, and `tasks.md` when Phase 3 implementation proceeds.
- **Frontend source**: Future `crates/scryrs-dashboard/frontend/` scaffold uses Bun, Tailwind CSS v4, shadcn-vue, Reka UI, lucide-vue, Vue Router, Pinia, and strict TypeScript.
- **Build integration**: Future dashboard crate build scripts use Bun commands (`bun install`, `bun run build`, `bun run check`) rather than npm.
- **Agent guidance**: Project should include shadcn-vue dashboard rules derived from TheGreatMigration's `.agents/skills/shadcn-vue/` guidance, adapted to this repository's `.pi/skills/` or equivalent instruction surface.
- **Dependencies**: Adds frontend dependencies for shadcn-vue/reka/lucide/tailwind utilities; removes planned dependency on a custom-only CSS approach and avoids importing donor app-domain dependencies.
