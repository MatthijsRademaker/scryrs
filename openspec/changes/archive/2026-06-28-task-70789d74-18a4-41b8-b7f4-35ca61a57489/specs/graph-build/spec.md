## MODIFIED Requirements

### Requirement: Graph build consumes hotspot and docs inputs

The `scryrs graph build <PATH>` command SHALL consume the required hotspot artifact at `.scryrs/hotspots.json` under the resolved repository root, the optional docs structure under `.devagent/docs/docs/` (including `_nav.json`), and the optional accepted-evidence directory at `.scryrs/accepted/`. Accepted evidence files SHALL be processed in sorted filename order. Graph build SHALL NOT read `.scryrs/proposals/` or `.scryrs/rejected/`.

#### Scenario: Accepted evidence input is optional

- **GIVEN** a repository root with a valid `.scryrs/hotspots.json`
- **AND** no `.scryrs/accepted/` directory exists
- **WHEN** the user runs `scryrs graph build <PATH>`
- **THEN** the command exits with code 0
- **AND** the emitted graph is built from hotspot and docs inputs only

#### Scenario: Accepted evidence files are processed deterministically

- **GIVEN** `.scryrs/accepted/` contains multiple valid accepted review-decision artifacts
- **WHEN** graph build runs
- **THEN** those files are processed in sorted filename order
- **AND** repeated runs with identical hotspot, docs, and accepted inputs produce byte-identical stdout and `.scryrs/graph.json`

#### Scenario: Pending proposals and rejected decisions are ignored

- **GIVEN** `.scryrs/proposals/` contains pending proposal artifacts
- **AND** `.scryrs/rejected/` contains rejected review-decision artifacts
- **WHEN** graph build runs
- **THEN** graph build does not read either directory
- **AND** `.scryrs/graph.json` is unchanged relative to the same repository without those pending or rejected files

#### Scenario: Malformed accepted evidence fails graph build loudly

- **GIVEN** `.scryrs/accepted/{id}.json` exists but contains malformed JSON, an incompatible schema version, or content that fails `ProposalReviewDecision::validate()`
- **WHEN** graph build runs
- **THEN** the command exits non-zero
- **AND** stderr identifies the accepted artifact as invalid
- **AND** `.scryrs/graph.json` is not partially updated

## ADDED Requirements

### Requirement: Accepted semantic grouping decisions become explicit graph structure

Graph build SHALL deserialize accepted review-decision artifacts as `ProposalReviewDecision` and project only accepted decisions with `targetType = "semantic_graph_grouping"` into graph structure. For each projected decision, graph build SHALL create a group node with `id = targetGroupNodeId`, `label = targetGroupLabel`, and `kind` derived from the prefix of `targetGroupNodeId` before `:`. It SHALL create a `contains` edge from that group node to each accepted `sourceNodeId`. Group nodes SHALL carry `recorded_evidence` provenance referencing the accepted decision. Grouping edges SHALL carry the accepted decision `sourceEvidence`. Edge IDs SHALL use the format `{targetGroupNodeId}_contains_{sourceNodeId}`.

#### Scenario: Accepted semantic grouping creates a group node and contains edges

- **GIVEN** `.scryrs/accepted/{id}.json` contains a valid accepted `semantic_graph_grouping` decision with `targetGroupNodeId = "domain_term:auth"`, `targetGroupLabel = "Auth"`, and `sourceNodeIds = ["file:auth", "search:auth"]`
- **AND** hotspot/docs construction already produced nodes `file:auth` and `search:auth`
- **WHEN** graph build runs
- **THEN** `.scryrs/graph.json` contains a node with `id = "domain_term:auth"`, `label = "Auth"`, and `kind = "domain_term"`
- **AND** that node includes `recorded_evidence` provenance naming the accepted decision artifact
- **AND** `.scryrs/graph.json` contains edges `domain_term:auth_contains_file:auth` and `domain_term:auth_contains_search:auth`
- **AND** each edge uses relationship `contains`
- **AND** each edge carries the decision `sourceEvidence`

#### Scenario: Non-semantic accepted decisions are skipped with a warning

- **GIVEN** `.scryrs/accepted/{id}.json` contains a valid accepted review decision whose `targetType` is not `semantic_graph_grouping`
- **WHEN** graph build runs
- **THEN** graph build emits a warning to stderr naming the skipped target type
- **AND** no nodes or edges are added for that decision

#### Scenario: Missing accepted source nodes fail graph build

- **GIVEN** an accepted `semantic_graph_grouping` decision cites `sourceNodeIds` containing `file:missing.rs`
- **AND** hotspot/docs construction did not produce a node with ID `file:missing.rs`
- **WHEN** graph build runs
- **THEN** the command exits non-zero
- **AND** stderr identifies the accepted decision and missing source node ID
- **AND** graph output is not written partially

#### Scenario: Target group node IDs without a kind prefix fail graph build

- **GIVEN** an accepted `semantic_graph_grouping` decision uses `targetGroupNodeId = "auth"`
- **WHEN** graph build runs
- **THEN** the command exits non-zero
- **AND** stderr reports that graph build cannot derive a node kind from that accepted target group node ID

#### Scenario: Conflicting accepted group targets fail graph build

- **GIVEN** two accepted `semantic_graph_grouping` decisions target the same `targetGroupNodeId`
- **WHEN** graph build runs
- **THEN** the command exits non-zero
- **AND** stderr reports a conflicting accepted grouping for that target group node ID
