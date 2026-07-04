---
name: curate-skills
description: "Refresh loop for curated repo-specific skills — verify their claims against current tree and rewrite what drifted. Use when: asked to curate `.pi/skills/`, after large refactors that moved files a curated skill references, or when `scripts/skill-lint` keeps failing on a curated skill. Do not use for: authoring a brand-new skill (follow `.pi/skills/CONVENTION.md` directly) or fixing generic tool skills (out of curation scope)."
metadata:
  curated: true
  sources:
    - .pi/skills/**
    - scripts/skill-lint
    - AGENTS.md
  last-verified: 2026-07-04
  verified-commit: f7d769b
---

# Curate Skills

You are ceiling of two-layer staleness defense (`scripts/skill-lint` is always-on floor; see `.pi/skills/CONVENTION.md`). This pass is for frontier-tier model: re-verify claims that lighter models consume as ground truth. Precision beats speed.

**Scope:** only skills whose frontmatter has `metadata.curated: true`. Generic tool skills (`tdd`, `github-cli`, `mcp-builder`, …) are untouched.

**Output contract:** ordinary working-tree diff for review. Never commit to main, never self-merge. Summarize per skill: clean no-op / drifted (what changed) / flagged (needs human decision).

## Read first

1. `.pi/skills/CONVENTION.md` — frontmatter contract, routing-description format, recipe-body section order.
2. `scripts/skill-lint` — floor checks; tells you what path/glob failures are already enforced mechanically.
3. `AGENTS.md` — repo guardrails that easy-to-miss sections should encode and preserve.

## Steps

1. Enumerate curated skills only: inspect `.pi/skills/*/SKILL.md` and skip anything without `metadata.curated: true`.
   Verify: `grep -n "curated: true" .pi/skills/*/SKILL.md` lists only repo-specific curated skills; generic tool skills stay absent.
2. For each curated skill, run dirty check from its frontmatter markers: `git log --oneline <verified-commit>..HEAD -- <source globs>`.
   Verify: empty output means no-op path; non-empty output names commits that touched the skill's claimed source area.
3. Empty dirty check: bump only `verified-commit` to current `HEAD` and `last-verified` to today. Non-empty dirty check: read touched diffs plus current source files, then re-verify every path, symbol, command, count, and Verify line in the skill body before rewriting drifted sections.
   Verify: `git diff -- .pi/skills/<skill>/SKILL.md` shows marker-only changes for clean skills, and body changes only where drift required them for dirty skills.
4. On every dirty rewrite, run glob-coverage sanity check: compare repo paths mentioned in body against `metadata.sources`; widen globs or flag mismatch if body references paths outside coverage. Keep Easy-to-miss details append-biased: update moved references, delete only if code is gone.
   Verify: `scripts/skill-lint` stays green after your rewrite, and `git log <new-verified-commit>..HEAD -- <sources>` would be sound for next pass.
5. After loop, run `scripts/skill-lint`, then review routing borders so `Use when:` / `Do not use for:` clauses still partition neighboring skills cleanly. Report result as reviewed diff, never self-merged.
   Verify: `scripts/skill-lint` exits 0, and your outcome report has one line per curated skill plus diff stat.

## Easy-to-miss details

- **Clean means marker-only.** If `git log <verified-commit>..HEAD -- <sources>` is empty, changing body text anyway violates no-op path and makes future drift review noisy.
- **Body references outside `sources` make clean checks fiction.** If skill mentions `crates/scryrs-types/**` but sources omit it, widen globs now or future curation will miss real drift.
- **Do not prune gotchas casually.** Easy-to-miss entries are harvested from failures and review findings; keep them unless referenced code is genuinely gone.
- **Never "fix" failing Verify lines by weakening them.** If command no longer proves claim, skill is stale; repair claim or registration point instead.
- **Installed hook copies stay untouchable.** Even during curation, `.pi/extensions/scryrs/index.ts` and `.pi/extensions/pi-trace/index.ts` are not source of truth. <!-- skill-lint-ignore: gitignored install target, absent until scryrs init runs -->

## Self-check before reporting done

- [ ] Did you skip every non-curated skill?
- [ ] Did every clean skill get marker-only changes, not opportunistic rewrites?
- [ ] Did every dirty skill get path/command/claim re-verification plus glob-coverage review?
- [ ] Is `scripts/skill-lint` green after edits?
- [ ] Are results left as reviewable diff, not self-merged?
