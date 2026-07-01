## 1. Pre-Reconciliation: Board Status Verification

- [ ] 1.1 Query board task history for all 19 non-archived change directories under `openspec/changes/` (excluding task-12579118).
- [ ] 1.2 For each change, record whether the board status is Done, In Progress, or Blocked.
- [ ] 1.3 Flag any change where board status contradicts the evidence-based classification (e.g., board says In Progress but repo evidence shows fully shipped).
- [ ] 1.4 Document board-status findings in the reconciliation note, citing the board snapshot timestamp.

## 2. Classification Inventory

- [ ] 2.1 Classify all 19 non-archived changes as archive-ready, superseded, or still-active using the per-change evidence table below.
- [ ] 2.2 For each classification, cite board status (from §1) and repository evidence (shipped code, docs, scripts).
- [ ] 2.3 Publish the complete classification inventory in the reconciliation note.

### Archive-Ready (16 changes)

- [ ] 2.4 `add-init-interactive-wizard` — fully checked, `scryrs init` wizard shipped.
- [ ] 2.5 `add-scryrs-debug-logging` — fully checked, `SCRYRS_DEBUG` env var shipped.
- [ ] 2.6 `allow-source-repo-pi-init` — fully checked, Pi source-repo init shipped.
- [ ] 2.7 `default-live-mode` — fully checked, live default verified by `scripts/verify-live-hotspots`.
- [ ] 2.8 `native-harness-hook-command` — fully checked, `scryrs hook` shipped.
- [ ] 2.9 `observer-first-disable-bash-default` — fully checked, bash gating behind `SCRYRS_DEBUG` shipped.
- [ ] 2.10 `rename-pi-extension-to-scryrs` — fully checked, extension renamed.
- [ ] 2.11 `split-init-and-setup-commands` — fully checked, init/setup split shipped.
- [ ] 2.12 `sync-scryrs-swarm-scaffold` — fully checked, scaffold sync done.
- [ ] 2.13 `task-53fc3cbb` (dashboard phase goal) — 36/36 tasks checked, dashboard shipped.
- [ ] 2.14 `task-5c682a97` (proposal review contract) — fully checked, `ProposalReviewDecision` and `REVIEW_DECISION_SCHEMA_VERSION` in scryrs-types.
- [ ] 2.15 `task-9b0c8757` (test infrastructure) — fully checked, test infrastructure shipped.
- [ ] 2.16 `task-9b98b3fd` (CLI v0 contract) — fully checked, CLI contract shipped (product evolved multi-command but change tasks were completed).
- [ ] 2.17 `task-e5d582d9` (live verification) — stale checklist; `scripts/verify-live-hotspots` and `scripts/verification/README.md` exist as shipped artifacts.
- [ ] 2.18 `task-2abf2484` (central server/ingest) — stale checklist; `crates/scryrs-server/` with batch ingest, `scryrs server` CLI, `BatchIngestResponse` in types all shipped.
- [ ] 2.19 `task-cc52db89` (--help-json) — stale checklist; `--help-json`/`-hj` implemented in dispatch.rs, serialized in help_json.rs, documented in help_text.rs, with 15+ tests.

### Superseded (1 change)

- [ ] 2.20 `task-3797be85` (live-dashboard-mode design) — superseded by archived `2026-06-29-task-206a6986` whose work was promoted to canonical spec at `openspec/specs/live-dashboard-mode/spec.md`.

### Still-Active (2 changes)

- [ ] 2.21 `live-signal-feed-motion` — tasks 5.1-5.3 (manual visual verification) unchecked; no shipped evidence of completion.
- [ ] 2.22 `public-binary-and-image-distribution` — tasks 6.1-6.4 (operational release steps: tag push, visibility flip, public verification) unchecked; no shipped evidence of completion.

## 3. Archive Completed and Stale-Checklist Changes

