## Context

The scryrs project has shipped substantial CLI surfaces since the narrative roadmap and production-suite pages were last updated. The advisor-built surface now includes:

- `scryrs proposals list|accept|reject` (review CLI)
- `scryrs publish markdown|rspress` (publishing adapters)
- `scryrs route explain <PATH> --query <TEXT>` (deterministic explain)
- `scryrs route bundle <PATH> --query <TEXT> --limit <N>` (bounded planning)
- `debugging_playbook` proposal generation (FailedLookup >= 2)

These shipped surfaces are either missing from or misrepresented as future work in `roadmap.mdx` and `production-suite.md`. Two other doc pages (`proposals.md`, `route-manifests.md`) are already accurate but need audit confirmation.

## Goals / Non-Goals

### Goals

1. Roadmap Phases 6-8 explicitly distinguish shipped command surfaces from remaining product-complete gaps.
2. Fix the stale `debugging_playbook` claim in Phase 6.
3. Production-suite route/runtime gap rewritten from missing-command to usefulness/closure gap.
4. Proposals.md and route-manifests.md confirmed accurate via audit.
5. Docs build succeeds through existing `scripts/check` lane.

### Non-Goals

- No Rust/TypeScript runtime behavior, CLI contracts, or adapter changes.
- No publication or editing of canonical OpenSpec specs.
- Do not claim runtime retrieval is semantically useful or product-complete.
- Do not broaden scope into unrelated roadmap cleanup outside the named pages.
- `graph.md` "Route → Runtime retrieval (future)" is out of scope for this change. The sentence is technically accurate (autonomous context loading is not shipped) and changing it risks scope creep.

## Decisions

### Decision 1: Surgical edit approach with truth baseline

**Choice:** Edit only `roadmap.mdx` and `production-suite.md`; audit `proposals.md` and `route-manifests.md`. Use `cli-v0-contract.md` and Rust CLI/runtime sources as the executable truth baseline.

**Rationale:** The dossier and all three refinement reviewers confirmed this approach. Proposals.md and route-manifests.md already match shipped behavior; over-editing them risks introducing drift. The truth baseline is unambiguous — `crates/` is executable truth.

**Sources:** Accepted decisions 1-swarm-architect-recommendation, 1-swarm-lead-dev-recommendation, 1-swarm-reviewer-recommendation.

### Decision 2: Explicit partial-delivery marker taxonomy

**Choice:** Use consistent language across Phases 6-8: a "Partial delivery" note naming the shipped command surface, followed by explicit remaining product gaps. Use ✅ Shipped markers for delivered milestones in the near-term milestones table.

**Rationale:** The risk identified by all reviewers is that partial-delivery markers could confuse readers if the distinction between "shipped command" and "product-complete phase" is not consistent. A single taxonomy applied uniformly across all three phases prevents this.

**Sources:** Round outputs from swarm-architect, swarm-lead-dev, swarm-reviewer.

### Decision 3: graph.md "Route → Runtime retrieval (future)" out of scope

**Choice:** Do not edit graph.md in this change.

**Rationale:** The sentence is technically accurate — autonomous context loading is not shipped. The three reviewers flagged this as a potential scope creep risk. The named scope is roadmap, production-suite, proposals, and route-manifests. Treating graph.md as an advisory noting rather than required scope is the correct posture.

**Sources:** Non-blocking note from swarm-reviewer; question from swarm-lead-dev; question from swarm-architect.

### Decision 4: Verify through scripts/check

**Choice:** Use the existing `scripts/check` lane (which runs `verify-docs-publish`) as the only verification path.

**Rationale:** All three reviewers confirmed this is the correct approach. Do not invent a bespoke build path. `scripts/check` already runs real `scryrs publish markdown` and `scryrs publish rspress`, then builds `.devagent/docs` and checks generated `llms.txt` / `llms-full.txt`.

**Sources:** Accepted decisions; dossier `likelyAffectedAreas`.

## Risks

| Risk | Mitigation |
|------|-----------|
| Phase 8 presented as product-complete after adding partial-delivery marker | Explicitly state "deterministic manifest matching only, no automatic context loading" in the partial-delivery note |
| Phase 6 remaining gaps understated after adding shipped review CLI | Keep the explicit gaps: no dashboard review UX, no broader accepted-evidence consumers |
| Docs verification fails due to build environment | `scripts/check` is the canonical lane; if it fails, debug the lane rather than bypassing it |
| Roadmap Delivery Phases table conflates per-phase "done" with product-complete status | Use consistent "Partial delivery" language with explicit remaining-gap text per phase |

## Traceability

| Source | Evidence type | Used for |
|--------|-------------|----------|
| Task 7f0b6e6e-7c4b-46a7-ac4d-70036c6c0529 prompt | Backlog request, scenarios, acceptance criteria | Scope boundary, non-Goals |
| Dossier 2026-07-06T20:41:38.760Z | Exploration findings, affected areas, acceptance criteria | Problem framing, affected pages, verification path |
| Accepted decision 1-swarm-architect-recommendation | Round 1 architect review | Phase 6-8 partial delivery markers, suggested requirements |
| Accepted decision 1-swarm-lead-dev-recommendation | Round 1 lead dev review | Truth baseline (cli-v0-contract.md + Rust sources), risk profile |
| Accepted decision 1-swarm-reviewer-recommendation | Round 1 reviewer review | Blockers/non-blocking: debugging_playbook, review CLI, P4 milestone |
| Validated round outputs | All three agent outputs | Consolidated suggested requirements, risks, questions |
| roadmap.mdx (current) | Source doc | Stale claims identified |
| production-suite.md (current) | Source doc | Gap text, P4 milestone |
| proposals.md (current) | Source doc | Audit target — confirmed accurate |
| route-manifests.md (current) | Source doc | Audit target — confirmed accurate |
| crates/scryrs-curator/src/lib.rs | Executable truth | debugging_playbook generation at FailedLookup >= 2 |
| crates/scryrs-cli/src/proposals.rs | Executable truth | Shipped review CLI |
| crates/scryrs-cli/src/publish.rs | Executable truth | Shipped publish CLI |
| crates/scryrs-cli/src/route_explain.rs | Executable truth | Shipped route explain |
| crates/scryrs-runtime/src/lib.rs | Executable truth | Deterministic explain matching, packed relevance |
| crates/scryrs-cli/src/route.rs | Executable truth | loadTarget derivation, non_loadable boundaries |
| scripts/check | Executable truth | Docs verification lane |
| scripts/verify-docs-publish | Executable truth | Docs publish/build verification |