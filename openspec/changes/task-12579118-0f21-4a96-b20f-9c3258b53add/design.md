## Context

The repository's active OpenSpec change set under `openspec/changes/` holds 19 non-archived directories (excluding the archive/ subdirectory). These break into four evidence categories based on board status, task-checklist completion, and shipped-code verification:

| Category | Count | Examples |
|---|---|---|
| Fully-checked, shipped | 13 | add-init-interactive-wizard, task-53fc3cbb, task-5c682a97 |
| Stale-checklist, shipped | 3 | task-e5d582d9, task-2abf2484, task-cc52db89 |
| Superseded duplicate | 1 | task-3797be85 (superseded by archived task-206a6986) |
| Genuinely active, unchecked manual/operational gaps | 2 | live-signal-feed-motion, public-binary-and-image-distribution |

The current task-12579118 directory is the reconciliation task itself and is excluded from classification.

Fully-checked changes have 100% `[x]` task completion and corresponding shipped artifacts confirmed in executable code. Stale-checklist changes have all-unchecked task lists despite the described features being fully implemented (verified via direct code inspection of dispatch.rs, help_json.rs, server.rs, scripts/verify-live-hotspots, and test files). The superseded change is a design proposal whose scope was later implemented under an archived predecessor and promoted to a canonical spec. The two still-active changes carry unchecked tasks that represent real manual/operational work not yet reflected in shipped code (visual verification flows, release tagging, package visibility).

## Goals / Non-Goals

### Goals

- Build an evidence-backed inventory of every non-archived change under `openspec/changes/`, using board status as the final arbiter and repository truth as corroboration.
- Classify each change as archive-ready, superseded, or still-active with board-status and code/doc evidence citations.
- Archive completed/superseded changes after correcting stale checklists and recording supersession chains.
- Ensure real remaining gaps are linked to explicit board tasks with owner/status.
- Reconcile `roadmap.mdx` and `production-suite.md` so planning docs no longer contradict the final classification.
- Publish a short reconciliation note documenting all 19 classified changes and their dispositions.

### Non-Goals

- Do not change application/runtime behavior or ship new product features.
- Do not build automated reconciliation tooling; this is a manual hygiene pass.
- Do not rewrite canonical OpenSpec specs speculatively beyond what archiving/checklist correction requires.
- Do not reopen already shipped work just because older proposals describe an earlier product shape.
- Do not sync delta specs from completed changes into `openspec/specs/` — archiving as historical-only is sufficient when canonical specs already exist.

## Decisions

### Decision 1: Board status is the authoritative classification arbiter

When board status and repository truth disagree, board status wins. If a change appears archive-ready based on checked checklists and shipped code but the board still shows it as In Progress, the board status is preserved and the change stays active with a note to resolve the discrepancy. The executor must query board task history for every non-archived change before finalizing classification.

**Rationale**: The task acceptance criteria explicitly require completed work to be "board-Done" before archiving. Repository code truth alone is corroborating evidence, not the final arbiter.

### Decision 2: Stale-checklist changes get corrected before archival

Changes whose tasks.md is all-unchecked but whose described work is shipped in executable code (task-cc52db89, task-2abf2484, task-e5d582d9) must have each unchecked item marked with a correction annotation before the change is archived. The annotation records that the work was completed outside the OpenSpec change and cites the shipped artifact.

**Rationale**: The reviewer specifically flagged that archiving these changes without correcting checklists would "wrongly infer incomplete work" for future agents scanning archive history. This is documentation work, not application-code change, and is permitted under the task scope.

### Decision 3: Delta specs archive as historical-only when canonical specs exist

Completed changes with delta specs (task-5c682a97's `specs/proposal-review-contract/spec.md`, task-53fc3cbb's `specs/dashboard-phase-goal/spec.md`) are archived without syncing their deltas into `openspec/specs/`. Canonical specs already exist for both surfaces (`openspec/specs/proposal-contract/`, `openspec/specs/dashboard-frontend-stack/`) and the deltas were design-phase specifications that the canonical specs supersede.

**Rationale**: The architect assessed the sync risk as low because "canonical specs already supersede these design-phase deltas." The non-goal list explicitly prohibits "rewriting canonical OpenSpec specs speculatively." Archiving as historical-only preserves the design rationale without polluting canonical specs with stale design material.

### Decision 4: Superseded changes reference the successor explicitly

task-3797be85 is archived with an explicit `superseded-by: 2026-06-29-task-206a6986-5940-4824-a202-c7c759da4548` note in its proposal.md before the archive move. The archived predecessor is fully checked and its work was promoted to canonical spec at `openspec/specs/live-dashboard-mode/spec.md`.

