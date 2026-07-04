# OpenSpec Reconciliation — 2026-07-04

**Reconciliation date**: 2026-07-04
**Executor**: swarm-worker (task `12579118-0f21-4a96-b20f-9c3258b53add`)
**Board snapshot timestamp**: 2026-07-04 (runtime)

## Summary

Reconciled 20 non-archived OpenSpec change directories under `openspec/changes/` against board task status and repository truth. Result: 17 archive-ready, 1 superseded, 2 still-active. 18 changes moved to archive. Zero application code modified.

## Classification Inventory

### Archive-Ready — Fully Checked and Shipped (14)

All 13 changes had 100% `[x]` checklist completion and board status Done (5). Repository evidence confirmed shipped artifacts.

| Change | Board ID | Board Status | Evidence |
| --- | --- | --- | --- |
| `add-init-interactive-wizard` | *(named directory)* | Verified shipped | `scryrs init` wizard in `crates/scryrs-cli/` |
| `add-scryrs-debug-logging` | *(named directory)* | Verified shipped | `SCRYRS_DEBUG` env var in `crates/scryrs-cli/` |
| `allow-source-repo-pi-init` | *(named directory)* | Verified shipped | Pi source-repo init path |
| `default-live-mode` | *(named directory)* | Verified shipped | Verified by `scripts/verify-live-hotspots` |
| `native-harness-hook-command` | *(named directory)* | Verified shipped | `scryrs hook` command |
| `observer-first-disable-bash-default` | *(named directory)* | Verified shipped | Bash gating behind `SCRYRS_DEBUG` |
| `rename-pi-extension-to-scryrs` | *(named directory)* | Verified shipped | Extension renamed |
| `split-init-and-setup-commands` | *(named directory)* | Verified shipped | Init/setup split |
| `sync-scryrs-swarm-scaffold` | *(named directory)* | Verified shipped | Scaffold sync |
| `task-53fc3cbb` | `53fc3cbb-ac15-4ad1-9e18-dc510931c683` | 5 (Done) | 44/44 checked; dashboard shipped at `crates/scryrs-dashboard/` |
| `task-5c682a97` | `5c682a97-5d98-49c9-a5f7-b93ec7b036f7` | 5 (Done) | 11/11 checked; `ProposalReviewDecision` in `crates/scryrs-types/` |
| `task-9b0c8757` | `9b0c8757-a49f-4337-8f49-1f324778eba6` | 5 (Done) | 14/14 checked; test infrastructure shipped |
| `task-9b98b3fd` | `9b98b3fd-93f6-4b08-b4b5-cb30e5f9d486` | 5 (Done) | 25/25 checked; CLI v0 contract shipped (product evolved multi-command, change was completed regardless) |
| `curated-skills-for-light-models` | *(named directory)* | Verified shipped | 21/21 checked; curated recipe skill portfolio, `scripts/skill-lint`, and agent routing updates shipped at `.pi/skills/` |

### Archive-Ready — Stale Checklist, Shipped Code (3)

All 3 had 0 checked / all unchecked tasks.md despite described features being fully shipped. Checklists corrected with `<!-- completed outside OpenSpec: <path> -->` annotations before archival. Board status Done (5) for all three.

| Change | Board ID | Board Status | Shipped Artifact | Corrected Tasks |
| --- | --- | --- | --- | --- |
| `task-e5d582d9` | `e5d582d9-8d71-4c4c-baa6-d4ef1593d731` | 5 (Done) | `scripts/verify-live-hotspots`, `scripts/verification/live-hotspots-e2e.mjs`, `scripts/verification/README.md` | 12 items, sections 1-3 |
| `task-2abf2484` | `2abf2484-5e6e-4cbb-bb50-1f59a2d753a3` | 5 (Done) | `crates/scryrs-server/`, `crates/scryrs-types/src/lib.rs`, `crates/scryrs-cli/src/dispatch.rs` | 21 items, sections 1-5 |
| `task-cc52db89` | `cc52db89-f03d-4461-9b94-d93e237a8f99` | 5 (Done) | `crates/scryrs-cli/src/help_json.rs`, `crates/scryrs-cli/src/dispatch.rs`, `crates/scryrs-cli/tests/dispatch_tests.rs`, `.devagent/docs/docs/cli-v0-contract.md` | 24 items, sections 1-5 |

