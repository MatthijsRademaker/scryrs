---
name: skill-convention
description: "Internal convention for authoring curated repo-specific skills in .pi/skills/. Hidden reference doc; read before editing curated skills."
disable-model-invocation: true
---

# Curated Skill Convention

Repo-specific skills in `.pi/skills/` are **curated artifacts**: authored and refreshed by a frontier-tier model, consumed as ground truth by lighter-tier swarm agents. A light model cannot detect a stale skill — it will confidently follow it into files that no longer exist. This convention keeps curated skills true and routable.

Generic tool skills (`tdd`, `github-cli`, `mcp-builder`, …) are exempt from the curation contract but still pass `scripts/skill-lint`.

## Frontmatter contract

```yaml
---
name: add-cli-command
description: <see "Description format" below>
metadata:
  curated: true
  sources:
    - crates/scryrs-cli/**
  last-verified: 2026-07-04
  verified-commit: 8f8ca6c
---
```

- `curated: true` — marks the skill as subject to `/curate-skills` refresh passes.
- `sources` — repo-relative globs covering every file the skill makes claims about. This is the drift-detection contract: `git log <verified-commit>..HEAD -- <sources>` non-empty means the skill needs re-verification. If the skill body references paths outside its `sources`, widen the globs.
- `verified-commit` — the commit SHA whose tree the skill's claims were last verified against. Authoritative drift marker.
- `last-verified` — human-readable companion date. Never trust it over the SHA.

## Description format

The description is the router. Light models select skills by matching task text against descriptions, and they both under- and over-trigger — so descriptions are mechanical:

```
<one sentence: what this skill makes you able to do>.
Use when: <literal repo paths, symbols, and task phrasings that should trigger it>.
Do not use for: <the adjacent skill's territory, named>.
```

Rules:

- `Use when:` MUST contain at least one literal repo path (task prompts usually contain paths; paths are the strongest trigger signal).
- `Do not use for:` MUST name the neighboring skill(s) that own the adjacent territory, so borders are explicit.
- Keep agent `skills:` lists in `.pi/agents/*.md` short. Selection among ~6–8 well-bordered skills is reliable; among 18 it is not.

## Recipe body structure

Curated recipe skills counter a specific failure mode of light models: skipped verification (laziness, assumptions, glossed-over details). Every section exists to make the lazy path and the correct path the same path. Required sections, in order:

### 1. Read first

1–3 files maximum, each with one line on what it tells you. Small on purpose: a bounded reading list gets done; "explore the codebase" gets skipped. If you need a fourth file, the skill's scope is too big — split it.

### 2. Steps

Exact paths, exact registration points. Every step that changes state ends with a verify line:

```
Verify: `grep -n 'pub mod' crates/scryrs-cli/src/commands.rs` — your module must appear in the output.
```

Verification written as prose is optional to a lazy model; written as a runnable command with an expected observation, it is a step. Verify commands must run in the agent container without language SDKs (grep/ls/git, or `scripts/*` Docker-backed entries).

### 3. Easy-to-miss details

3–5 concrete gotchas for this task shape. This section is harvested from real review findings: when a reviewer gate catches a light model missing a detail, that detail gets promoted here. Entries are append-biased — the curator updates them but does not prune them unless the code they refer to is gone.

### 4. Self-check

Yes/no items the agent must pass before calling `report_work_outcome`. Each item should be checkable with a command or a direct file observation, not a feeling.

Map-style skills (orientation documents like `crate-map`) carry the frontmatter contract and description format but are exempt from the recipe body structure.

## Staleness defenses

Two layers, both required:

1. **Floor — `scripts/skill-lint`** (pre-commit + CI): every repo-relative path referenced in any SKILL.md must exist in the working tree; every `sources` glob must match at least one file. Placeholders containing `<`, `>`, or `*` are skipped; a line containing `skill-lint-ignore` is exempt.
2. **Ceiling — `/curate-skills`** (frontier model, on demand): per curated skill, cheap no-op when `sources` are untouched since `verified-commit`; full claim re-verification and rewrite when dirty. Output is a reviewed diff — the curator never self-merges. See `.pi/skills/curate-skills/SKILL.md`.

## Authoring checklist

- [ ] Frontmatter has `curated: true`, `sources`, `last-verified`, `verified-commit` (current HEAD at authoring time).
- [ ] Description has `Use when:` with ≥1 literal path and `Do not use for:` naming neighbors.
- [ ] Recipe body has Read first (≤3 files), Steps with Verify lines, Easy-to-miss details, Self-check.
- [ ] Every referenced path exists (`scripts/skill-lint` passes).
- [ ] Verify commands run without host SDKs.
