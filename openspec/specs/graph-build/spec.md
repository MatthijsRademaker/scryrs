# graph-build Specification

## Purpose
TBD - created by archiving change task-03049c99-a529-446b-b253-2a4b7666d066. Update Purpose after archive.
## Requirements
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

### Requirement: Hotspot entries become graph nodes with provenance

The builder SHALL convert every entry in the loaded `.scryrs/hotspots.json` `entries` array into a `GraphNode`. Node identity SHALL use the format `{subjectKind}:{subject}`. Each node SHALL carry an `EvidenceLink` recording the hotspot provenance.

#### Scenario: All five subject kinds are included

- **GIVEN** hotspot entries exist for subject kinds `file`, `search`, `symbol`, `command`, and `document`
- **WHEN** the builder processes hotspot entries
- **THEN** a `GraphNode` is created for every entry
- **AND** no hotspot entry is silently dropped based on subject kind

#### Scenario: Node ID uses the subject-kind-prefixed scheme

- **GIVEN** a hotspot entry with `subjectKind: "file"` and `subject: "src/main.rs"`
- **WHEN** the builder creates a graph node
- **THEN** the node's `id` is `"file:src/main.rs"`

#### Scenario: Hotspot evidence row IDs become EvidenceLinks

- **GIVEN** a hotspot entry with `evidence.rowIds: [5, 12, 23]` and `subject: "src/auth.rs"`
- **WHEN** the builder creates a graph node
- **THEN** the node's `evidence_links` contains one entry with `sourceKind: "local_trace_row"`
- **AND** `subject` is `"src/auth.rs"`
- **AND** `rowIds` is `[5, 12, 23]` preserving the exact order

#### Scenario: Hotspot score is preserved as evidence link score

- **GIVEN** a hotspot entry with `score: 42`
- **WHEN** the builder creates the corresponding evidence link
- **THEN** the evidence link's `score` is `42`

### Requirement: Doc pages become graph nodes with structural edges

The builder SHALL scan `.devagent/docs/docs/` for `.md` and `.mdx` files and parse `_nav.json` for navigation hierarchy. Each discovered page SHALL become a `GraphNode` with kind `"doc_page"`. Nav hierarchy SHALL produce `contains` edges.

#### Scenario: Doc pages are discovered from the docs directory

- **GIVEN** the docs directory `.devagent/docs/docs/` contains `graph.md`, `hotspots.md`, and `architecture.mdx`
- **WHEN** the builder scans the docs directory
- **THEN** a `GraphNode` is created for each page
- **AND** each node has `kind: "doc_page"`

#### Scenario: Doc page node IDs derive from page slugs

- **GIVEN** a doc page at `.devagent/docs/docs/graph.md` with a nav link `"/graph"`
- **WHEN** the builder creates a graph node
- **THEN** the node's `id` is `"doc_page:graph"`

#### Scenario: Doc pages carry doc_reference evidence links

- **GIVEN** a doc page with slug `graph`
- **WHEN** the builder creates a graph node
- **THEN** the node's `evidence_links` contains one entry with `sourceKind: "doc_reference"`
- **AND** `docRef` is `"graph"`
- **AND** `rowIds` is empty

#### Scenario: Nav hierarchy produces contains edges

- **GIVEN** `_nav.json` defines a nav group with items linking to `"/graph"` and `"/hotspots"`
- **WHEN** the builder processes the nav hierarchy
- **THEN** `contains` edges are created from the nav group node to `doc_page:graph` and `doc_page:hotspots`
- **AND** a synthetic `docs_root` node exists as the root of the doc hierarchy
- **AND** `contains` edges connect `docs_root` to each top-level nav group

#### Scenario: No cross-domain edges in v1

- **GIVEN** the graph contains hotspot nodes and doc page nodes
- **WHEN** the builder produces edges
- **THEN** no edges connect hotspot nodes to doc page nodes
- **AND** edges are restricted to `contains` relationships derived from navigation hierarchy

