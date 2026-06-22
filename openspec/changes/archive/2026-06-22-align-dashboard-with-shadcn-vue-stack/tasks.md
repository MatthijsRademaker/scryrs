## 1. Reconcile Active Dashboard Change

- [x] 1.1 Update `openspec/changes/task-53fc3cbb-ac15-4ad1-9e18-dc510931c683/proposal.md` to name Bun, Tailwind CSS v4, shadcn-vue, Reka UI, and lucide-vue as the dashboard frontend stack
- [x] 1.2 Update `openspec/changes/task-53fc3cbb-ac15-4ad1-9e18-dc510931c683/design.md` to remove the custom-CSS/no-component-library decision and replace it with the shadcn-vue design-system decision
- [x] 1.3 Update the active dashboard design to document TheGreatMigration as a curated donor reference, including explicit allowlist and denylist
- [x] 1.4 Update the active dashboard design to make Bun the only dashboard frontend package manager and remove npm-first build-script language
- [x] 1.5 Update the active dashboard design to make Chart.js optional rather than mandatory, with charting required to fit semantic Tailwind/shadcn-vue tokens

## 2. Reconcile Dashboard Frontend Specs

- [x] 2.1 Update `openspec/changes/task-53fc3cbb-ac15-4ad1-9e18-dc510931c683/specs/dashboard-frontend-app/spec.md` so the frontend stack requirement includes Vue 3, Vite, strict TypeScript, Bun, Tailwind CSS v4, shadcn-vue, Reka UI, lucide-vue, Vue Router, and Pinia
- [x] 2.2 Add frontend package-manager scenarios requiring `bun.lock`, `bun install`, `bun run build`, `bun run check`, and `bunx --bun shadcn-vue@latest`
- [x] 2.3 Add donor-boundary scenarios requiring reusable infrastructure only and rejecting TheGreatMigration domain pages, generated client files, public images, Dockerfile, and npm lockfiles
- [x] 2.4 Add typed hand-written API client scenarios for `/api/hotspots`, `/api/sessions`, and `/api/events`
- [x] 2.5 Replace Chart.js-specific frontend scenarios with design-system-compatible event distribution and timeline visualization scenarios
- [x] 2.6 Add shadcn-vue composition, semantic token, and shared layout extensibility scenarios to the dashboard frontend spec

## 3. Reconcile Dashboard Implementation Tasks

- [x] 3.1 Update active dashboard task 5.1 to scaffold from the shadcn-vue/Tailwind/Bun stack instead of a generic Vue/Vite/Chart.js stack
- [x] 3.2 Replace active dashboard task 5.14 custom-CSS-only styling with Tailwind CSS v4 semantic design tokens and shadcn-vue component composition
- [x] 3.3 Add tasks for curated donor import from TheGreatMigration and explicit cleanup of stale donor domain references
- [x] 3.4 Add tasks for creating `components.json`, `src/shared/ui/**`, `src/shared/lib/utils.ts`, and `src/app/styles.css` in the dashboard frontend
- [x] 3.5 Add tasks for hand-written typed API client functions and remove any task implying generated OpenAPI client adoption
- [x] 3.6 Update build integration tasks to use Bun commands and require only `bun.lock` for the dashboard frontend

## 4. Add Project-Local shadcn-vue Guidance

- [x] 4.1 Add `.pi/skills/shadcn-vue/SKILL.md` or another documented local instruction surface adapted from TheGreatMigration's shadcn-vue guidance
- [x] 4.2 Add supporting shadcn-vue CLI, customization, styling, composition, forms, and icons rule files as needed for dashboard work
- [x] 4.3 Replace donor-specific package-manager ambiguity with Bun-only commands in the guidance
- [x] 4.4 Replace donor-specific app paths, examples, and moving-domain labels with scryrs dashboard context
- [x] 4.5 Document that agents must read shadcn-vue guidance before adding or modifying dashboard UI components

## 5. Validation

- [x] 5.1 Run `openspec status --change align-dashboard-with-shadcn-vue-stack` and confirm proposal, design, specs, and tasks are complete
- [x] 5.2 Run `openspec status --change task-53fc3cbb-ac15-4ad1-9e18-dc510931c683` and confirm the active dashboard change remains apply-ready after artifact reconciliation
- [x] 5.3 Search updated dashboard artifacts for stale phrases: `custom CSS`, `no external UI component library`, `npm ci`, `npm run build`, and mandatory `Chart.js`
- [x] 5.4 Search project-local shadcn-vue guidance for stale donor labels: `The Great Migration`, `moving`, `tasks`, `calendar`, `people`, and `rooms`
- [x] 5.5 Run `openspec validate align-dashboard-with-shadcn-vue-stack --strict` if supported by the local OpenSpec CLI; otherwise run available OpenSpec validation/status checks
