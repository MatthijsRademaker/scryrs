## Context

The scryrs project already ships:
- A versioned `ProposalDocument` contract in `scryrs-types` with six target types, deterministic content-addressed IDs, validation, and inbox filename computation.
- An inbox layout defined by the proposal contract: `.scryrs/proposals/{id}.json` files, non-authoritative, review-only.
- Hotspot artifact (`.scryrs/hotspots.json`) with scored, evidence-carrying entries.
- Graph artifact (`.scryrs/graph.json`) with deterministic node/edge identity and `EvidenceLink` provenance.
- CLI patterns for `graph` and `route` commands: load artifacts, validate, delegate, write output, feature-gated.
- A `scryrs-curator` crate wired in CLI via the `curator` feature but containing only a single-entry placeholder.

What is missing is the generation engine and CLI command that connects these pieces.

## Goals / Non-Goals

### Goals
- Register `scryrs propose <PATH>` in CLI dispatch, help text, help JSON, and README.
- Load `.scryrs/hotspots.json` and `.scryrs/graph.json`; fail with exit code 2 and command-specific error on missing or malformed inputs.
- Generate zero or more validated `ProposalDocument` files under `.scryrs/proposals/` for `docs_note`, `adr`, `skill`, `memory_patch`, and `semantic_graph_grouping` target types.
- Every proposal must carry non-empty rationale, non-empty proposed content, and non-empty evidence using the existing `EvidenceLink` vocabulary.
- Deterministic output: same inputs → same proposal set with stable content-addressed IDs. `createdAt` derived from hotspot `generatedAt`.
- Source-of-truth non-mutation: no writes to `.scryrs/graph.json`, `.scryrs/routes.json`, docs, ADRs, skills, or memory truth.
- Upsert-only semantics: write/overwrite current candidates; do not remove stale proposals.

### Non-Goals
- Review/accept/reject lifecycle, inbox status fields, or dashboard review UI.
- Auto-publishing or mutating docs, ADRs, skills, memory truth, graph, or routes.
- LLM-authored drafting or non-deterministic generation.
- Changing graph build, route generation, live-hotspot APIs, or hotspot scoring.
- Generating `debugging_playbook` proposals.
- Automatic stale proposal cleanup.

## Decisions

### Decision 1: Command shape — flat `scryrs propose <PATH>`
**Choice**: Single positional PATH argument, matching the established `graph`/`route` pattern.
**Rationale**: Consistency with existing artifact-building commands. No flags needed for V1. Feature-gated behind `curator` like `graph`/`route` are gated behind `graph`.
**Traceability**: architect round 1, lead-dev round 1, reviewer round 1 (all agreed).

### Decision 2: `createdAt` determinism — use hotspot `generatedAt`
**Choice**: `ProposalDocument.createdAt` SHALL equal the `generatedAt` field from the loaded hotspot report.
**Rationale**: AC-3 requires deterministic output on identical inputs. Hotspot `generatedAt` is stable for a given artifact and provides a meaningful timestamp. Wall-clock time would break byte-identical reruns.
**Traceability**: architect round 1 (recommended), reviewer round 1 (flagged as blocker, resolved by this choice). lead-dev's wall-clock preference is overridden — it conflicts with AC-3 determinism.

### Decision 3: Evidence-to-target-type heuristics
**Choice**: V1 uses concrete, testable thresholds:
- `docs_note`: every hotspot entry (baseline, always generated)
- `skill`: hotspot entries where outcome includes `Failure` events
- `memory_patch`: hotspot entries with failure-ratio ≥ 0.5 (failure events ≥ half of total events) and score ≥ 4
- `adr`: subject strings appearing across ≥2 distinct subject kinds with aggregate score ≥ 10
- `semantic_graph_grouping`: graph node families with ≥2 distinct subject kinds sharing the same subject stem (e.g., `file:auth`, `symbol:auth`) and at least one shared hotspot-backed evidence link
**Rationale**: All three reviewers identified the initial sketch language as too vague. These thresholds are concrete, testable, and deterministic.
**Traceability**: architect round 1 (suggested thresholds), lead-dev round 1 (suggested subjectKind+score heuristics), reviewer round 1 (flagged as blocker, resolved).

### Decision 4: Rerun semantics — upsert-only
**Choice**: The command writes/overwrites `.scryrs/proposals/{id}.json` for each currently-generated candidate. It does NOT remove stale proposals from previous runs. When evidence/rationale changes but proposedContent stays the same (same ID), overwrite the file with updated fields.
**Rationale**: Preserves human-reviewed proposals that may have been accepted. Content-addressed IDs ensure identical content always maps to the same file.
**Traceability**: lead-dev round 1 (recommended), reviewer round 1 (flagged, resolved by explicit semantics).

### Decision 5: Curator API surface
**Choice**: `pub fn generate_proposals(graph: &KnowledgeGraphDocument, hotspots: &[HotspotEntry]) -> Vec<ProposalDocument>`
**Rationale**: Clean entry point accepting both required inputs, returning all generated proposals. CLI handles validation, directory creation, and file I/O.
**Traceability**: reviewer round 1 (flagged as blocker, resolved with explicit signature).

### Decision 6: `debugging_playbook` explicitly out of scope
**Choice**: The command SHALL NOT generate `debugging_playbook` proposals, even though the `ProposalTargetType` enum supports it.
**Rationale**: Dossier non-goals and task scope explicitly limit to five target types. The contract supports six; generation covers five.
**Traceability**: dossier nonGoals, reviewer round 1 risk flag.

## Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| ADR candidates are weak from hotspot-only heuristics | Medium | Narrow ADR rule (cross-kind clusters only); document improvement path |
| Semantic grouping false positives from stem matching | Medium | Require ≥2 distinct subject kinds + shared evidence link |
| Hotspot `generatedAt` changes on hotspot re-run, producing different proposal timestamps | Low | Document behavior; deterministic content-addressed IDs are unaffected |
| Feature gating CI breakage if propose code is not `#[cfg(feature = "curator")]` gated | Low | Follow existing pattern from `graph.rs`/`route.rs` |
| Help JSON ordering requires insertion in sorted position | Low | Insert `propose` entry between `init` and `record` (sorted by name) |

## Traceability

- Task: `597db7ef-f2da-4d5e-9459-7c19601a5e85` (Curator Foundation 02)
- Dossier: `2026-06-27T20:54:26.261Z`
- Round 1 decisions: architect (command shape, createdAt, V1 rules), lead-dev (thresholds, crate deps, upsert-only), reviewer (rerun semantics, curator API, feature-gating)
- Artifact base: `openspec/changes/task-597db7ef-f2da-4d5e-9459-7c19601a5e85@initial`
- Specs: `proposal-contract` (ProposalDocument schema), `graph-contract` (EvidenceLink vocabulary, node identity), `graph-build` (artifact loading patterns)
- Source: `scryrs-types/src/lib.rs` (ProposalDocument, compute_id, validate), `scryrs-curator/src/lib.rs` (current placeholder), `scryrs-cli/src/dispatch.rs` (command registration), `scryrs-cli/src/graph.rs` (artifact loading pattern), `help_text.rs`, `help_json.rs`, `dispatch_tests.rs`