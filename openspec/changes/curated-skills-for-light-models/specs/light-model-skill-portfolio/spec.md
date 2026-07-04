# light-model-skill-portfolio

## ADDED Requirements

### Requirement: Initial recipe portfolio exists in curated format
The following skills SHALL exist in `.pi/skills/`, each conforming to the curated-skill-format capability, with every referenced path verified against the current tree: `crate-map` (orientation map), `add-cli-command`, `add-adapter`, `dashboard-ui`, `graph-core-changes`, `testing-this-repo`, `pi-hook-changes` (recipes).

#### Scenario: Portfolio skills pass lint
- **WHEN** `scripts/skill-lint` runs after the portfolio lands
- **THEN** all seven skills pass with zero missing-path findings

#### Scenario: Recipe skills follow the format
- **WHEN** any portfolio recipe skill is inspected
- **THEN** it contains Read first, Steps with Verify lines, Easy-to-miss details, and Self-check sections, plus curated provenance frontmatter

### Requirement: Skills encode known repo constraints
`pi-hook-changes` SHALL encode the hook source-ownership rule (edit `hooks/pi/index.ts` only; never edit the installed `.pi/extensions/scryrs/index.ts` copy; refresh via `scryrs init --agent pi`). `dashboard-ui` SHALL encode the Vue 3 constraint that `lucide-vue` is incompatible and inline SVG icons in the shared icon directory are used instead, and SHALL state its routing border with the generic `frontend-design` and `shadcn-vue` guidance.

#### Scenario: Hook skill prevents editing installed copy
- **WHEN** `pi-hook-changes` is read
- **THEN** it explicitly forbids editing `.pi/extensions/scryrs/index.ts` and gives the refresh command

#### Scenario: Dashboard skill encodes icon constraint
- **WHEN** `dashboard-ui` is read
- **THEN** it states that `lucide-vue` must not be imported and names the inline SVG icon location

### Requirement: Stale and duplicate skills are removed
`.pi/skills/workflow-taskflow-expert/` SHALL be deleted. `read-project-docs` and `project-docs` SHALL be merged into a single `project-docs` skill, with all references updated.

#### Scenario: Stale Go-era skill is gone
- **WHEN** `.pi/skills/` is listed after the change
- **THEN** `workflow-taskflow-expert/` does not exist

#### Scenario: Docs skill is singular
- **WHEN** `.pi/skills/` is listed after the change
- **THEN** exactly one of `project-docs`/`read-project-docs` exists, and no agent frontmatter references the removed name

### Requirement: Agent skill lists route to the portfolio and stay short
Each `.pi/agents/*.md` `skills:` list SHALL be updated to reference the relevant portfolio skills, SHALL contain no references to removed skills, and SHALL be kept small (target ≤ 8 skills per agent) so light-model skill selection stays reliable.

#### Scenario: Worker routes to recipes
- **WHEN** `swarm-worker.md` frontmatter is inspected after the change
- **THEN** its `skills:` list includes the portfolio skills relevant to generalist work, excludes removed skill names, and contains at most 8 entries

#### Scenario: Routing flips atomically
- **WHEN** the agent `skills:` list updates land
- **THEN** they land as a single commit separable from skill authoring, so a revert restores prior routing without touching skill content
