## MODIFIED Requirements

### Requirement: The graph crate validates structure without implementing a build pipeline

`crates/scryrs-graph` SHALL provide a `KnowledgeGraph` container that stores nodes and directed edges under the shared contract, validates structural references, preserves evidence links, and materializes a deterministic `KnowledgeGraphDocument`. This foundation SHALL NOT implement graph build, CLI graph commands, route-manifest generation, docs crawling, adapter integration, server endpoints, dashboard work, or runtime retrieval behavior.

#### Scenario: Dangling edges are rejected

- **GIVEN** a graph edge whose `sourceNodeId` or `targetNodeId` does not match any node in the graph
- **WHEN** the graph is validated or materialized
- **THEN** the graph is rejected as invalid
- **AND** the invalid edge is not silently preserved as successful output

#### Scenario: Graph crate remains build-pipeline-free; build command is a separate CLI consumer

- **WHEN** the graph build capability is introduced
- **THEN** the `crates/scryrs-graph` crate does not implement graph build, CLI commands, route-manifest generation, docs crawling, or input I/O
- **AND** the CLI command `scryrs graph build` lives in `crates/scryrs-cli` as a separate consumer that assembles and materializes graph documents through `KnowledgeGraph::add_node()`, `KnowledgeGraph::add_edge()`, and `KnowledgeGraph::to_document()`
- **AND** the graph crate's public API remains a pure container/contract interface