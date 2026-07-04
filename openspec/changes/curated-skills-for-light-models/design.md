# Design — Curated Skills for Light Models

## Context

Swarm agents run headless in Docker with difficulty-based model routing (`modelEasy`/`modelModerate`/`modelComplex` in `.pi/agents/*.md`). Lighter-tier models treat skills as ground truth and fail by skipping verification: laziness, unstated assumptions, glossed-over details. Repo-specific skills embed paths and symbols that rot silently — `workflow-taskflow-expert` still describes a Go codebase that predates this Rust workspace, and no mechanism exists to catch that.

Existing assets this design builds on: `.pi/skills/` (18 skills, symlinked into `.claude/skills/`), `scripts/` Docker-backed verification, `.pre-commit-config.yaml`, and the `swarm-eval-loop` precedent of a strong model improving artifacts consumed by weaker ones.

Constraints: host has no Rust/Node/Python guarantees for agents (per `.pi/rules/agent-verification.md`), so any lint that runs in agent context must be plain shell or Docker-backed. Skills are markdown consumed verbatim; there is no runtime to enforce structure, only convention + lint.

## Goals / Non-Goals

**Goals:**

- Repo-specific skills that make light models perform recurring task shapes correctly without judgment calls.
- Staleness is caught mechanically (lint at commit time) and repaired systematically (curation pass by a frontier model).
- Skill descriptions function as a reliable router for weak trigger-matchers.

**Non-Goals:**

- No changes to generic tool skills (`tdd`, `github-cli`, `mcp-builder`, `playwright-cli`, …) beyond lint coverage.
- No automated/scheduled curation daemon in this change — `/curate-skills` is manually invoked; scheduling is a follow-up once trusted.
- No symbol-level (type/function name) lint verification in v1 — paths only.
- No changes to the swarm runtime, model routing, or `crates/` application code.

## Decisions

### D1 — Curated skills declare provenance in frontmatter

Curated skills carry a `metadata` block:

```yaml
metadata:
  curated: true
  sources:
    - crates/scryrs-cli/src/**
  last-verified: 2026-07-04
  verified-commit: <sha>
```

`sources` globs are the contract that makes drift detection mechanical: `git log <verified-commit>..HEAD -- <sources>` non-empty ⇒ dirty. The commit SHA is the authoritative marker (dates are ambiguous across rebases); `last-verified` is kept as the human-readable companion.

*Alternative considered:* no provenance, curator re-reads everything every pass — rejected: makes curation expensive enough that it won't be run, and gives no cheap no-op path.

### D2 — Recipe body structure with per-step exit conditions

Required sections for curated recipe skills:

1. **Read first** — 1–3 files max, each with one line on why it matters. Small on purpose: a bounded reading list gets done; "explore the codebase" gets skipped.
2. **Steps** — exact paths and registration points; every step ends with a `Verify:` line containing a runnable command and its expected observation (e.g., "`grep -n 'pub mod' crates/scryrs-cli/src/commands.rs` must show your module").
3. **Easy-to-miss details** — 3–5 concrete gotchas for this task shape, harvested from review findings over time.
4. **Self-check** — yes/no list to pass before calling `report_work_outcome`.

Rationale: the observed failure mode is skipped verification, not missing knowledge. Verification expressed as prose is optional to a lazy model; expressed as a command with an expected result, it is a step. This mirrors the repo's existing verification culture (`scripts/precommit-run` as exact command, explicit "Do not use" lists).

*Alternative considered:* map-style domain-expert skills (like the dead `workflow-taskflow-expert`) — rejected as the primary shape: maps leverage reasoning the light tiers don't reliably apply. One map skill (`crate-map`) is kept for orientation only.

### D3 — Description format is the router

Every curated skill description follows: `<one sentence: capability>. Use when: <literal paths, symbols, task phrasings>. Do not use for: <adjacent skill's territory, by name>.`

Literal file paths in descriptions are the strongest trigger signal because swarm task prompts usually contain paths. The `Do not use for` clause draws explicit borders between adjacent skills (`dashboard-ui` vs `frontend-design`) because light models over-trigger as readily as they under-trigger. Agent `skills:` lists in `.pi/agents/*.md` stay short — selection among ~6 well-bordered skills is reliable; among 18 it is not.

### D4 — `skill-lint` is a plain shell script, paths only

