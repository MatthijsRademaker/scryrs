## Context

`scryrs-runtime` currently defines an ad-hoc `RouteHint { target, reason }` struct — a placeholder with six basic preservation tests. The route-manifest pipeline (`scryrs graph` → `scryrs route`) is shipped and stable: `RouteManifestDocument` carries all fields needed for route explanation (`id`, `subjectKind`, `label`, `target`, `evidenceLinks`, `grouping`). The product roadmap (Phase 8 Runtime Retrieval) lists route-hint schema and `scryrs route explain` as deferred work, with the schema coming before the CLI command (production-suite near-term item 5).

The existing crate topology assigns shared wire contracts to `scryrs-types` and agent-facing foundation helpers to `scryrs-runtime`. `scryrs-types` already owns `RouteManifestDocument`, `RouteEntry`, `EvidenceLink`, `ProposalDocument`, and `ProposalReviewDecision` — all with independent schema version constants. This boundary is correct for the new hint contract.

The refinement room confirmed three design resolutions: (1) scope is shared schema + runtime function + tests + docs — the `scryrs route explain` CLI command is deferred; (2) rank is a deterministic 1-based ordinal from manifest sort order, relevance is optional/deferred; (3) evidence citations reuse `EvidenceLink` directly from source `RouteEntry` entries.

## Goals

1. Define a versioned `RouteHintDocument` envelope and `RouteHintItem` entry in `scryrs-types` with an independent `HINT_SCHEMA_VERSION`, following the existing pattern of per-contract version constants.
2. Implement a deterministic, model-free `hints_from_manifest` function in `scryrs-runtime` that consumes `&RouteManifestDocument` and produces `RouteHintDocument`.
3. Preserve identity boundaries: one `RouteHintItem` per `RouteEntry`, no collapsing of distinct identities like `file:auth`, `search:auth`, and `symbol:auth`.
4. Make rank a deterministic 1-based ordinal derived from manifest entry sort order (by `id` ascending).
5. Keep `relevance` as an optional field with value `None` (deferred for future enhancement).
6. Derive `reason` from a deterministic template citing the source route entry identity and evidence count.
7. Copy `evidence` directly from `RouteEntry.evidenceLinks` — no new citation types, no file I/O beyond manifest consumption.
8. Update CLI help-text, help-json, `cli-v0-contract.md`, and `route-manifests.md` with the hint contract shape, JSON examples, and explicit deferred-ranking language.
9. Add identity-preservation and determinism regression tests in `scryrs-runtime`.

## Non-Goals

- No `scryrs route explain` CLI command implementation — the subcommand dispatch, clap registration, handler, and dispatch-tests are deferred.
- No model-based ranking, fuzzy retrieval, or hidden heuristics.
- No mutation of `.scryrs/graph.json`, `.scryrs/routes.json`, `.scryrs/proposals/`, `.scryrs/accepted/`, or `.scryrs/rejected/`.
- No opening of proposal or review-artifact directories during hint generation.
- No new citation reference types — hints reuse `EvidenceLink` directly.
- No long-term ranking formula — `relevance` remains `None` and is explicitly deferred.
- No change to graph build, route-manifest materialization, or proposal generation semantics.

## Decisions

### D1: Schema ownership — `scryrs-types`
**Decision**: `RouteHintDocument` and `RouteHintItem` are defined in `crates/scryrs-types/src/lib.rs` with an independent `HINT_SCHEMA_VERSION = "1.0.0"`.
**Rationale**: `scryrs-types` already owns every cross-crate wire contract (`RouteManifestDocument`, `ProposalDocument`, `ProposalReviewDecision`, `KnowledgeGraphDocument`, `EvidenceLink`) with independent version constants. The route hint schema must be sharable between `scryrs-runtime` (producer), `scryrs-cli` (future command), and any future server or library consumers. Putting it in `scryrs-types` follows the established pattern and prevents circular dependencies.
**Sources**: swarm-architect (round 1), swarm-lead-dev (round 1), `crates/scryrs-types/src/lib.rs`, `.devagent/docs/docs/architecture.mdx`.

### D2: Envelope pattern — `RouteHintDocument`
**Decision**: The top-level contract is a versioned envelope `RouteHintDocument { schemaVersion: String, hints: Vec<RouteHintItem> }`.
**Rationale**: Every existing contract in `scryrs-types` follows this pattern (`RouteManifestDocument`, `ProposalDocument`, `KnowledgeGraphDocument`). A versioned envelope enables independent schema evolution, self-describing serialization, and future CLI output. The `HINT_SCHEMA_VERSION` is independent so the hint contract can evolve without forcing re-versioning of graph or route contracts.
**Sources**: swarm-architect (round 1), swarm-lead-dev (round 1), existing `RouteManifestDocument` pattern.

### D3: Rank semantics — deterministic ordinal
**Decision**: `rank` is a `u32` 1-based ordinal equal to the position of the source `RouteEntry` in the manifest's `routes` array (sorted by `id` ascending).
**Rationale**: Manifest entries are already deterministically sorted. Using that order as rank is trivial, deterministic, and model-free. It satisfies the acceptance criteria for an explicit rank field without prematurely freezing a long-term scoring formula. The contract docs explicitly state that rank is a deterministic placeholder, not a final ranking policy.
**Sources**: swarm-lead-dev (round 1), `crates/scryrs-cli/src/route.rs` (sort-by-id), swarm-reviewer blocker-2 resolution (round 1).

