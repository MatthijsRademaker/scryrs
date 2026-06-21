## Context

The scryrs roadmap defines a strict delivery order. Phase 1 (Deterministic Proxy Capture) established the `scryrs record`, `.scryrs/scryrs.db`, `scryrs.json`, reference hooks, and `scryrs init` foundation. Phase 2 (Hotspot Materialization) turns persisted trace events into real hotspot outputs — the first genuine product value beyond trace collection.

Through three archived Hotspot Foundation changes (02, 03, 04), Phase 2 hotspot materialization was implemented:

- **Hotspot Foundation 02**: SQLite read path via `TraceQuery`, scoring engine in `scoring.rs`, `HotspotsReport`/`HotspotEntry` types, CLI wiring.
- **Hotspot Foundation 03**: Contract-gap closure — `runMetadata.firstEventId`/`lastEventId`, six-key tie-break with chronological evidence order, artifact-write failure handling, placeholder-surface removal from help/help-json/README.
- **Hotspot Foundation 04**: End-to-end verification — `hotspot_e2e.rs` covering the full record → SQLite → hotspots → artifact pipeline, multievent-family fixtures, snapshot assertions, empty-store and missing-store coverage.

All three changes are archived and their tasks are complete. The live code at `crates/scryrs-cli/src/lib.rs`, `crates/scryrs-core/src/scoring.rs`, and `crates/scryrs-cli/tests/hotspot_e2e.rs` proves the implementation is functional.

However, four `.devagent/docs/` pages and three `openspec/specs/` specs were written during Phase 1 or early Phase 2 and still describe placeholder hotspot behavior. They contradict the live implementation.

## Goals / Non-Goals

### Goals

1. Reconcile all stale published docs so they describe the real Phase 2 hotspot product boundary instead of placeholder or deferred behavior.
2. Reconcile all stale OpenSpec specs so no active canonical requirement still asserts placeholder hotspot behavior or 'Phase 2 out of scope' semantics.
3. Publish a Phase 2 closure evidence matrix mapping each roadmap deliverable to concrete code artifacts.
4. Document accepted limitations honestly.

### Non-Goals

- Do not modify production code, scoring logic, or CLI behavior.
- Do not add Phase 3+ scope (graph, proposal, adapter, runtime, dashboard, MCP, LLM).
- Do not change README.md (already accurate).
- Do not modify hotspot-report/spec.md or hotspot-verification/spec.md (already canonical).
- Do not delete any spec — add reconciliation headers that supersede stale requirements while preserving traceability.

## Decisions

### Decision 1: Supersede, don't delete

Stale OpenSpec specs (phase-1-closure, cli-foundation-closure, cli-golden-tests) will receive reconciliation headers that explicitly supersede their placeholder-era hotspot requirements. This preserves archive traceability while removing contract conflicts.

**Rationale**: Deleting or rewriting specs would destroy the historical record of Phase 1 closure. Adding supersedure notes preserves the audit trail and makes the contract evolution explicit.

### Decision 2: No code changes

The live implementation is complete and tested. This change is purely source-of-truth reconciliation in docs and OpenSpec artifacts. Tests serve as verification that the docs match reality, not as new behavioral validation.

**Rationale**: All three refinement round agents (architect, lead-dev, reviewer) confirmed with high confidence that Phase 2 hotspot materialization is functionally complete. The dossier identifies no contract holes in the code.

### Decision 3: Trace-hook-contract.md canonicalization wording

The phrase 'Canonicalization for hotspot grouping is deferred to Phase 2' will be updated to state that command canonicalization remains a known limitation not scheduled for any current roadmap phase. This reflects reality: Phase 2 never listed command canonicalization as a deliverable, and Phase 2 is now complete.

**Rationale**: Architected consensus from refinement rounds. Removing the Phase 2 reference eliminates the misleading implication that Phase 2 is incomplete.

### Decision 4: No dedicated closure matrix page

Instead of a separate matrix page (like Phase 1 had), the evidence matrix is embedded in the reconciliation spec (`specs/phase-2-closure/spec.md`). Updating the roadmap's Phase 2 section to show 'Completed' is sufficient.

**Rationale**: Phase 1 closure created a dedicated matrix because it was the first closure artifact and needed to establish the pattern. Phase 2 can embed evidence mapping in its reconciliation spec without duplicating infrastructure.

## Risks

| Risk | Mitigation |
|------|-----------|
| trace-hook-contract.md canonicalization wording ambiguity — whether to defer 'to a future phase' or remove the reference entirely | Use the architect-recommended wording: 'Command canonicalization remains a known limitation not scheduled for any current roadmap phase.' This removes the Phase 2 reference while honestly documenting the limitation. |
| Architecture.mdx test count drift — hardcoded test counts (~60 scryrs-core, ~85 scryrs-cli) may be stale | Verify or generalize wording during doc update; do not ship stale counts. |
| Snapshot update workflow docs in cli-v0-contract.md still reference placeholder snapshot tests | Update the Testing section to reference the real hotspot_e2e snapshots and hotspot_integration_tests, removing the placeholder-specific snapshot instructions. |

## Traceability

| Source | Link |
|--------|------|
| Task prompt | task:56573ced-fdeb-49b2-aea6-41b30f19d2bf — 'Wrap up phase 2' |
| Exploration dossier | dossier:2026-06-21T18:34:37.431Z — identifies the gap as source-of-truth conflict, not code gap |
| Architect recommendation | round:1:agent:swarm-architect — 'Phase 2 hotspot materialization is fully implemented ... remaining work is entirely documentation and OpenSpec reconciliation' |
| Lead-dev recommendation | round:1:agent:swarm-lead-dev — 'The gap is entirely source-of-truth reconciliation' |
| Reviewer recommendation | round:1:agent:swarm-reviewer — 'Approve the Phase 2 closure change with requirements to reconcile stale docs and OpenSpec specs' |
| Hotspot Foundation 03 (archived) | openspec/changes/archive/2026-06-21-task-649bd576-c00f-40d7-8edb-79e22b1783d5 — completed contract reconciliation, placeholder-surface removal, edge-case tests |
| Hotspot Foundation 04 (archived) | openspec/changes/archive/2026-06-21-task-6dad120d-e43c-44d1-a257-d56de11ce553 — completed E2E verification and snapshots |
| Live hotspot-report spec | openspec/specs/hotspot-report/spec.md — canonical real Phase 2 hotspot contract |
| Live hotspot-verification spec | openspec/specs/hotspot-verification/spec.md — canonical E2E verification requirements |