- [ ] 3.1 For the 13 fully-checked archive-ready changes (2.4-2.16): verify board status is Done, then move each directory to `openspec/changes/archive/<YYYY-MM-DD>-<change-name>/`.
- [ ] 3.2 For the 3 stale-checklist changes (2.17-2.19): correct each tasks.md by adding a `[x]` prefix and a `<!-- completed outside OpenSpec: <shipped artifact> -->` annotation to every task line.
- [ ] 3.3 After checklist correction, verify board status for the 3 stale-checklist changes is Done, then archive each to `openspec/changes/archive/`.
- [ ] 3.4 For task-5c682a97 and task-53fc3cbb: archive delta specs (`specs/proposal-review-contract/spec.md`, `specs/dashboard-phase-goal/spec.md`) as historical-only without syncing to `openspec/specs/`.

## 4. Close Superseded Change

- [ ] 4.1 Update `task-3797be85/proposal.md` with an explicit `## Superseded By` section referencing `2026-06-29-task-206a6986-5940-4824-a202-c7c759da4548` and linking to `openspec/specs/live-dashboard-mode/spec.md`.
- [ ] 4.2 Archive `task-3797be85` to `openspec/changes/archive/`.

## 5. Board-Task Handoff for Still-Active Changes

- [ ] 5.1 For `live-signal-feed-motion` tasks 5.1-5.3: create or verify a board task for each unchecked item with an assigned owner and current status.
- [ ] 5.2 For `public-binary-and-image-distribution` tasks 6.1-6.4: create or verify a board task for each unchecked item with an assigned owner and current status.
- [ ] 5.3 Document the board-task IDs, owners, and statuses in the reconciliation note.
- [ ] 5.4 Update each still-active change's `proposal.md` to reference the corresponding board tasks.

## 6. Documentation Reconciliation

- [ ] 6.1 Update `roadmap.mdx` Phase 3 section: change "Required deliverables" verb tenses from future to past for shipped items (dashboard CLI, local HTTP server, Vue.js SPA, hotspot table, session/event views, per-subject drill-down, extensible component architecture). Keep future tense for deferred items (graph visualization, route exploration, hosted dashboard, real-time updates, data mutation).
- [ ] 6.2 Update `roadmap.mdx` Phase 4 section: change verb tenses from future to past for shipped sub-items (server ingest contract, server-owned SQLite, remote record transport, incremental accumulators, HotspotSignal query/stream APIs, dashboard live mode). Keep future tense for deferred items (dashboard live-mode browser automation, multi-agent E2E workflow).
- [ ] 6.3 Verify `production-suite.md` Live hotspots row aligns with the final classification (it already claims central ingest server, idempotency, accumulators, query API, SSE signals as shipped). No changes needed unless board status contradicts.
- [ ] 6.4 Verify no other roadmap or production-suite sections make claims that contradict the final classification.

## 7. Reconciliation Note

- [ ] 7.1 Write `.devagent/docs/docs/reconciliation-2026-07.md` with the full inventory of all 19 classified changes.
- [ ] 7.2 Include per-change board status, repository evidence citations, and disposition (archived / superseded-by / still-active with board task references).
- [ ] 7.3 Document the 3 stale-checklist corrections with the shipped-artifact mapping for each task.
- [ ] 7.4 List all follow-up board tasks created for remaining gaps with owner and status.
- [ ] 7.5 Record the reconciliation date, executor, and board snapshot timestamp.

## 8. Validation

- [ ] 8.1 Confirm `openspec/changes/` contains only the 3 still-active changes (task-12579118, live-signal-feed-motion, public-binary-and-image-distribution).
- [ ] 8.2 Confirm `openspec/changes/archive/` contains 16 new dated directories plus all previous archives.
- [ ] 8.3 Confirm no application code, test files, or crate source was modified.
- [ ] 8.4 Confirm `roadmap.mdx` and `production-suite.md` verb tenses match the final classification.
- [ ] 8.5 Confirm each still-active change has linked board tasks with explicit owner/status.
- [ ] 8.6 Run `openspec validate --strict` to confirm the archive is structurally valid.