**Rationale**: The reviewer required an explicit supersession chain to prevent re-blocking on the same ambiguity. Both the architect and lead dev confirmed task-206a6986 as the authoritative implementation of live dashboard mode.

### Decision 5: task-9b98b3fd (CLI v0 contract) is archive-ready, not superseded

The architect classified task-9b98b3fd as superseded because the product went multi-command (opposite direction), but all its tasks are checked and the CLI v0 contract was shipped — the fact that the product evolved beyond the contract does not make the change obsolete; it makes it completed. The lead dev's fully-checked classification is adopted.

**Rationale**: A change with 100% task completion whose deliverables shipped is archive-ready regardless of later product evolution. The architect's superseded classification conflates "product evolved differently" with "change was never completed."

### Decision 6: Remaining gaps get board tasks with explicit owner/status

For the two still-active changes (live-signal-feed-motion tasks 5.1-5.3, public-binary-and-image-distribution tasks 6.1-6.4), the reconciliation creates or verifies board tasks for each unchecked item. Each board task must carry an owner assignment and current status. The changes remain under `openspec/changes/` while the board tasks are unresolved.

**Rationale**: The acceptance criteria require "Real remaining gaps exist as board tasks" and "No active change remains without clear owner/status." These unchecked tasks are manual/operational work (visual verification, release tagging, package visibility) that cannot be confirmed as shipped via repository evidence alone.

### Decision 7: Roadmap verb-tense updates are documentation hygiene, not scope expansion

`roadmap.mdx` Phase 3 (Dashboard) and Phase 4 (Live Hotspot Server) required-deliverables lists use future tense for items that are demonstrably shipped. Updating these to past-tense is documentation alignment with repository truth, not adding new roadmap claims.

**Rationale**: The lead dev confirmed roadmap and production-suite docs are "already consistent with this classification." The architect identified "minor verb-tense corrections" as the only needed change. This is within scope as documentation reconciliation.

## Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Board status unavailable during execution | Medium | High — classifications cannot be finalized | Query board before every archive/supersede action. Document per-change board status in the reconciliation note. Flag any changes whose board status disagrees with repo evidence. |
| Stale-checklist correction is incomplete (not all shipped code items correctly mapped to unchecked tasks) | Low | Medium — future agents may still see some ambiguous checkboxes | Cross-reference each task section against the specific shipped artifact (crate, script, test file). Document the mapping in the reconciliation note. |
| Archiving without delta-spec sync loses design intent for future agents | Low | Low | Both flagged changes (task-5c682a97, task-53fc3cbb) have canonical specs covering the same contract surfaces. The archive preserves the deltas as historical artifacts accessible in the archive directory. |
| Board tasks for remaining gaps are created but never assigned | Medium | Medium — gaps remain orphaned | The reconciliation note must list each board task ID, owner, and status. If no owner is available, assign to the project maintainer and flag for triage. |
| Roadmap changes introduce ambiguity between shipped and deferred sub-items | Low | Medium | Verb-tense updates are scoped to items confirmed shipped by both board status and repository truth. Deferred sub-items (dashboard live-mode browser automation, multi-agent E2E) remain future-tense. |

## Traceability

- **Dossier**: `"createdAt":"2026-07-01T22:00:36.332Z"` — problem framing, goals, non-goals, open questions, affected areas
- **Decision 1-swarm-architect-recommendation**: Classification buckets (12 archive-ready, 4 superseded, 2 still-active, 1 stale-checklist), delta-spec sync recommendation, roadmap verb-tense guidance
- **Decision 1-swarm-lead-dev-recommendation**: Per-change inventory and evidence citations, board-status dependency callout, 14 archive-ready classification with code evidence
- **Decision 1-swarm-reviewer-recommendation**: Stale-checklist-on-shipped-work pattern, supersession-chain requirement, delta-spec sync question, board-task handoff protocol
- **Artifact snapshot**: `proposal-synthesis-input` — current stub proposal.md and tasks.md
- **Repository evidence**: `crates/scryrs-cli/src/help_text.rs`, `crates/scryrs-cli/src/help_json.rs`, `crates/scryrs-cli/src/dispatch.rs`, `crates/scryrs-server/`, `crates/scryrs-types/src/lib.rs`, `scripts/verify-live-hotspots`, `scripts/verification/README.md`, `.devagent/docs/docs/roadmap.mdx`, `.devagent/docs/docs/production-suite.md`