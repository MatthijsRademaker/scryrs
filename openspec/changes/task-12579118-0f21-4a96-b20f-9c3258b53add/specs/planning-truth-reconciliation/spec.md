# planning-truth-reconciliation Specification

## Purpose

Define the manual reconciliation protocol that brings `openspec/changes/` into agreement with board task truth. This spec describes the classification rules, archival protocol, stale-checklist correction procedure, supersession handling, board-task handoff, and documentation reconciliation steps required for a planning-surface hygiene pass. It does not define automated tooling — reconciliation is a manual process.

## ADDED Requirements

### Requirement: Active Change Inventory

Every non-archived directory under `openspec/changes/` SHALL be inventoried and classified before any archival action proceeds. The current reconciliation task directory (task-12579118) SHALL be excluded from classification.

Each classification SHALL cite board task status (Done, In Progress, Blocked) from the authoritative task board and repository evidence (shipped code paths, scripts, docs, test files) that corroborates or contradicts the board status.

Three classification buckets SHALL be used:
- **archive-ready**: Board status is Done AND repository evidence confirms the described work is complete.
- **superseded**: Another change (archived or active) already implements the same scope, AND the board confirms the original change is no longer the active implementation target.
- **still-active**: Board status is In Progress or Blocked, OR board status is Done but the task checklist has unchecked items representing real remaining work not yet reflected in shipped code.

#### Scenario: Fully-checked change is archive-ready

- **GIVEN** `add-init-interactive-wizard/tasks.md` has every task line marked `[x]`
- **AND** board status for the corresponding task is Done
- **AND** `crates/scryrs-cli/src/` contains the shipped wizard implementation
- **WHEN** the classification inventory is built
- **THEN** `add-init-interactive-wizard` is classified as archive-ready
- **AND** the inventory cites board status Done plus the shipped code path as evidence

#### Scenario: Stale-checklist change is archive-ready after correction

- **GIVEN** `task-cc52db89/tasks.md` has all unchecked `[ ]` items
- **AND** board status for the task is Done
- **AND** `crates/scryrs-cli/src/help_json.rs` contains the shipped `--help-json` implementation
- **AND** `crates/scryrs-cli/tests/dispatch_tests.rs` has 15+ tests covering `--help-json`
- **WHEN** the classification inventory is built
- **THEN** `task-cc52db89` is classified as archive-ready with a stale-checklist annotation
- **AND** the inventory cites the shipped code and test paths as evidence

#### Scenario: Superseded duplicate is identified by archived predecessor

- **GIVEN** `task-3797be85/tasks.md` describes live-dashboard-mode work with all unchecked items
- **AND** `openspec/changes/archive/2026-06-29-task-206a6986/tasks.md` has the same scope fully checked
- **AND** `openspec/specs/live-dashboard-mode/spec.md` was created by archiving task-206a6986
- **AND** board status for task-3797be85 is Done or Blocked with a note referencing the archived successor
- **WHEN** the classification inventory is built
- **THEN** `task-3797be85` is classified as superseded
- **AND** the inventory cites the archived predecessor and canonical spec as evidence

#### Scenario: Genuinely active change with unchecked operational tasks

- **GIVEN** `public-binary-and-image-distribution/tasks.md` has unchecked items 6.1-6.4 (tag push, visibility flip, public verification)
- **AND** board status for the task is In Progress
- **AND** no shipped code or script confirms completion of the release operations
- **WHEN** the classification inventory is built
- **THEN** `public-binary-and-image-distribution` is classified as still-active
- **AND** the unchecked items 6.1-6.4 are flagged for board-task handoff

### Requirement: Stale Checklist Correction

When a change is classified as archive-ready but its `tasks.md` has all-unchecked items because the described work was completed outside the OpenSpec change, each unchecked task line SHALL be corrected before archival.
The correction SHALL replace `- [ ]` with `- [x]` and append an HTML comment annotating the shipped artifact. The annotation SHALL cite a specific file path and, where practical, relevant line numbers.

#### Scenario: Stale-checklist task is corrected with artifact citation

- **GIVEN** `task-e5d582d9/tasks.md` line 1.1 reads `- [ ] 1.1 Create scripts/verify-live-hotspots`
- **AND** `scripts/verify-live-hotspots` exists as an executable shipped artifact
- **WHEN** stale-checklist correction is applied
- **THEN** the line becomes `- [x] 1.1 Create scripts/verify-live-hotspots <!-- completed outside OpenSpec: scripts/verify-live-hotspots -->`

### Requirement: Archiving Protocol

Changes classified as archive-ready SHALL be moved to `openspec/changes/archive/<YYYY-MM-DD>-<change-name>/`. The date prefix SHALL be the date of archival.

Before the archive move, all stale checklists SHALL be corrected per the stale-checklist correction requirement. If the change has a delta spec directory and a canonical spec already exists under `openspec/specs/` for the same capability, the delta SHALL be archived as historical-only without syncing to the canonical location. The change's `.openspec.yaml` SHALL NOT be modified.

After the archive move, the change directory SHALL no longer exist under `openspec/changes/` (only in `openspec/changes/archive/`). Existing canonical specs under `openspec/specs/` SHALL remain unmodified.

#### Scenario: Fully-checked change is archived with date prefix

- **GIVEN** `add-init-interactive-wizard/` is classified as archive-ready
- **AND** the archival date is 2026-07-01
- **WHEN** the archive action runs
- **THEN** the directory is moved to `openspec/changes/archive/2026-07-01-add-init-interactive-wizard/`
- **AND** `openspec/changes/add-init-interactive-wizard/` no longer exists

#### Scenario: Completed change with delta spec is archived as historical-only

