## Context

`scryrs graph` currently builds `.scryrs/graph.json` from required hotspot input plus optional docs structure. The repository already has a review loop that records accepted proposal decisions under `.scryrs/accepted/{proposalId}.json` using the `ProposalReviewDecision` contract, including exact `semantic_graph_grouping` payloads with `sourceNodeIds`, `targetGroupNodeId`, and `targetGroupLabel`.

The missing behavior is authoritative ingestion of that accepted evidence into graph build. Without it, reviewed semantic groupings remain durable review artifacts but never become explicit graph structure, so `scryrs route` cannot reflect them through the existing `contains`-edge grouping path.

## Goals / Non-Goals

### Goals

- Load accepted review-decision artifacts from `.scryrs/accepted/` during deterministic graph build.
- Project accepted `semantic_graph_grouping` decisions into explicit graph nodes and `contains` edges.
- Preserve deterministic ordering, fail-fast validation, and the `scryrs-graph` pure-container boundary.
- Keep route generation graph-driven with no proposal-specific route loader.
- Document the accepted-evidence flow and trust boundary.

### Non-Goals

- No inference from pending proposals, labels, stems, or model output.
- No direct consumption of `.scryrs/proposals/` or `.scryrs/rejected/` as graph truth.
- No mapping yet for accepted non-semantic proposal target types.
- No changes to `crates/scryrs-graph` build responsibilities.
- No proposal-review UI, publishing adapters, runtime retrieval, or route-explain work.

## Decisions

### Decision 1: Keep accepted-evidence ingestion inside `crates/scryrs-cli/src/graph.rs`

Add a dedicated accepted-evidence loading step after hotspot/docs graph assembly and before `KnowledgeGraph::to_document()`. This preserves the existing boundary that `crates/scryrs-graph` is a pure container and validation/materialization layer, not an input-loading pipeline.

### Decision 2: Treat `.scryrs/accepted/` as the only authoritative review input

Graph build reads `.scryrs/accepted/*.json` only. It does not inspect `.scryrs/proposals/` or `.scryrs/rejected/`. Accepted files are processed in sorted filename order to preserve byte-identical output for identical inputs.

### Decision 3: Support only `semantic_graph_grouping` projection in this task

Every accepted artifact is deserialized and validated as `ProposalReviewDecision`. Only accepted decisions whose `targetType` is `semantic_graph_grouping` create graph structure. Other accepted target types are schema-validated and then skipped with a stderr warning so future accepted-content mappings can coexist without breaking current builds.

### Decision 4: Build accepted group nodes from reviewed payload identity

For each accepted semantic grouping:

- node `id` = `targetGroupNodeId`
- node `label` = `targetGroupLabel`
- node `kind` = prefix before `:` in `targetGroupNodeId`

If `targetGroupNodeId` has no prefix delimiter, graph build fails with a descriptive error instead of guessing a kind.

### Decision 5: Split provenance between accepted node existence and grouping edges

The accepted group node carries `recorded_evidence` provenance that identifies the accepted decision artifact as the reason the node exists. Each `contains` edge carries the review decision's `sourceEvidence`, preserving the cited evidence that justified grouping the specific source nodes.

### Decision 6: Use deterministic `contains` edges and fail-fast graph validation

For each accepted `sourceNodeId`, graph build emits a `contains` edge with ID `{targetGroupNodeId}_contains_{sourceNodeId}`. Before materialization, graph build verifies every cited source node already exists after hotspot/docs construction. Missing source nodes fail the build with an error naming the accepted decision and missing node ID.

### Decision 7: Conflicting accepted group targets fail instead of merging

If more than one accepted semantic grouping targets the same `targetGroupNodeId`, graph build fails fast. This avoids inventing a merge policy for labels, evidence, or source-node sets that was not accepted during refinement.

### Decision 8: Route generation remains unchanged

`scryrs route` continues to consume `.scryrs/graph.json` only. Accepted groupings reach routes because they are materialized as normal graph `contains` edges, which the existing route generator already uses for grouping.

### Decision 9: Documentation updates explain the accepted-evidence trust boundary

Update `.devagent/docs/docs/graph.md`, `.devagent/docs/docs/proposals.md`, and `.devagent/docs/docs/route-manifests.md` to describe:

- accepted evidence as graph-build input
- pending and rejected directories as ignored for graph truth
- non-semantic accepted decisions as currently non-projecting
- route grouping as a downstream effect of graph `contains` edges rather than proposal-aware route code

## Conflict Resolution

- **Edge ID format**: adopted `{targetGroupNodeId}_contains_{sourceNodeId}` per accepted architect and reviewer guidance.
- **Node vs edge provenance**: group nodes use `recorded_evidence`; grouping edges use accepted decision `sourceEvidence`, matching the accepted architect and lead-dev recommendation.
- **Non-semantic accepted decisions**: skip with warning after validation rather than fail fatally, following the accepted architect and lead-dev recommendation.
- **Duplicate target-group policy**: fail fast on multiple accepted decisions for the same `targetGroupNodeId` rather than invent a merge policy. This resolves the reviewer’s open blocker in favor of the accepted architect/lead-dev direction.

## Risks

- Accepted groupings can fail graph build if their cited `sourceNodeIds` no longer exist after hotspot/docs assembly. The error message must identify the accepted decision and missing node IDs clearly.
- Non-semantic accepted decisions will be present in `.scryrs/accepted/` but intentionally have no graph effect yet. Warning output must make that boundary obvious.
- Determinism depends on explicit sorting of accepted filenames and preserving the existing graph ordering/materialization rules.

## Traceability

- Task: `70789d74-18a4-41b8-b7f4-35ca61a57489`
- Dossier: `2026-06-28T18:04:28.567Z`
- Accepted decisions: `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, `1-swarm-reviewer-recommendation`
- Round evidence: `round:1:agent:swarm-architect`, `round:1:agent:swarm-lead-dev`, `round:1:agent:swarm-reviewer`
- Source boundaries consulted in refinement: `openspec/specs/graph-build/spec.md`, `openspec/specs/graph-contract/spec.md`, `openspec/specs/proposal-contract/spec.md`, `openspec/specs/proposal-review-cli/spec.md`, `openspec/specs/route-manifest/spec.md`, `openspec/changes/task-5c682a97-5d98-49c9-a5f7-b93ec7b036f7/specs/proposal-review-contract/spec.md`, `.devagent/docs/docs/graph.md`, `.devagent/docs/docs/proposals.md`, `.devagent/docs/docs/route-manifests.md`
