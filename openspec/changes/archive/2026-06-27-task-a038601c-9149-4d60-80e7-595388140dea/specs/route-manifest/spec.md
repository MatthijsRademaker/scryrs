# route-manifest Specification

## Purpose

Define the route manifest wire contract and generator behavior. Route manifests consume the knowledge graph artifact at `.scryrs/graph.json` and emit a stable, evidence-backed JSON artifact at `.scryrs/routes.json`. Every route entry preserves `(subjectKind, subject)` identity boundaries by default; higher-level grouping is only derived from explicit `contains` edges in the graph. The generator must not perform LLM inference, semantic clustering, or cross-domain grouping that lacks explicit graph evidence.

## ADDED Requirements

### Requirement: Route manifest documents are versioned and self-describing

The system SHALL define a `RouteManifestDocument` wire contract for route consumers. Each serialized route manifest SHALL include `schemaVersion`, `metadata`, and `routes`. `schemaVersion` SHALL equal `ROUTE_SCHEMA_VERSION` and SHALL be versioned independently from `SCHEMA_VERSION`, `HOTSPOT_SCHEMA_VERSION`, `LIVE_HOTSPOT_SCHEMA_VERSION`, and `GRAPH_SCHEMA_VERSION`.

#### Scenario: Serialized manifest carries the required top-level fields

- **GIVEN** a route manifest document produced under this contract
- **WHEN** a consumer inspects the serialized JSON
- **THEN** it contains `schemaVersion` equal to `"1.0.0"`
- **AND** it contains `metadata`
- **AND** it contains `routes` as a JSON array

#### Scenario: Route schema version is independent from graph and hotspot versions

- **GIVEN** the workspace already defines `GRAPH_SCHEMA_VERSION`, `HOTSPOT_SCHEMA_VERSION`, and `SCHEMA_VERSION`
- **WHEN** the route manifest contract is versioned
- **THEN** it defines a separate `ROUTE_SCHEMA_VERSION`
- **AND** every serialized manifest uses `ROUTE_SCHEMA_VERSION` for `schemaVersion`

### Requirement: Route entries carry structured identity, target, and evidence

Each `RouteEntry` SHALL include `id` (the graph node ID), `subjectKind` (the node kind), `subject` (raw subject value), `label` (human-readable label), `target` (normalized load target), `kind` (node kind repeated), and `evidenceLinks` (provenance backlinks). Optional fields SHALL include `grouping` (when explicit `contains` edge from a parent group node exists) and `metadata`.

#### Scenario: Route entry fields match the source graph node

- **GIVEN** a graph node with `id = "file:src/main.rs"`, `kind = "file"`, `label = "src/main.rs"`, and an `EvidenceLink` with `sourceKind = "local_trace_row"`
- **WHEN** the route generator emits a route entry for this node
- **THEN** the entry's `id` is `"file:src/main.rs"`
- **AND** `subjectKind` is `"file"`
- **AND** `subject` is `"src/main.rs"`
- **AND** `label` is `"src/main.rs"`
- **AND** `target` is `"file:src/main.rs"`
- **AND** `evidenceLinks` contains the same evidence link with `sourceKind = "local_trace_row"`

#### Scenario: Route entry for doc pages carries doc_reference evidence

- **GIVEN** a graph node with `id = "doc_page:graph"`, `kind = "doc_page"`, and an `EvidenceLink` with `sourceKind = "doc_reference"` and `docRef = "graph"`
- **WHEN** the route generator emits a route entry for this node
- **THEN** the entry's `evidenceLinks` contains one link with `sourceKind = "doc_reference"`
- **AND** that link's `docRef` is `"graph"`

### Requirement: Route generation preserves subject identity boundaries

Each graph node SHALL produce exactly one `RouteEntry`. Distinct `(subjectKind, subject)` identities SHALL NOT be collapsed unless explicit graph edges justify higher-level grouping. In particular, nodes `file:auth`, `search:auth`, and `symbol:auth` with no linking domain node or edges SHALL remain three distinct route entries.

#### Scenario: Hotspot subjects with shared text remain distinct

- **GIVEN** the graph contains nodes `file:auth`, `search:auth`, and `symbol:auth` with no edges connecting them to any common parent node
- **WHEN** the route generator emits the manifest
- **THEN** the `routes` array contains three distinct entries with `id` values `"file:auth"`, `"search:auth"`, and `"symbol:auth"`
- **AND** no entry is silently omitted
- **AND** no inferred grouping merges them into a single entry

#### Scenario: Every graph node produces exactly one route entry

- **GIVEN** a `KnowledgeGraphDocument` with N graph nodes
- **WHEN** the route generator runs
- **THEN** the emitted `routes` array contains exactly N entries
- **AND** each entry's `id` matches a distinct graph node `id`

### Requirement: Grouping is derived only from explicit contains edges

Route entries SHALL carry an optional `grouping` field ONLY when the source graph node is the target of a `contains` edge from a parent group node. `grouping` SHALL include `groupId` (the parent node's `id`) and `groupLabel` (the parent node's `label`). No other relationship kind or inferred grouping SHALL be applied.

