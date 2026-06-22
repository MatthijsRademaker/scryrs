# shadcn-vue-dashboard-guidance Specification

## Purpose
TBD - created by archiving change align-dashboard-with-shadcn-vue-stack. Update Purpose after archive.
## Requirements
### Requirement: Project contains shadcn-vue dashboard guidance
The project SHALL provide local agent guidance for dashboard UI work with shadcn-vue. The guidance SHALL be discoverable from the repository and SHALL cover CLI usage, component selection, styling, composition, forms, icons, theming, and update workflow.

#### Scenario: Guidance is present in project-local instructions
- **WHEN** an agent prepares to modify dashboard shadcn-vue UI
- **THEN** project-local guidance exists under `.pi/skills/shadcn-vue/` or another documented local instruction path
- **AND** it identifies the dashboard frontend as a shadcn-vue Vite project using Bun
- **AND** it tells agents to use `bunx --bun shadcn-vue@latest` for shadcn-vue commands

#### Scenario: Guidance is adapted from donor repo
- **WHEN** the guidance content is inspected
- **THEN** it preserves TheGreatMigration's useful shadcn-vue rules for CLI, theming, styling, forms, composition, and icons
- **AND** it replaces donor-specific paths, package-manager ambiguity, examples, and domain labels with scryrs dashboard context

### Requirement: Agents use shadcn-vue components before custom UI
Dashboard UI work SHALL prefer installed or installable shadcn-vue components before creating custom styled markup. Custom dashboard components SHALL compose shadcn-vue primitives and shared semantic tokens.

#### Scenario: Common UI primitives use shadcn-vue
- **WHEN** a dashboard view needs cards, tables, buttons, badges, alerts, empty states, skeletons, separators, tabs, select controls, sidebars, tooltips, dialogs, or sheets
- **THEN** it uses shadcn-vue components from `@/shared/ui` when available
- **AND** it installs missing shadcn-vue components through the documented CLI workflow rather than hand-copying registry files

#### Scenario: Custom wrappers remain thin
- **WHEN** a domain-specific component such as `HotspotScoreCard` or `EventTypeBadge` is created
- **THEN** it composes shadcn-vue primitives or existing shared dashboard components
- **AND** it does not duplicate lower-level button, card, badge, table, dialog, or sidebar behavior

### Requirement: Styling uses semantic tokens and shadcn-vue rules
Dashboard styling SHALL use Tailwind CSS v4 semantic tokens and shadcn-vue conventions. It SHALL avoid raw colors, manual dark-mode overrides, `space-x-*`/`space-y-*`, manual z-index on overlays, and component color overrides through arbitrary classes.

#### Scenario: Semantic colors are used
- **WHEN** dashboard component templates are inspected
- **THEN** status, event-kind, chart, background, foreground, border, muted, destructive, warning, success, and info styling use semantic classes or CSS variables
- **AND** raw color utilities such as `bg-blue-500`, `text-green-600`, or `dark:bg-gray-950` are not used for normal dashboard styling

#### Scenario: Layout spacing follows guidance
- **WHEN** dashboard component templates are inspected
- **THEN** repeated child spacing uses `gap-*` with flex or grid containers
- **AND** equal width/height elements use `size-*` where applicable
- **AND** truncation uses `truncate` where applicable

#### Scenario: Conditional classes use cn utility
- **WHEN** dashboard components compute conditional class names
- **THEN** they use the shared `cn()` utility from `@/shared/lib/utils`
- **AND** they do not rely on long hand-built template-literal class expressions for merge-sensitive Tailwind classes

### Requirement: Component composition follows shadcn-vue accessibility rules
Dashboard components SHALL follow shadcn-vue composition and accessibility rules for grouped items, dialogs, sheets, cards, avatars, tabs, loading states, empty states, and icons.

#### Scenario: Cards use full card composition
- **WHEN** a dashboard card component is inspected
- **THEN** it uses `Card`, `CardHeader`, `CardTitle`, `CardDescription`, `CardContent`, and `CardFooter` as appropriate
- **AND** it does not dump all content into `CardContent` when header or footer semantics are needed

#### Scenario: Overlay components include titles
- **WHEN** a dashboard dialog, sheet, or drawer is used
- **THEN** it includes the required title component for accessibility
- **AND** visually hidden titles use `sr-only` rather than being omitted

#### Scenario: Grouped menu and select items use groups
- **WHEN** select, dropdown, menubar, context menu, or command items are used
- **THEN** item components are rendered inside the corresponding group component where the shadcn-vue primitive expects grouping

#### Scenario: Icons follow configured icon-library rules
- **WHEN** dashboard icons are imported
- **THEN** they come from the configured icon library
- **AND** icons inside buttons use `data-icon="inline-start"` or `data-icon="inline-end"`
- **AND** icons inside shadcn-vue components do not add manual sizing classes unless custom sizing is explicitly required

### Requirement: shadcn-vue CLI and component updates follow safe workflow
Dashboard component installation and updates SHALL use the shadcn-vue CLI and project configuration. Registry files SHALL NOT be manually fetched from GitHub or raw URLs.

#### Scenario: Component installation uses CLI
- **WHEN** a dashboard implementation needs a new shadcn-vue component
- **THEN** the agent checks existing installed components first
- **AND** uses `bunx --bun shadcn-vue@latest add <component>` from the dashboard frontend directory when installation is needed
- **AND** reviews added files for imports, composition, dependencies, and TypeScript correctness

#### Scenario: Upstream component updates are previewed
- **WHEN** an installed shadcn-vue component is updated from upstream
- **THEN** the agent previews changes with the documented dry-run or diff workflow
- **AND** preserves local modifications unless the user explicitly approves overwrite

