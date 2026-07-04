# Tasks — Curated Skills for Light Models

## 1. Convention & lint (the floor)

- [x] 1.1 Write the curated-skill convention doc (frontmatter contract with `curated`/`sources`/`last-verified`/`verified-commit`, `Use when:`/`Do not use for:` description format, recipe body sections: Read first, Steps with Verify lines, Easy-to-miss details, Self-check) and link it from `AGENTS.md`
- [x] 1.2 Implement `scripts/skill-lint` in plain bash: extract repo-relative path tokens (crates/, scripts/, hooks/, tests/, .pi/, .devagent/, openspec/, xtask/, examples/, fuzz/) from `.pi/skills/*/SKILL.md`; skip tokens with `<`/`>`/`*` and lines with `skill-lint-ignore`; validate `sources` globs match ≥1 file; exit non-zero naming skill file + offending token
- [x] 1.3 Register skill-lint as a local hook in `.pre-commit-config.yaml` and confirm it executes via `scripts/precommit-run`
- [x] 1.4 Verify the lint catches the known-stale case: run it against the current tree and confirm it fails on `workflow-taskflow-expert`'s Go paths before that skill is removed

## 2. Removals & merge

- [x] 2.1 Delete `.pi/skills/workflow-taskflow-expert/`
- [x] 2.2 Merge `read-project-docs` into `project-docs` (single directory, best content of both) and remove the duplicate
- [x] 2.3 Grep `.pi/agents/*.md` and any other references for `read-project-docs`/`workflow-taskflow-expert` and update or remove them

## 3. Portfolio authoring (each skill: verify paths against tree, stamp verified-commit, pass skill-lint)

- [ ] 3.1 Author `crate-map` — one line per crate from `Cargo.toml`/`crates/*/Cargo.toml`, who-calls-whom; orientation only, no task steps
- [ ] 3.2 Author `add-cli-command` recipe from `crates/scryrs-cli/src/**` (locate real command registration point, existing command to mirror, and test pattern; every step gets a Verify line)
- [ ] 3.3 Author `add-adapter` recipe from `crates/scryrs-adapter-*/**` and the adapter trait in `crates/scryrs-core/**` (use an existing adapter crate as the template reference)
- [ ] 3.4 Author `dashboard-ui` recipe from `crates/scryrs-dashboard/frontend/**` — encode Vue 3 / no `lucide-vue` / inline SVG icon location; state routing border with `shadcn-vue` and `frontend-design` in the description
- [ ] 3.5 Author `graph-core-changes` recipe from `crates/scryrs-graph/**`, `crates/scryrs-core/**`, `crates/scryrs-types/**` (cross-reference `.devagent/docs/docs/graph.md` for claims, prefer executable truth in crates)
- [ ] 3.6 Author `testing-this-repo` recipe from `tests/**`, `scripts/test`, `.docker-fixtures/**` — Docker-backed verification commands only, explicit "do not run cargo on host" border with generic `tdd`
- [ ] 3.7 Author `pi-hook-changes` recipe from `hooks/pi/**` — encode source-ownership rule (never edit `.pi/extensions/scryrs/index.ts`; refresh via `scryrs init --agent pi`)
- [ ] 3.8 Seed each recipe's Easy-to-miss details from known constraints (AGENTS.md rules, `.pi/rules/*`, memory notes, recent review findings where available)

## 4. Curation loop (the ceiling)

- [ ] 4.1 Author `.pi/skills/curate-skills/SKILL.md`: iterate curated skills; dirty check via `git log <verified-commit>..HEAD -- <sources>`; no-op bump when clean; full claim re-verification + append-biased rewrite when dirty; glob-coverage sanity check; output as reviewed diff, never self-merged
- [ ] 4.2 Run one manual curation pass with a frontier model over the new portfolio to validate the no-op path and marker bumping

## 5. Routing flip & verification

- [ ] 5.1 Update `skills:` lists in `.pi/agents/*.md` to the portfolio (≤ 8 skills per agent, no removed names) as a single dedicated commit
- [ ] 5.2 Run `scripts/skill-lint` and `scripts/precommit-run` clean across the final tree
- [ ] 5.3 Verify all spec scenarios: portfolio skills conform to format sections, descriptions carry both routing clauses, sources globs all match files, exactly one project-docs skill exists