#### Scenario: Doc page with parent group carries grouping

- **GIVEN** the graph contains a node `doc_page:graph` and a `contains` edge from node `technical` (label `"Technical"`) to `doc_page:graph`
- **WHEN** the route generator emits the entry for `doc_page:graph`
- **THEN** the entry includes `grouping` with `groupId = "technical"` and `groupLabel = "Technical"`

#### Scenario: Doc page without parent group has no grouping

- **GIVEN** the graph contains a node `doc_page:orphan` with no incoming `contains` edges
- **WHEN** the route generator emits the entry for `doc_page:orphan`
- **THEN** the entry does NOT include a `grouping` field

#### Scenario: Hotspot nodes remain ungrouped in v1

- **GIVEN** the graph contains a hotspot node `file:src/main.rs`
- **AND** v1 graph build does not emit cross-domain edges between hotspot and doc nodes
- **WHEN** the route generator emits the entry for `file:src/main.rs`
- **THEN** the entry does NOT include a `grouping` field

### Requirement: Route manifest output is deterministic

The generator SHALL produce byte-identical JSON output for identical `.scryrs/graph.json` input. The `routes` array SHALL sort by `id` ascending. Evidence links within each entry SHALL sort by `(sourceKind, subject, docRef, description, rowIds, score)` ascending. The output SHALL NOT include wall-clock timestamps, random identifiers, or non-deterministic iteration.

#### Scenario: Repeated runs produce identical JSON

- **GIVEN** the same `.scryrs/graph.json` artifact
- **WHEN** `scryrs route <PATH>` is run twice
- **THEN** the stdout output is byte-identical across both runs
- **AND** `.scryrs/routes.json` is byte-identical across both runs

#### Scenario: Routes are sorted by id ascending

- **GIVEN** graph nodes with IDs `"file:zzz.rs"`, `"file:aaa.rs"`, `"search:routing"`
- **WHEN** the route generator sorts routes
- **THEN** the routes array order is `"file:aaa.rs"`, `"file:zzz.rs"`, `"search:routing"` (lexicographic by node ID)

#### Scenario: No wall-clock timestamps in output

- **GIVEN** a valid route manifest has been produced
- **WHEN** the output is inspected
- **THEN** no fields contain wall-clock timestamps
- **AND** the document's `metadata` does not include generation time fields

### Requirement: The CLI command accepts a required PATH argument

The `scryrs route <PATH>` command SHALL accept exactly one required positional argument `PATH`. It SHALL resolve `PATH` to an absolute repository root, load `.scryrs/graph.json` under that root, and fail with exit code 2 and a contract three-line error on stderr when the artifact is missing or malformed.

#### Scenario: Missing graph artifact fails with exit code 2

- **GIVEN** a repository root where `.scryrs/graph.json` does not exist
- **WHEN** the user runs `scryrs route <PATH>`
- **THEN** the command exits with code 2
- **AND** an explicit error message on stderr states the graph artifact is missing
- **AND** the error follows the three-line contract format

#### Scenario: Malformed graph artifact fails with exit code 2

- **GIVEN** a repository root where `.scryrs/graph.json` exists but contains invalid JSON
- **WHEN** the user runs `scryrs route <PATH>`
- **THEN** the command exits with code 2
- **AND** an explicit error message on stderr describes the parse failure

#### Scenario: Incompatible graph schema version fails with exit code 2

- **GIVEN** `.scryrs/graph.json` contains `schemaVersion` not equal to `GRAPH_SCHEMA_VERSION`
- **WHEN** the user runs `scryrs route <PATH>`
- **THEN** the command exits with code 2
- **AND** an error message on stderr states the schema version mismatch

### Requirement: Route manifest is discoverable in the CLI

The `scryrs route <PATH>` command SHALL appear in the human-readable `--help` output, the machine-readable `--help-json` output, and the README. It SHALL NOT be rejected as an unknown command.

#### Scenario: Route appears in help output

- **GIVEN** the command is registered in the CLI dispatcher
- **WHEN** the user runs `scryrs --help`
- **THEN** the help output includes a `scryrs route <PATH>` entry

#### Scenario: Route appears in help-json output

- **GIVEN** the command is registered in the CLI dispatcher
- **WHEN** the user runs `scryrs --help-json`
- **THEN** the JSON output includes a `route` entry in the `commands` array with argument specifications and output contract

#### Scenario: Route is accepted by dispatch

- **GIVEN** the command is registered in the CLI dispatcher
- **WHEN** the user runs `scryrs route /some/path`
- **THEN** the command is routed to the route manifest generator
- **AND** it does not produce an "unknown command" error

### Requirement: The graph crate remains unchanged

The `crates/scryrs-graph` crate SHALL NOT be modified to implement route manifest generation. The route generator lives entirely in `crates/scryrs-cli` and consumes the graph artifact as JSON input.

#### Scenario: Route generation does not change scryrs-graph

- **GIVEN** the route manifest implementation is complete
- **WHEN** `crates/scryrs-graph/src/lib.rs` is inspected
- **THEN** its public API remains unchanged
- **AND** no new modules, features, or public functions related to route generation are added