### Requirement: Output is a valid KnowledgeGraphDocument

The builder SHALL assemble nodes and edges into a `KnowledgeGraph` container, validate structural integrity, and materialize a deterministic `KnowledgeGraphDocument` via `KnowledgeGraph::to_document()`. The document SHALL be written to stdout as a single-line JSON and to `.scryrs/graph.json`.

#### Scenario: Document passes structural validation

- **GIVEN** the builder has assembled nodes and edges into a `KnowledgeGraph`
- **WHEN** `to_document()` is called
- **THEN** structural validation succeeds (no dangling edge references)
- **AND** the returned `KnowledgeGraphDocument` contains `schemaVersion`, `metadata`, `nodes`, and `edges`

#### Scenario: Document is written to stdout and artifact file

- **GIVEN** a valid `KnowledgeGraphDocument` has been materialized
- **WHEN** the builder writes output
- **THEN** the document JSON is written to stdout as a single line
- **AND** the document JSON is written to `.scryrs/graph.json`

#### Scenario: Artifact write failure is fatal

- **GIVEN** `.scryrs/graph.json` cannot be written (e.g., permission denied)
- **WHEN** the builder attempts to write the artifact
- **THEN** the command exits with code 1
- **AND** an error message on stderr describes the write failure

### Requirement: Build output is deterministic

The builder SHALL produce byte-identical JSON output for identical inputs. It SHALL NOT introduce wall-clock timestamps, non-deterministic iteration order, or LLM inference.

#### Scenario: Repeated runs produce identical JSON

- **GIVEN** the same `.scryrs/hotspots.json` and docs structure
- **WHEN** `scryrs graph build <PATH>` is run twice
- **THEN** the stdout output is byte-identical across both runs
- **AND** `.scryrs/graph.json` is byte-identical across both runs

#### Scenario: Filesystem traversal is sorted

- **GIVEN** the docs directory contains multiple `.md` files
- **WHEN** the builder scans the directory
- **THEN** files are processed in sorted path order
- **AND** node creation order does not depend on filesystem iteration order

#### Scenario: No wall-clock timestamps in output

- **GIVEN** a valid graph document has been produced
- **WHEN** the output is inspected
- **THEN** no fields contain wall-clock timestamps
- **AND** the document's `metadata` does not include generation time fields

### Requirement: The graph crate remains a pure container

The `crates/scryrs-graph` crate SHALL NOT be modified to implement graph build, CLI commands, route-manifest generation, or input I/O. The graph build command lives entirely in `crates/scryrs-cli`.

#### Scenario: Graph build does not change scryrs-graph

- **GIVEN** the graph build implementation is complete
- **WHEN** `crates/scryrs-graph/src/lib.rs` is inspected
- **THEN** its public API remains unchanged (`KnowledgeGraph::new()`, `add_node()`, `add_edge()`, `validate()`, `to_document()`)
- **AND** no new modules, features, or public functions are added to `scryrs-graph`

### Requirement: CLI surface is discoverable

The `scryrs graph build` command SHALL appear in the human-readable `--help` output, the machine-readable `--help-json` output, and the README. It SHALL NOT be rejected as an unknown command.

#### Scenario: Graph appears in help output

- **GIVEN** the command is registered in the CLI dispatcher
- **WHEN** the user runs `scryrs --help`
- **THEN** the help output includes a `graph build <PATH>` entry

#### Scenario: Graph appears in help-json output

- **GIVEN** the command is registered in the CLI dispatcher
- **WHEN** the user runs `scryrs --help-json`
- **THEN** the JSON output includes a `graph` entry in the `commands` array with argument specifications and output contract

#### Scenario: Graph is accepted by dispatch

- **GIVEN** the command is registered in the CLI dispatcher
- **WHEN** the user runs `scryrs graph build /some/path`
- **THEN** the command is routed to the graph build handler
- **AND** it does not produce "unknown command" error

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

