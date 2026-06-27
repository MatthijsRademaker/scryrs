## ADDED Requirements

### Requirement: Graph build consumes hotspot and docs inputs

The `scryrs graph build <PATH>` command SHALL consume two local inputs: the hotspot artifact at `.scryrs/hotspots.json` under the resolved repository root, and the docs structure under `.devagent/docs/docs/` (including `_nav.json`).

#### Scenario: Hotspot input is required

- **GIVEN** a repository root where `.scryrs/hotspots.json` does not exist
- **WHEN** the user runs `scryrs graph build <PATH>`
- **THEN** the command exits with code 2
- **AND** an explicit error message on stderr states the hotspot artifact is missing

#### Scenario: Malformed hotspot input fails

- **GIVEN** a repository root where `.scryrs/hotspots.json` exists but contains invalid JSON
- **WHEN** the user runs `scryrs graph build <PATH>`
- **THEN** the command exits with code 2
- **AND** an explicit error message on stderr describes the parse failure

#### Scenario: Docs input is optional

- **GIVEN** a repository root with a valid `.scryrs/hotspots.json` but no `.devagent/docs/docs/` directory
- **WHEN** the user runs `scryrs graph build <PATH>`
- **THEN** the command exits with code 0
- **AND** a warning is printed to stderr indicating the docs layer is empty
- **AND** the emitted graph contains hotspot-derived nodes but no doc-page nodes

#### Scenario: Missing PATH argument fails

- **GIVEN** the `scryrs graph build` command is invoked without a PATH argument
- **WHEN** the command dispatcher processes the invocation
- **THEN** the command exits with code 2
- **AND** an error message on stderr follows the contract three-line format naming the command, usage, and `scryrs --help` escalation

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