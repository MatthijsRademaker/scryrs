## 1. Part A — Author the agent context skill

- [ ] 1.1 Create the canonical skill source (working name `scryrs-context`) with curated frontmatter per `.pi/skills/CONVENTION.md`: `curated: true`, `sources` globs covering `crates/scryrs-cli/**` and the route/hint spec surface, `last-verified`, `verified-commit`.
- [ ] 1.2 Write the `description` as a routing trigger — the field a model reads to decide whether to load the skill. State when to use it and when not to.
- [ ] 1.3 Document the read-side CLI surface with verified flags: `route bundle`, `route explain`, `hotspots`, `graph`, `doctor`. Run each command and confirm every flag and output field before writing it down.
- [ ] 1.4 Document the prerequisite chain: `graph` → `.scryrs/graph.json`, `route` → `.scryrs/routes.json`, `explain`/`bundle` read-only over the manifest.
- [ ] 1.5 Document bundle vs. explain: bundle is the bounded planning surface for loading context, explain is the unbounded diagnostic ranking surface.
- [ ] 1.6 Document the `RouteHintItem` / `RouteBundleDocument` fields an agent must act on: `loadTarget` kinds (`file`, `doc_page`, `non_loadable`), `rank` vs. explain `relevance`, evidence citations, `reason`.
- [ ] 1.7 Document `non_loadable` as a correct, expected result for `search` / `symbol` / `domain_term` / `doc_group` routes — not an error and not something to work around.
- [ ] 1.8 Document unmet preconditions as normal states: missing manifest, empty `hints` array, manifest older than the working tree — with what each means and what to do.
- [ ] 1.9 State the search fallback explicitly: when retrieval returns nothing, search normally — and note that the fallback is itself the signal scryrs captures.
- [ ] 1.10 `scripts/skill-lint` passes on the new skill.
- [ ] 1.11 Verify the skill is reachable by `/curate-skills` refresh passes.

## 2. Part A — Install the skill via init

- [ ] 2.1 Decide the install location per harness (shared vs. harness-specific) and record the decision in `design.md` (D5 open question).
- [ ] 2.2 Settle the ownership model following the Pi hook precedent: canonical source in the repository, installed copy explicitly non-canonical and not agent-editable. Document it in `AGENTS.md` if it warrants a rule.
- [ ] 2.3 Install the skill from `scryrs init --agent <NAME>` alongside the trace hook.
- [ ] 2.4 Keep `init` idempotent and config-free: re-running writes no duplicate, and `init` still never writes `scryrs.json` or `.scryrs/`.
- [ ] 2.5 Confirm `setup` remains the only command that writes `scryrs.json` and the `.scryrs/` scaffold.
- [ ] 2.6 Report skill install status in `scryrs doctor`, matching how hook status is reported.
- [ ] 2.7 Update `scryrs init` help text and `--help-json` to state that init installs the hook **and** the context skill.
- [ ] 2.8 Handle a locally-modified installed skill without silently overwriting a human's edits, per the 2.2 decision.

## 3. Part B — Curator generates a valid SKILL.md draft

> Depends on `trace-signal-fidelity` for output quality. Do not judge generated content against the current corpus.

- [ ] 3.1 Replace the metadata-stub content in `skill_proposal` (`crates/scryrs-curator/src/lib.rs:134`) with a valid `SKILL.md` draft.
- [ ] 3.2 Emit YAML frontmatter with a kebab-case `name` matching the eventual directory and a `description` written as a routing trigger.
- [ ] 3.3 Ground the body in evidence: subject, the failure pattern that triggered generation, co-occurring files from the same sessions, and evidence row ids a reviewer can verify.
- [ ] 3.4 Mark the artifact as machine-generated with its source proposal id so it is never mistaken for a hand-curated skill.
- [ ] 3.5 Keep generation fully deterministic — no LLM in this path (`scryrs-curator-llm` stays library-only and off the deterministic path).
- [ ] 3.6 Confirm the generated draft passes `scripts/skill-lint`'s structural floor.
- [ ] 3.7 Record in the proposal that content-addressed `skill` proposal ids change, and update affected tests and fixtures.
- [ ] 3.8 Do not overstate the artifact: it is an evidence-assembled stub for a human to write into, not an authored skill (D6).

## 4. Part B — The publish skill adapter

- [ ] 4.1 Create `crates/scryrs-adapter-skill` following the `add-adapter` recipe and the shape of `scryrs-adapter-markdown`.
- [ ] 4.2 Expose `publish_accepted_skill` reading `.scryrs/accepted/` only; pending and rejected artifacts never publish.
- [ ] 4.3 Write one directory per skill containing `SKILL.md`, with frontmatter `name` matching its directory name.
- [ ] 4.4 Publish only `skill`-target accepted artifacts; ignore other target types without failing.
- [ ] 4.5 Deterministic output, and never delete stale output — matching the markdown adapter's contract.
- [ ] 4.6 Add the Cargo feature and register it in the workspace feature split.
- [ ] 4.7 Add `scryrs publish skill <PATH> --output <DIR>` to `crates/scryrs-cli/src/publish.rs`, dispatched alongside markdown and rspress.
- [ ] 4.8 Emit a `SkillPublishSummary` JSON summary consistent with the other adapters.
- [ ] 4.9 Update `scryrs publish` help text, `--help-json`, and the top-level help COMMANDS block.
- [ ] 4.10 Keep publishing explicit: `scryrs proposals accept` still does not publish.

## 5. The PR review gate

- [ ] 5.1 Document the loop order: `hotspots → graph → route → propose → accept (staged) → publish skill → open PR`.
- [ ] 5.2 Require the staged decision's `reviewer` to be the automating identity (for example `scryrs-autonomous-loop`), never a human name.
- [ ] 5.3 Ensure both the review-decision artifact and the generated `SKILL.md` land in the same PR.
- [ ] 5.4 Confirm the loop never writes into a live skill directory — output is a proposed diff only.
- [ ] 5.5 No auto-merge, and no branch protection bypass.
- [ ] 5.6 Decide who runs the loop (scheduled CI vs. human-triggered) and bound PR volume accordingly (D7 open question).
- [ ] 5.7 Document the dual meaning of `.scryrs/accepted/` (human-accepted vs. automation-staged) and how the `reviewer` field disambiguates it.

## 6. Verification

- [ ] 6.1 `scripts/precommit-run` passes.
- [ ] 6.2 `scripts/skill-lint` passes on both the authored skill and a generated skill.
- [ ] 6.3 Adapter tests: accepted-only reading, deterministic output, no stale deletion, non-skill target types ignored, invalid accepted artifact rejected.
- [ ] 6.4 `init` tests: skill installed, idempotent re-run, no `scryrs.json` or `.scryrs/` written, locally-modified copy handled per 2.8.
- [ ] 6.5 Verify a published skill actually loads in a real harness — frontmatter parses, the skill is routable by name.
- [ ] 6.6 Verify the Part A skill end-to-end in a real session: an agent loads it, runs `route bundle`, and loads the returned `loadTarget` files.
- [ ] 6.7 Confirm scryrs's own retrieval invocations do not pollute the hotspot corpus they read from (D7 risk).
- [ ] 6.8 Dogfood the full loop on this repository and read the resulting PR as a reviewer. If the PR is not worth merging, 3.x is not done.
- [ ] 6.9 Update `.devagent/docs/docs/route-manifests.md` and `proposals.md` for the new consumer and publishing surfaces.
- [ ] 6.10 Update `roadmap.mdx` Phase 8 to name the shipped consumer surface, and Phase 7 to name the third publish adapter. Keep `roadmap-truth` spec requirements satisfied.