### D4: Relevance — optional/deferred
**Decision**: `relevance` is `Option<u32>` with value `None` in the initial implementation.
**Rationale**: The dossier open question #4 and the reviewer blocker #2 flagged that relevance semantics must not be frozen prematurely. Making it optional with a `None` default communicates that this is a deferred dimension without committing to a specific computation. Future enhancements (e.g., evidence link score aggregation, text-match tiers) can populate it without breaking the schema. Contract docs explicitly label it as "deferred for future enhancement."
**Sources**: swarm-lead-dev (round 1), swarm-reviewer blocker-2 resolution (round 1), `.devagent/docs/docs/roadmap.mdx`.

### D5: Evidence citations — reuse `EvidenceLink`
**Decision**: `RouteHintItem.evidence` is `Vec<EvidenceLink>`, copied directly from the source `RouteEntry.evidenceLinks`.
**Rationale**: `EvidenceLink` already captures full provenance (source kind, subject, row IDs, doc ref, score). Creating a new citation reference type adds indirection without adding information. The hint is a downstream projection — it should not synthesize new evidence or open proposal/review directories. Copying the manifest's evidence links keeps the hint self-contained and traceable to the graph artifact.
**Sources**: swarm-architect (round 1), swarm-lead-dev (round 1), swarm-reviewer blocker-3 resolution (round 1).

### D6: Reason template — deterministic, cites entry identity
**Decision**: The `reason` field is generated by a deterministic template: `"Route '{label}' ({id}): {N} evidence link(s), subject kind {subjectKind}"`, where `{label}`, `{id}`, `{subjectKind}` come from the source `RouteEntry` and `{N}` is `evidence_links.len()`.
**Rationale**: The task requires evidence-backed explanation text. The reason must not be LLM-authored prose (assumption #2). A template that cites the route entry identity and evidence count provides traceability without duplicating full `EvidenceLink` JSON. The lead-dev recommended citing route entry identity and evidence link ordinals to keep output concise; this template follows that guidance while using evidence count as the simplest ordinal reference.
**Sources**: swarm-architect (round 1), swarm-lead-dev (round 1), task assumptions.

### D7: CLI command scope — deferred
**Decision**: This task produces the shared schema, runtime producer, tests, and contract docs. The `scryrs route explain` CLI command is explicitly deferred to a follow-up task.
**Rationale**: The task acceptance criteria mention schema, tests, and contract docs — not a new CLI command. The production-suite near-term task order lists `scryrs route explain` as item 5, sequenced after foundation schema work. A shared contract enables CLI, server, or library consumers without waiting for a specific command implementation. Both the architect and lead-dev recommended this scope boundary.
**Sources**: swarm-architect (round 1), swarm-lead-dev (round 1), task acceptance criteria, `.devagent/docs/docs/production-suite.md`.

### D8: Identity preservation
**Decision**: `hints_from_manifest` produces exactly one `RouteHintItem` per `RouteEntry`. No merging, deduplication, or collapsing occurs based on shared labels or subject text. The identity boundary enforced at the route-manifest level (one entry per graph node) extends unchanged to the hint projection.
**Rationale**: The task scenario explicitly requires that `file:auth`, `search:auth`, and `symbol:auth` remain distinct hints. The route-manifest pipeline already enforces this (one entry per node, no label-based merging). The hint producer inherits this guarantee by simply projecting 1:1 from manifest entries. Collapsing identities would require explicit graph evidence (e.g., a `contains` edge from a parent node) — which would already be reflected in the manifest's `grouping` field.
**Sources**: task scenarios, `openspec/specs/route-manifest/spec.md`, `crates/scryrs-cli/src/dispatch_tests.rs`.

## Risks

| Risk | Mitigation |
|------|-----------|
| R1: Schema proliferation — adding `HINT_SCHEMA_VERSION` as another independent constant adds maintenance burden. | Follows the existing pattern exactly. Each contract already has its own version constant. Version bumps only happen on wire-breaking changes. |
| R2: Premature rank/relevance freeze — if rank semantics are specified too narrowly, future enhancements may be constrained. | Contract docs explicitly label both fields as deterministic placeholders, not final ranking policy. Rank is defined as a simple ordinal; relevance is `None` and deferred. |
| R3: Evidence citation duplication — if reason text redundantly restates what `EvidenceLink` already captures, output becomes bloated. | Reason is a compact template citing entry identity and evidence count — it does not serialize or duplicate `EvidenceLink` content. Consumers can inspect the `evidence` array for full provenance. |
| R4: Over-scoping — if scope creeps to include `scryrs route explain` CLI command, it delays the shared contract that runtime consumers need. | Scope is explicitly bounded in the task list and design decisions. The CLI command is deferred. |
| R5: Existing `RouteHint` placeholder tests break. | The six existing tests are replaced by new tests for `hints_from_manifest`. There are no external consumers of the placeholder. |

## Traceability

- Task: `40c5b995-5010-4cae-be7f-8a859e86468c`
- Exploration dossier: `2026-06-28T20:21:40.899Z`
- Accepted decisions: `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, `1-swarm-reviewer-recommendation`
- Round 1 agent outputs: swarm-architect, swarm-lead-dev, swarm-reviewer
- Repository sources: `crates/scryrs-types/src/lib.rs`, `crates/scryrs-runtime/src/lib.rs`, `crates/scryrs-cli/src/dispatch_tests.rs`, `crates/scryrs-cli/src/help_text.rs`, `crates/scryrs-cli/src/help_json.rs`, `.devagent/docs/docs/route-manifests.md`, `.devagent/docs/docs/cli-v0-contract.md`, `.devagent/docs/docs/production-suite.md`, `.devagent/docs/docs/architecture.mdx`, `openspec/specs/route-manifest/spec.md`