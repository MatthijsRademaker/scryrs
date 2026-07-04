## Context

scryrs already ships a lifecycle-free `ProposalDocument` contract, deterministic proposal generation into `.scryrs/proposals/`, and explicit non-mutation guarantees for graph, route, docs, and memory truth. The missing boundary is durable review evidence: accepted and rejected decisions cannot yet be recorded without either remaining ephemeral or overloading proposal inbox files with lifecycle state.

The repository evidence is already aligned on the next slice:

- `crates/scryrs-types` owns versioned shared contracts, validation, and serde wire formats.
- `openspec/specs/proposal-contract/spec.md` explicitly keeps `ProposalDocument` free of acceptance lifecycle fields.
- `.devagent/docs/docs/production-suite.md` already describes `.scryrs/proposals/` as review inbox only and `.scryrs/accepted/` as reviewed evidence / approved knowledge.
- The roadmap places accepted/rejected artifact contract before any accepted-evidence graph ingestion or publishing.

This task therefore defines the review-decision contract only. It does not implement review commands or downstream consumption.

## Goals / Non-Goals

### Goals

- Define a versioned `ProposalReviewDecision` contract in `scryrs-types` for explicit accepted and rejected proposal outcomes.
- Reuse existing `EvidenceLink`, `ProposalTargetType`, `ProposedContent`, and `SemanticGraphGrouping` types instead of introducing parallel content or provenance models.
- Require reviewer metadata, rationale, decision timestamp, and non-empty provenance for every review decision.
- Preserve exact `sourceNodeIds` when accepted content is a `semantic_graph_grouping`.
- Keep `ProposalDocument` unchanged and `.scryrs/proposals/` review-only.
- Document the boundary between proposal inbox artifacts and reviewed evidence artifacts.

### Non-Goals

- No `scryrs accept` or `scryrs reject` command surface.
- No dashboard review UI or reviewer assignment workflow.
- No graph build, route generation, docs adapter, or memory system consumption of accepted evidence.
- No publishing of accepted docs/skills/ADRs outside `.scryrs/`.
- No mutation of proposal inbox files to represent lifecycle state.

## Decisions

### Decision 1: Use one unified `ProposalReviewDecision` schema

Accepted and rejected outcomes share the same top-level contract rather than separate accepted/rejected document types. The contract uses a closed `ReviewOutcome` with `Accepted` and `Rejected` outcomes so both paths share common metadata and validation rules.

### Decision 2: Version the review decision contract independently

`crates/scryrs-types` will export `REVIEW_DECISION_SCHEMA_VERSION = "1.0.0"` independent of proposal, graph, route, hotspot, and trace schema constants. Review decisions are durable evidence artifacts with their own lifecycle and need separate versioning.

### Decision 3: Reuse existing proposal content and provenance types

Accepted review decisions reuse:

- `EvidenceLink` for `sourceEvidence`
- `ProposalTargetType` for `targetType`
- `ProposedContent` for `acceptedContent`
- `SemanticGraphGrouping` via `ProposedContent::SemanticGraphGrouping`

This keeps proposal and review artifacts on the same wire vocabulary and avoids a second provenance or content model.

### Decision 4: Common fields are mandatory for both outcomes

Every serialized `ProposalReviewDecision` includes:

- `schemaVersion`
- `proposalId`
- `reviewer`
- `decidedAt`
- `rationale`
- `sourceEvidence`
- `outcome`

Validation rejects wrong schema version and empty or missing required fields. `decidedAt` is specified as an RFC 3339 timestamp string.

### Decision 5: Accepted outcomes carry reviewed content; rejected outcomes do not

Accepted outcomes must include `targetType` plus `acceptedContent`. Rejected outcomes carry no accepted-content payload. Validation enforces the outcome-dependent invariants:

- accepted => non-empty `acceptedContent` with matching `targetType`
- rejected => no accepted-content fields
- `semantic_graph_grouping` accepted content must preserve explicit, non-empty `sourceNodeIds`

### Decision 6: Reviewed artifacts live outside the proposal inbox

Proposal inbox files remain at `.scryrs/proposals/{proposalId}.json` and stay review-only. Reviewed artifacts are separate files:

- `.scryrs/accepted/{proposalId}.json`
- `.scryrs/rejected/{proposalId}.json`

This preserves the proposal artifact as immutable review input while giving acceptance/rejection durable evidence paths.

### Decision 7: Documentation updates are limited to the boundary docs

The implementation updates `.devagent/docs/docs/proposals.md` and `.devagent/docs/docs/production-suite.md` to explain the three-zone layout: proposal inbox, accepted evidence, and rejected decisions. The docs must also restate that no graph, route, docs, or memory publishing occurs in this task.

### Decision 8: Downstream workflow and truth changes stay deferred

This change does not add review commands, graph ingestion, route changes, docs publishing, or memory mutation. Accepted evidence becomes a durable contract now; consuming it is explicitly later work.

## Conflict Resolution

- **Accepted/rejected directories vs `.scryrs/decisions/`**: refinement included a reviewer preference for a unified `.scryrs/decisions/` directory, but the architect and lead-dev decisions both chose separate `.scryrs/accepted/` and `.scryrs/rejected/` paths, and `production-suite.md` already names `.scryrs/accepted/` as the reviewed-evidence boundary. This spec adopts separate accepted/rejected directories.
- **Whether `acceptedContent` must equal the source proposal's `proposedContent`**: reviewer feedback proposed enforcing equality in this contract-only phase, but the accepted architect/lead-dev decisions and the task acceptance criteria require explicit accepted content, not a schema-level equality rule. This spec keeps `acceptedContent` explicit and validated for shape/provenance without introducing a workflow-level equality check that would require source-proposal comparison behavior outside this task.
- **Artifact identity strategy**: proposal-id-based filenames are adopted for reviewed artifacts because the accepted decisions explicitly target `.scryrs/accepted/{proposalId}.json` and `.scryrs/rejected/{proposalId}.json`. Re-review/history semantics are deferred.

## Risks

- **Single-decision filename convention**: proposal-id-based reviewed filenames imply one current decision artifact per proposal path. If later workflow needs multiple revisions, filename/version strategy will need a follow-up contract.
- **Shared content-type reuse**: new proposal target types will automatically become valid accepted-content shapes because the decision contract reuses `ProposedContent`. This is acceptable because accepted content is semantically the reviewed proposal content.
- **Future edited-acceptance policy**: if later review workflow allows reviewer-edited accepted content, downstream consumers must treat the reviewed payload as authoritative. This task only defines the contract, not the edit policy or CLI flow.

## Traceability

- Task: `5c682a97-5d98-49c9-a5f7-b93ec7b036f7`
- Dossier: `2026-06-28T09:20:49.872Z`
- Accepted decisions: `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, `1-swarm-reviewer-recommendation`
- Round evidence: `round:1:agent:swarm-architect`, `round:1:agent:swarm-lead-dev`, `round:1:agent:swarm-reviewer`
- Interpreted source boundaries: `openspec/specs/proposal-contract/spec.md`, `openspec/specs/proposal-generation/spec.md`, `openspec/specs/graph-contract/spec.md`, `.devagent/docs/docs/proposals.md`, `.devagent/docs/docs/production-suite.md`, `.devagent/docs/docs/roadmap.mdx`
