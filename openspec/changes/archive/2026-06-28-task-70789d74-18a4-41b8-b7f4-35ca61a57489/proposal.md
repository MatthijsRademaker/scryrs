## Why

Graph build currently stops at hotspots and optional docs, even though reviewed proposal outcomes under `.scryrs/accepted/` already exist as durable evidence. That leaves accepted semantic groupings unable to affect `.scryrs/graph.json` or downstream route manifests unless a later consumer re-infers them.

This change closes that loop by making accepted review decisions authoritative graph input while preserving the existing trust boundary: pending proposal inbox files and rejected review decisions remain non-authoritative and must not mutate graph truth.

## What Changes

- Extend graph build in `crates/scryrs-cli/src/graph.rs` to load `.scryrs/accepted/*.json` after hotspot/docs graph assembly and before graph materialization.
- Validate every accepted artifact as `ProposalReviewDecision`, process files in deterministic filename order, and project only accepted `semantic_graph_grouping` decisions into explicit graph structure.
- Materialize each accepted grouping as:
  - a group node with `id = targetGroupNodeId`, `label = targetGroupLabel`, and `kind` derived from the node-id prefix before `:`
  - `contains` edges from the group node to each accepted `sourceNodeId`
  - node provenance using `recorded_evidence` pointing at the accepted decision, and edge provenance copied from the decision `sourceEvidence`
- Use deterministic edge IDs of the form `{targetGroupNodeId}_contains_{sourceNodeId}`.
- Fail fast on malformed accepted artifacts, missing source nodes, missing target-group kind prefix, or conflicting accepted decisions targeting the same `targetGroupNodeId`.
- Skip non-`semantic_graph_grouping` accepted decisions with a warning so later accepted target types can coexist without breaking graph build.
- Leave `crates/scryrs-graph` and `crates/scryrs-cli/src/route.rs` unchanged in behavior: route manifests pick up accepted grouping through normal `contains`-edge processing.
- Update graph/proposals/route-manifest documentation to explain the accepted-evidence flow and that pending/rejected directories are ignored by graph build.

## Impact

- Accepted semantic grouping decisions become explicit, deterministic graph evidence and therefore affect route manifests through the existing graph-to-route pipeline.
- Pending `.scryrs/proposals/` files and rejected `.scryrs/rejected/` files remain inert for graph generation.
- Implementation stays localized to graph-build CLI code, tests, and docs; no new proposal-specific route logic and no graph-crate pipeline changes are introduced.
- Verification must cover accepted-grouping graph output, deterministic ordering, fail-fast validation, ignored pending/rejected inputs, and end-to-end route grouping from accepted graph edges.