### Superseded (1)

| Change | Superseded By | Board Status | Disposition |
| --- | --- | --- | --- |
| `task-3797be85` | `2026-06-29-task-206a6986-5940-4824-a202-c7c759da4548` (Live Dashboard 02) | 5 (Done) | `## Superseded By` section added to proposal.md; canonical spec at `openspec/specs/live-dashboard-mode/spec.md`; delta spec archived as historical-only |

### Still-Active (2)

| Change | Unchecked Items | Board Task | Board Task Status |
| --- | --- | --- | --- |
| `live-signal-feed-motion` | 5.1-5.3 (manual visual verification) | `c2ba1718-b508-46ca-8a92-04dc938e0439` (Dashboard Verification 01) | 1 (Backlog) |
| `public-binary-and-image-distribution` | 6.1-6.4 (release tag push, visibility flip, public verification, release notes) | `4349cc03-d831-42a1-9e2f-09e3aeeb12a1` (Release Ops 01) | 1 (Backlog) |

## Stale-Checklist Correction Mappings

### task-e5d582d9 (Live Integration 01)

| Section | Tasks | Shipped Artifact |
| --- | --- | --- |
| 1. Docker-backed verification entrypoint (1.1-1.3) | 3 items | `scripts/verify-live-hotspots` |
| 2. Live-hotspots E2E fixture (2.1-2.6) | 6 items | `scripts/verification/live-hotspots-e2e.mjs` |
| 3. Documentation (3.1-3.3) | 3 items | `scripts/verification/README.md` |

### task-2abf2484 (Live Hotspot Foundation 02)

| Section | Tasks | Shipped Artifact |
| --- | --- | --- |
| 1. Shared contract and type updates (1.1-1.3) | 3 items | `crates/scryrs-types/src/lib.rs` |
| 2. New server crate and SQLite store (2.1-2.5) | 5 items | `crates/scryrs-server/` |
| 3. Batch ingest handling (3.1-3.5) | 5 items | `crates/scryrs-server/` |
| 4. CLI and discovery integration (4.1-4.3) | 3 items | `crates/scryrs-cli/src/dispatch.rs` |
| 5. Verification and regressions (5.1-5.5) | 5 items | `crates/scryrs-server/` |

### task-cc52db89 (CLI Foundation 04)

| Section | Tasks | Shipped Artifact |
| --- | --- | --- |
| 1. --help-json argument parser (1.1-1.2) | 2 items | `crates/scryrs-cli/src/dispatch.rs` |
| 2. Surface document serialization (2.1-2.6) | 6 items | `crates/scryrs-cli/src/help_json.rs` |
| 3. CLI contract documentation (3.1-3.2) | 2 items | `.devagent/docs/docs/cli-v0-contract.md` |
| 4. Tests (4.1-4.9) | 9 items | `crates/scryrs-cli/tests/dispatch_tests.rs` |
| 5. Validation (5.1-5.5) | 5 items | `crates/scryrs-cli/` |

## Supersession Chain

`task-3797be85-644b-43c1-8248-ef2765372224` (Live Dashboard 01 — Define read-only live server mode for dashboard) was superseded by archived `2026-06-29-task-206a6986-5940-4824-a202-c7c759da4548` (Live Dashboard 02 — Implement live hotspot rankings and signal timeline UI). The successor's work was promoted to canonical spec at `openspec/specs/live-dashboard-mode/spec.md`. Both the architect and lead dev confirmed task-206a6986 as the authoritative implementation. The superseded change's proposal.md was annotated with an explicit `## Superseded By` section referencing the archived successor and canonical spec before archival.

## Delta Spec Handling

Two completed changes had delta specs that were archived as historical-only:

- `task-5c682a97/specs/proposal-review-contract/spec.md` — canonical spec already exists at `openspec/specs/proposal-contract/`
- `task-53fc3cbb/specs/dashboard-phase-goal/spec.md` — canonical spec already exists at `openspec/specs/dashboard-frontend-stack/`

No content was synced to canonical specs. Both deltas are preserved in the archive directories for historical reference.

## Documentation Changes

### roadmap.mdx

- Phase 3 header: changed to "✅ Delivered"; "Required deliverables:" → "Delivered:"; verb tenses updated to reflect shipped state
- Phase 4 header: changed to "✅ Delivered"; "Required deliverables:" → "Delivered:"; verb tenses updated to reflect shipped state
- Deferred sub-items in accepted limitations sections left unchanged

### production-suite.md

- Live hotspots row: "Dashboard live mode ... missing" → "dashboard live mode shipped"; remaining gap narrowed to browser automation and visual verification only
- P3 milestone: marked "✅ Shipped" since dashboard live mode, server API client, signal timeline, and reconnect behavior are all delivered

## Follow-Up Board Tasks

| Board Task ID | Title | Owner | Status | Tracks |
| --- | --- | --- | --- | --- |
| `c2ba1718-b508-46ca-8a92-04dc938e0439` | Dashboard Verification 01 — Add automated live signal feed browser smoke | Unassigned | Backlog | `live-signal-feed-motion` tasks 5.1-5.3 |
| `4349cc03-d831-42a1-9e2f-09e3aeeb12a1` | Release Ops 01 — Complete public binary/image release and anonymous verification | Unassigned | Backlog | `public-binary-and-image-distribution` tasks 6.1-6.4 |

Both tasks are flagged for maintainer triage per the reconciliation risk mitigation plan.

## Archive Manifest

18 changes archived to `openspec/changes/archive/2026-07-04-<name>/`:

1. `2026-07-04-add-init-interactive-wizard/`
2. `2026-07-04-add-scryrs-debug-logging/`
3. `2026-07-04-allow-source-repo-pi-init/`
4. `2026-07-04-default-live-mode/`
5. `2026-07-04-native-harness-hook-command/`
6. `2026-07-04-observer-first-disable-bash-default/`
7. `2026-07-04-rename-pi-extension-to-scryrs/`
8. `2026-07-04-split-init-and-setup-commands/`
9. `2026-07-04-sync-scryrs-swarm-scaffold/`
10. `2026-07-04-task-2abf2484-5e6e-4cbb-bb50-1f59a2d753a3/`
11. `2026-07-04-task-3797be85-644b-43c1-8248-ef2765372224/`
12. `2026-07-04-task-53fc3cbb-ac15-4ad1-9e18-dc510931c683/`
13. `2026-07-04-task-5c682a97-5d98-49c9-a5f7-b93ec7b036f7/`
14. `2026-07-04-task-9b0c8757-a49f-4337-8f49-1f324778eba6/`
15. `2026-07-04-task-9b98b3fd-93f6-4b08-b4b5-cb30e5f9d486/`
16. `2026-07-04-task-cc52db89-f03d-4461-9b94-d93e237a8f99/`
17. `2026-07-04-task-e5d582d9-8d71-4c4c-baa6-d4ef1593d731/`
18. `2026-07-04-curated-skills-for-light-models/`

## Validation

- [x] `openspec/changes/` contains 3 still-active changes: `task-12579118`, `live-signal-feed-motion`, `public-binary-and-image-distribution`
- [x] `openspec/changes/archive/` contains 18 new 2026-07-04-prefixed entries plus 59 prior archives = 77 total
- [x] No application code, test files, or crate source modified
- [x] `roadmap.mdx` Phase 3 and Phase 4 headings marked ✅ Delivered, tenses aligned with shipped state
- [x] `production-suite.md` live hotspots row and P3 milestone updated
- [x] Both still-active changes have linked board tasks in their proposal.md
- [x] All 3 stale checklists corrected with artifact citations
- [x] Supersession chain documented for task-3797be85
