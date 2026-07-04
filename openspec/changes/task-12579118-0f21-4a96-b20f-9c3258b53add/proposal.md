## Why

`openspec/changes/` contains 19 non-archived change directories that no longer form a reliable planning surface. Twelve are fully checked and shipped, three have stale checklists on shipped code, one is a superseded duplicate of an archived predecessor with a canonical spec, and three carry genuinely unchecked operational/manual tasks. Agents scanning active changes risk chasing phantom work, and unchecked gaps lack board-task ownership.

This reconciliation pass classifies every active change against board status and repository truth, archives completed/superseded work, corrects stale checklists, converts remaining gaps into board tasks, and aligns roadmap/production-suite docs with the resulting inventory. No application code changes — this is a manual planning-surface hygiene pass.

## What Changes

### 1. Per-change classification inventory

Every non-archived directory under `openspec/changes/` is classified as archive-ready, superseded, or still-active, with board status as the final arbiter and repository code/docs as corroboration.

- **16 archive-ready changes**: 13 fully-checked-and-shipped (add-init-interactive-wizard, add-scryrs-debug-logging, allow-source-repo-pi-init, default-live-mode, native-harness-hook-command, observer-first-disable-bash-default, rename-pi-extension-to-scryrs, split-init-and-setup-commands, sync-scryrs-swarm-scaffold, task-53fc3cbb, task-5c682a97, task-9b0c8757, task-9b98b3fd) plus 3 stale-checklist-with-shipped-code (task-e5d582d9, task-2abf2484, task-cc52db89).
- **1 superseded change**: task-3797be85 (live-dashboard-mode design), superseded by archived task-206a6986 whose work was promoted to canonical spec at `openspec/specs/live-dashboard-mode/spec.md`.
- **2 still-active changes**: live-signal-feed-motion (tasks 5.1-5.3, manual visual verification) and public-binary-and-image-distribution (tasks 6.1-6.4, operational release steps).

The current task-12579118 change is excluded from classification — it represents the reconciliation work itself.

### 2. Archival of completed and stale-checklist changes

Fully-checked changes are archived directly. Stale-checklist changes (task-e5d582d9, task-2abf2484, task-cc52db89) have their unmarked tasks annotated with a correction note documenting shipped work before archiving. Delta specs from task-5c682a97 and task-53fc3cbb are archived as historical-only — canonical specs already cover the shipped contract surfaces.

### 3. Superseded change closure

task-3797be85 is archived with an explicit `superseded-by: 2026-06-29-task-206a6986` reference in its proposal.md before the archive move. No delta spec sync — the canonical spec already lives at `openspec/specs/live-dashboard-mode/spec.md`.

### 4. Board-task handoff for still-active changes

Remaining unchecked items in live-signal-feed-motion (5.1-5.3) and public-binary-and-image-distribution (6.1-6.4) are linked to explicit board tasks with owner and status. These changes stay active under `openspec/changes/` until board tasks are resolved.

### 5. Documentation reconciliation

`roadmap.mdx` Phase 3 and Phase 4 sections are updated from future-tense to past-tense for shipped sub-items (dashboard delivery, live server/ingest/accumulators/query/SSE), while preserving future-tense framing for genuinely deferred items (dashboard live-mode browser automation, multi-agent E2E workflow). `production-suite.md` already reflects shipped state and requires no changes beyond confirming the live-hotspots row aligns with the final classification.

### 6. Reconciliation note

A standalone reconciliation note under `.devagent/docs/docs/reconciliation-2026-07.md` documents the full inventory, evidence, dispositions, and follow-up board tasks. This note serves as the single-source reference for the hygiene pass.

## Impact

- **Openspec surface**: 19 active changes reduced to 3 (live-signal-feed-motion, public-binary-and-image-distribution, and the current task-12579118). Archive directory grows by 16 entries.
- **Agent clarity**: No agent scanning `openspec/changes/` will find fully-checked, superseded, or stale-checklist changes masquerading as active work.
- **Board alignment**: Every unchecked gap either has a board task or is marked as completed via corrected checklist. No orphaned work items.
- **Doc consistency**: Roadmap verb tenses match delivery reality. Production-suite inventory aligns with the final change classification.
- **Zero code impact**: No application, runtime, or test code is touched.