## Context

scryrs already produces generated local artifacts under `.scryrs/` and already defines stable shared wire contracts for hotspots, graphs, and routes in `crates/scryrs-types`. It does not yet define an equivalent contract for reviewable knowledge proposals. The current `KnowledgeProposal` placeholder only carries `title` and `rationale`, which is insufficient for the task requirement that proposal artifacts expose evidence, rationale, target type, and proposed content in a stable structure.

The product docs are already clear about the intended direction: repeated context becomes proposals first, not silent documentation or graph mutations. The roadmap also places proposal work after hotspot, graph, and route foundations, and explicitly calls for proposal types plus inbox semantics rather than silent writes. This task is therefore foundation work for a review-first proposal system, not the proposal engine itself.

Constraints carried from refinement:

- No proposal generation command, review UI, or auto-apply behavior in this task.
- No direct mutation of published docs, ADRs, skills, playbooks, memory files, `graph.json`, or `routes.json`.
- Semantic grouping proposals must cite exact source graph node IDs and evidence before any higher-level grouping becomes recorded evidence.
- Existing CLI behavior stays unchanged; `propose` and `suggest-docs` remain unknown commands.

## Goals / Non-Goals

**Goals**

- Define a versioned `ProposalDocument` contract and matching OpenSpec capability.
- Make `evidence`, `rationale`, `targetType`, and `proposedContent` mandatory parts of every proposal artifact.
- Cover proposal targets for docs notes, ADRs, skills, debugging playbooks, memory patches, and semantic graph groupings.
- Define a deterministic `.scryrs/proposals/` inbox layout so reviewers can find and triage suggestions predictably.
- Reuse `EvidenceLink` so proposal provenance stays compatible with graph and route explanations.
- Require explicit source graph node IDs and evidence citations for semantic grouping proposals.
- Replace the placeholder `KnowledgeProposal` with executable Rust serde types in `scryrs-types`.

**Non-Goals**

- No `scryrs propose`, `scryrs suggest-docs`, or any other proposal-generation command.
- No accepted/rejected review decision schema, workflow state machine, inbox subdirectories, or reviewer-assignment mechanics.
- No dashboard review experience or publishing adapter behavior.
- No changes to hotspot scoring, trace ingest, graph build semantics, or route-manifest generation.
- No automatic reading of `.scryrs/proposals/` by graph build, route generation, or other truth-producing commands.

## Decisions

### Decision 1: The proposal contract is both specified and executable now

The change defines a new `proposal-contract` capability in OpenSpec and implements the same contract as Rust serde types in `crates/scryrs-types`. This follows the established repository pattern for graph and route contracts and avoids a documentation-only schema that downstream crates would have to reimplement.

### Decision 2: `ProposalDocument` is versioned independently

The proposal contract gets its own `PROPOSAL_SCHEMA_VERSION`, independent from the existing hotspot, graph, route, and trace schema constants. The top-level document shape includes `schemaVersion`, deterministic `id`, `targetType`, `title`, non-empty `rationale`, target-type-specific `proposedContent`, non-empty `evidence`, and `createdAt`.

### Decision 3: `targetType` is explicit and `proposedContent` is target-type-specific

The contract supports exactly six `targetType` values:

- `docs_note`
- `adr`
- `skill`
- `debugging_playbook`
- `memory_patch`
- `semantic_graph_grouping`

For `docs_note`, `adr`, `skill`, and `debugging_playbook`, `proposedContent` is non-empty markdown text. For `memory_patch`, `proposedContent` is a structured JSON object rather than overloaded prose. For `semantic_graph_grouping`, `proposedContent` is a structured object carrying at least `sourceNodeIds`, `targetGroupNodeId`, and `targetGroupLabel`.

### Decision 4: Proposal evidence reuses `EvidenceLink`

Proposal artifacts reuse the existing `EvidenceLink` vocabulary and source kinds already defined by the graph contract. That keeps provenance compatible with hotspot, graph, and route explanations instead of creating a second evidence dialect.

### Decision 5: Inbox layout is a flat `.scryrs/proposals/` directory with deterministic JSON filenames

Proposal artifacts live under `.scryrs/proposals/` as one JSON file per proposal. The filename stem is the proposal `id`, and the `id` is a deterministic SHA-256 content address derived from the `targetType` plus the canonical serialized `proposedContent`. This gives the task a concrete layout without requiring a generator or review database.

### Decision 6: Review decision mechanics are explicitly deferred

This task defines the proposal artifact and inbox only. It does not add status fields, accepted/rejected folders, reviewer metadata, or lifecycle transitions. Acceptance remains a separate future workflow, and no proposal becomes published documentation or recorded graph evidence merely by existing in the inbox.

### Decision 7: Semantic groupings remain proposals until explicit acceptance

A `semantic_graph_grouping` proposal must cite the exact low-level graph nodes it groups and must include evidence links justifying that grouping. The proposal contract does not itself change `graph.json` or `routes.json`; a separate explicit acceptance action is required before the grouping can become recorded evidence.

### Decision 8: Placeholder consumers migrate for compatibility only

Because `KnowledgeProposal` is currently consumed by `scryrs-adapter-markdown` and `scryrs-curator`, this change includes a migration note: replace the placeholder shared type with the new contract and update existing consumers only as needed to compile and test against it. Those updates must not grow into real proposal generation, CLI registration, or file emission behavior.

## Conflict Resolution

- **Proposal ID scheme**: refinement produced conflicting recommendations between opaque IDs and deterministic IDs. This spec chooses deterministic content-addressed SHA-256 proposal IDs and filenames because the architect decision and reviewer guidance both favored stable deduplication without introducing a generator dependency.
- **`proposedContent` shape**: refinement questioned whether all proposal content should be a flat string. This spec chooses target-type-specific structured content because `memory_patch` and `semantic_graph_grouping` need structured fields that markdown prose cannot represent reliably.
- **`KnowledgeProposal` migration**: refinement described both replacing the placeholder type and keeping current curator behavior placeholder-only. This spec resolves that tension by replacing the shared type while limiting curator and adapter changes to compatibility migration only, with no expansion into real generation behavior.

## Risks

- Replacing `KnowledgeProposal` can break existing placeholder consumers if migration is incomplete. Mitigation: include compatibility-only updates for `scryrs-adapter-markdown`, `scryrs-curator`, and their tests in the same task.
- Over-specifying review workflow mechanics would scope-creep into later curator tasks. Mitigation: keep status, acceptance records, and reviewer metadata explicitly out of scope.
- Semantic grouping proposals could be mistaken for graph truth if boundaries are vague. Mitigation: state clearly that `.scryrs/proposals/` is not consumed by graph build or route generation, and that grouping promotion requires explicit acceptance.

## Traceability

- Task source: `task:d5be595a-6dac-4f21-8ae9-3c90fac4803b`
- Dossier: `dossier:2026-06-27T18:33:23.760Z`
- Accepted decisions: `decision:1-swarm-architect-recommendation`, `decision:1-swarm-lead-dev-recommendation`, `decision:1-swarm-reviewer-recommendation`
- Round evidence: `round:1:agent:swarm-architect`, `round:1:agent:swarm-lead-dev`, `round:1:agent:swarm-reviewer`
- Supporting contracts and docs interpreted during refinement: `openspec/specs/graph-contract/spec.md`, `openspec/specs/route-manifest/spec.md`, `.devagent/docs/docs/vision.md`, `.devagent/docs/docs/roadmap.mdx`