`scripts/skill-lint`: extracts repo-relative path tokens (prefixes `crates/`, `scripts/`, `hooks/`, `tests/`, `.pi/`, `.devagent/`, `openspec/`, `xtask/`, `examples/`, `fuzz/`) from every `.pi/skills/*/SKILL.md` and fails listing each token that does not exist in the working tree.

- Plain `bash` + `grep` so it runs on host and in agent containers without SDKs.
- Tokens containing `<`, `>`, or `*` are placeholders/globs and are skipped (globs from `sources:` are instead checked to match ≥1 file).
- An inline `skill-lint-ignore` marker on a line suppresses checking for intentional examples of nonexistent paths.
- Wired into `.pre-commit-config.yaml` as a local hook so it runs via the existing `scripts/precommit-run` path and in CI for free.

*Alternative considered:* symbol verification (grep type/function names into sources) — deferred: needs per-language heuristics and produces false positives; paths alone would have caught the `workflow-taskflow-expert` rot on day one.

### D5 — Curation is a skill, not infrastructure

`/curate-skills` is itself a skill in `.pi/skills/curate-skills/`, intended to be run by a frontier-tier model. Per curated skill:

1. Dirty check: `git log --oneline <verified-commit>..HEAD -- <sources>`; empty ⇒ bump `verified-commit`/`last-verified`, done.
2. Dirty ⇒ re-read the sources, verify every path, command, and claim in the skill, rewrite drifted sections, refresh the easy-to-miss list if review history surfaced new misses, bump markers.
3. Output is an ordinary diff reviewed like any change — the curator never self-merges.

Lean by design: no daemon, no pipeline. Promotion to a scheduled swarm task is a follow-up decision after a few manual passes build trust. The lint (D4) is the always-on floor; curation is the ceiling.

### D6 — Initial portfolio and removals

| Skill | Shape | Sources |
|---|---|---|
| `crate-map` | map (orientation only) | `Cargo.toml`, `crates/*/Cargo.toml` |
| `add-cli-command` | recipe | `crates/scryrs-cli/src/**` |
| `add-adapter` | recipe | `crates/scryrs-adapter-*/**`, `crates/scryrs-core/**` (adapter trait) |
| `dashboard-ui` | recipe | `crates/scryrs-dashboard/frontend/**` |
| `graph-core-changes` | recipe | `crates/scryrs-graph/**`, `crates/scryrs-core/**`, `crates/scryrs-types/**` |
| `testing-this-repo` | recipe | `tests/**`, `scripts/test`, `.docker-fixtures/**` |
| `pi-hook-changes` | recipe | `hooks/pi/**` |

Each is authored against the current tree (every path verified before commit) and stamped with the authoring commit as `verified-commit`. `dashboard-ui` absorbs the routing border with `shadcn-vue` and encodes the lucide-vue/Vue-3 constraint; `pi-hook-changes` encodes the source-ownership rule (never edit `.pi/extensions/scryrs/index.ts`).

Removals: delete `.pi/skills/workflow-taskflow-expert/`; merge `read-project-docs` into `project-docs` (keep one directory, update agent `skills:` references).

## Risks / Trade-offs

- [Lint false positives on example paths] → placeholder/glob token skipping plus `skill-lint-ignore` marker; lint output names the offending token and file so fixes are one-line.
- [Curator rewrites degrade a skill (loses hard-won gotchas)] → curation output is a reviewed diff, never auto-merged; easy-to-miss sections are append-biased (curator instructed to update, not prune, unless the gotcha's source is gone).
- [Recipes over-fit and constrain valid alternative approaches] → accepted trade-off: for light tiers, compliance beats generality; `modelComplex` escalations are unaffected since strong models treat skills as hints.
- [`sources` globs too narrow ⇒ silent staleness despite green drift check] → curator instructed to sanity-check glob coverage each pass; lint still catches dead paths regardless.
- [Portfolio gotcha sections start thin without mined failure history] → seed from known review findings (memory, AGENTS.md rules) and grow via the curation loop; thin-but-true beats speculative.

## Migration Plan

1. Land format doc + lint + pre-commit wiring first (floor).
2. Land removals and portfolio skills (each passing lint).
3. Update agent `skills:` lists last, in one commit, so routing flips atomically.
4. Rollback: revert the agent-list commit to restore old routing; skills themselves are additive and inert if unreferenced.

## Open Questions

- Should `/curate-skills` become a scheduled swarm task once trusted, and at what cadence (weekly vs. on-merge-to-main)?
- Is there accessible swarm run/review history to mine for seeding easy-to-miss sections now, or does that wait for the loop to accumulate them?
