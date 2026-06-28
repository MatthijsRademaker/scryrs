## MODIFIED Requirements

### Requirement: Grouping is derived only from explicit contains edges

Route entries SHALL carry an optional `grouping` field ONLY when the source graph node is the target of a `contains` edge from a parent group node. That includes parent group nodes materialized from accepted `semantic_graph_grouping` review decisions during graph build. `grouping` SHALL include `groupId` (the parent node's `id`) and `groupLabel` (the parent node's `label`). Route generation SHALL continue to consume `.scryrs/graph.json` only and SHALL NOT read `.scryrs/accepted/`, `.scryrs/rejected/`, or `.scryrs/proposals/` directly.

#### Scenario: Accepted semantic grouping appears through normal graph consumption

- **GIVEN** `.scryrs/graph.json` contains a node `domain_term:auth`
- **AND** `.scryrs/graph.json` contains a `contains` edge from `domain_term:auth` to `file:auth`
- **WHEN** `scryrs route <PATH>` runs
- **THEN** the route entry for `file:auth` includes `grouping.groupId = "domain_term:auth"`
- **AND** `grouping.groupLabel` equals the parent node label
- **AND** route generation does not inspect proposal or review-artifact directories