- **GIVEN** `task-5c682a97/` is classified as archive-ready
- **AND** `task-5c682a97/specs/proposal-review-contract/spec.md` exists as a delta spec
- **AND** `openspec/specs/proposal-contract/spec.md` exists as the canonical spec covering the same contract surface
- **WHEN** the archive action runs
- **THEN** the delta spec is preserved in the archive directory at `openspec/changes/archive/<date>-task-5c682a97/specs/proposal-review-contract/spec.md`
- **AND** no content is synced to `openspec/specs/proposal-contract/spec.md`
- **AND** the canonical spec remains unmodified

### Requirement: Supersession Chain

When a change is classified as superseded by an archived predecessor, its `proposal.md` SHALL be updated with an explicit `## Superseded By` section before archival. The section SHALL reference the full archived change directory name (including date prefix), link to the canonical spec that now covers the capability if one exists, and state that no delta spec sync is required.

#### Scenario: Superseded change references archived predecessor

- **GIVEN** `task-3797be85/` is classified as superseded by archived `2026-06-29-task-206a6986-5940-4824-a202-c7c759da4548`
- **AND** `openspec/specs/live-dashboard-mode/spec.md` exists as the canonical spec
- **WHEN** the supersession annotation is applied
- **THEN** `task-3797be85/proposal.md` gains a `## Superseded By` section referencing the archived change and canonical spec
- **THEN** `task-3797be85/` is archived to `openspec/changes/archive/`

### Requirement: Board-Task Handoff

Unchecked items in still-active changes that represent real remaining work SHALL be linked to explicit board tasks. Each board task SHALL have an assigned owner and current status. The still-active change's `proposal.md` SHALL reference the board task IDs. The reconciliation note SHALL list every created or verified board task with its ID, owner, and status.

#### Scenario: Manual verification tasks get board-task linkage

- **GIVEN** `live-signal-feed-motion/tasks.md` has unchecked items 5.1 (visual pass on static hotspot card state), 5.2 (visual pass on motion transitions), 5.3 (Live mode smoke in supported browsers)
- **AND** no shipped code confirms completion of these manual tasks
- **WHEN** board-task handoff runs
- **THEN** a board task is created or verified for each of 5.1, 5.2, and 5.3
- **AND** each board task has an assigned owner and current status
- **AND** `live-signal-feed-motion/proposal.md` is updated to reference the board task IDs
- **AND** the reconciliation note records the board-task IDs, owners, and statuses

#### Scenario: Operational release tasks get board-task linkage

- **GIVEN** `public-binary-and-image-distribution/tasks.md` has unchecked items 6.1 (tag and push release), 6.2 (set Docker Hub repo to public), 6.3 (verify public pull works), 6.4 (publish release notes)
- **AND** no shipped artifact confirms completion of these operational steps
- **WHEN** board-task handoff runs
- **THEN** a board task is created or verified for each of 6.1, 6.2, 6.3, and 6.4
- **AND** each board task has an assigned owner and current status
- **AND** the reconciliation note records the board-task IDs, owners, and statuses

### Requirement: Documentation Consistency

After all change classifications are finalized, `roadmap.mdx` and `production-suite.md` SHALL be verified for consistency with the final inventory.

For `roadmap.mdx`, phase delivery status descriptions SHALL use past tense for items confirmed as shipped by both board status and repository truth. Items that remain genuinely deferred SHALL keep future tense. The verb-tense update SHALL NOT add new roadmap claims or change delivery scope.

For `production-suite.md`, the Current State table SHALL be verified against the final classification. If the table already aligns, no changes are required.

#### Scenario: Roadmap verbs updated for shipped Phase 3 dashboard

- **GIVEN** `roadmap.mdx` Phase 3 section lists "Required deliverables" with future-tense descriptions
- **AND** the dashboard is confirmed shipped (board Done, `crates/scryrs-dashboard/` exists, `scryrs dashboard` CLI works)
- **WHEN** documentation reconciliation runs
- **THEN** shipped sub-items (dashboard CLI, local HTTP server, Vue.js SPA, hotspot table, session/event views, per-subject drill-down, extensible component architecture) change to past tense
- **AND** genuinely deferred sub-items (graph visualization, route exploration, hosted dashboard, real-time updates, data mutation) remain future tense

#### Scenario: Roadmap verbs updated for shipped Phase 4 live server

- **GIVEN** `roadmap.mdx` Phase 4 section lists "Required deliverables" with future-tense descriptions
- **AND** the live hotspot server is confirmed shipped (board Done, `crates/scryrs-server/` exists, batch ingest shipped, accumulators shipped, query/SSE APIs shipped, dashboard live mode shipped)
- **WHEN** documentation reconciliation runs
- **THEN** shipped sub-items (server ingest contract, server-owned SQLite, remote record transport, incremental accumulators, HotspotSignal query/stream APIs, dashboard live mode) change to past tense
- **AND** deferred sub-items (dashboard live-mode browser automation, multi-agent E2E workflow) remain future tense

### Requirement: Reconciliation Note

A standalone reconciliation note SHALL be published at `.devagent/docs/docs/reconciliation-2026-07.md`. The note SHALL contain the reconciliation date and executor identification, the board snapshot timestamp, the full change inventory with per-change classification and evidence, the stale-checklist correction mappings, the supersession chain for task-3797be85, the follow-up board tasks with owner and status, and a summary of documentation changes made.

#### Scenario: Reconciliation note captures full inventory

- **GIVEN** all 19 changes are classified and all archive/handoff/reconciliation steps are complete
- **WHEN** the reconciliation note is written
- **THEN** the note documents all 19 changes with their classifications, evidence, and dispositions
- **AND** the note serves as the single-source reference for the hygiene pass without requiring the reader to re-inspect individual change directories
