# skill-curation

## ADDED Requirements

### Requirement: Curation skill exists and targets curated skills only
A `curate-skills` skill SHALL exist in `.pi/skills/curate-skills/` instructing a frontier-tier model to refresh every skill whose frontmatter has `curated: true`, and SHALL leave non-curated (generic tool) skills untouched.

#### Scenario: Generic skills are skipped
- **WHEN** the curation pass runs over `.pi/skills/`
- **THEN** skills without `curated: true` (e.g. `tdd`, `github-cli`) are not modified

### Requirement: Mechanical drift detection with cheap no-op
For each curated skill, the curator SHALL run `git log <verified-commit>..HEAD -- <sources globs>`; if empty, it SHALL only bump `verified-commit` and `last-verified` without rewriting the body.

#### Scenario: Clean skill is a no-op
- **WHEN** no commits touched a skill's `sources` since its `verified-commit`
- **THEN** the curation pass changes only the `verified-commit` and `last-verified` fields of that skill

#### Scenario: Dirty skill triggers re-verification
- **WHEN** commits touched a skill's `sources` since its `verified-commit`
- **THEN** the curator re-reads the sources and verifies every path, command, and claim in the skill body before bumping the markers

### Requirement: Rewrites are reviewed diffs, append-biased on gotchas
Curation output SHALL be an ordinary working-tree diff submitted for review (never self-merged), and the curator SHALL be instructed to update rather than prune Easy-to-miss entries unless the code the entry refers to no longer exists.

#### Scenario: Curator preserves hard-won gotchas
- **WHEN** the curator rewrites a drifted skill whose Easy-to-miss entries still refer to existing code
- **THEN** those entries survive the rewrite (possibly updated, not deleted)

#### Scenario: Curation goes through review
- **WHEN** a curation pass completes
- **THEN** its changes exist as an uncommitted diff or branch for review, not a direct commit to main

### Requirement: Glob coverage sanity check
On each dirty-skill rewrite, the curator SHALL check whether the skill body references paths outside its `sources` globs and widen the globs (or flag the mismatch) so drift detection stays sound.

#### Scenario: Undersized globs are widened
- **WHEN** a skill's Steps reference `crates/scryrs-types/**` but its `sources` only lists `crates/scryrs-graph/**`
- **THEN** the curation pass adds the missing glob or reports the mismatch in its output
