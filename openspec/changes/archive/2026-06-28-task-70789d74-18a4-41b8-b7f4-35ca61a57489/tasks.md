## Implementation

- [x] Add accepted-evidence loading in `crates/scryrs-cli/src/graph.rs` after hotspot/docs assembly and before graph materialization.
- [x] Read `.scryrs/accepted/*.json` in sorted filename order, deserialize each file as `ProposalReviewDecision`, and validate every artifact before projection.
- [x] Project only accepted `semantic_graph_grouping` decisions into graph nodes and `contains` edges with:
  - deterministic edge IDs `{targetGroupNodeId}_contains_{sourceNodeId}`
  - node kind derived from `targetGroupNodeId` prefix before `:`
  - `recorded_evidence` provenance on accepted group nodes
  - review `sourceEvidence` provenance on grouping edges
- [x] Skip accepted non-semantic target types with a warning and no graph mutations.
- [x] Fail graph build loudly for malformed accepted artifacts, missing source nodes, invalid target-group IDs without a kind prefix, or conflicting accepted decisions sharing a `targetGroupNodeId`.
- [x] Leave `crates/scryrs-graph` and `crates/scryrs-cli/src/route.rs` free of proposal-specific ingestion logic.

## Verification

- [x] Add graph-build tests covering accepted semantic grouping node/edge creation and provenance.
- [x] Add tests proving pending `.scryrs/proposals/` files and rejected `.scryrs/rejected/` decisions do not affect `.scryrs/graph.json`.
- [x] Add tests for malformed accepted artifacts, skipped non-semantic accepted decisions, missing source nodes, duplicate target-group conflicts, and deterministic filename ordering.
- [x] Add or extend route-generation tests showing accepted grouping reaches route manifests through existing `contains`-edge grouping behavior with no direct accepted-directory reads.

## Documentation

- [x] Update `.devagent/docs/docs/graph.md` to document accepted-evidence ingestion, provenance, determinism, and the ignored pending/rejected directories.
- [x] Update `.devagent/docs/docs/proposals.md` to explain that proposal inbox files remain non-authoritative while accepted review decisions can become graph input.
- [x] Update `.devagent/docs/docs/route-manifests.md` to explain that accepted semantic grouping affects route grouping only through normal graph `contains` edges.
