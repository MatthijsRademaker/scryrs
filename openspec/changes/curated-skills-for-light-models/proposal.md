# Curated Skills for Light Models

## Why

Swarm agents on lighter models (`modelEasy`/`modelModerate` tiers) fail in a specific way: they are lazy, make assumptions, and glance over important details — and they treat skills as ground truth, so a stale skill is worse than no skill. The repo already shipped proof: `.pi/skills/workflow-taskflow-expert/` describes Go files (`src/manager/...`) that have never existed in this Rust workspace, and `.pi/skills/project-docs/` and `.pi/skills/read-project-docs/` are near-duplicates. There is currently no convention that keeps repo-specific skills true, no recipe-shaped skills for the recurring task shapes light models actually work on, and no routing discipline in skill descriptions.

## What Changes

- Define a **curated recipe skill format**: frontmatter with `sources:` globs and `last-verified` date, a routing-grade description (`Use when: … Do not use for: …`), and a body structure of mandatory read-first list, steps with per-step verify commands, easy-to-miss details, and a self-check gate before `report_work_outcome`.
- Add a **skill lint script** (`scripts/skill-lint`) that extracts referenced repo paths from `.pi/skills/*/SKILL.md` and fails when a referenced path does not exist. Wired into pre-commit so staleness is caught at commit time, not at the next curation pass.
- Add a **`/curate-skills` skill** run by a frontier model: for each curated skill, detect drift via `git log --since <last-verified> -- <sources>`; no-op (bump date) when clean, re-verify every claim against source and rewrite drifted sections when dirty.
- Author the initial **recipe skill portfolio**: `crate-map`, `add-cli-command`, `add-adapter`, `dashboard-ui`, `graph-core-changes`, `testing-this-repo`, `pi-hook-changes` — each in the curated format with accurate paths verified against the current tree.
- **Removals**: delete `.pi/skills/workflow-taskflow-expert/` (stale copy from swarm init referencing a nonexistent Go codebase); merge `read-project-docs` into `project-docs` and keep a single skill.
- Update agent `skills:` lists in `.pi/agents/*.md` to reference the new portfolio, keeping each agent's list short (routing quality degrades with skill count on light models).

## Capabilities

### New Capabilities

- `curated-skill-format`: The authoring convention for repo-specific skills — frontmatter contract (`sources`, `last-verified`), routing description format, and required body sections (read-first, verified steps, easy-to-miss details, self-check gate).
- `skill-lint`: Static validation that every repo path referenced by a skill exists in the working tree; runs in pre-commit and CI.
- `skill-curation`: The frontier-model refresh loop — drift detection from `sources` globs and git history, claim re-verification, section rewrite, and `last-verified` bump.
- `light-model-skill-portfolio`: The concrete set of recipe skills for this repo's recurring task shapes, plus removal of stale/duplicate skills and agent skill-list routing updates.

### Modified Capabilities

<!-- none — no existing spec covers skill authoring, linting, or curation -->

## Impact

- `.pi/skills/` — new skill directories, one deletion (`workflow-taskflow-expert`), one merge (`read-project-docs` → `project-docs`).
- `scripts/` — new `skill-lint` script (must be host-runnable without Rust/Node SDKs, per agent-verification rules; shell or Docker-backed).
- `.pre-commit-config.yaml` — new hook entry for skill-lint.
- `.pi/agents/*.md` — `skills:` frontmatter lists updated to the new portfolio.
- `AGENTS.md` / `CLAUDE.md` — pointer to the curated-skill convention so future skills follow it.
- No application code (`crates/`) is affected.
