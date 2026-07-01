## MODIFIED Requirements

### Requirement: Route entries carry structured identity, target, and evidence

Each `RouteEntry` SHALL include `id` (the graph node ID), `subjectKind` (the node kind), `subject` (raw subject value), `label` (human-readable label), `target` (the stable source graph node ID preserved for identity and matching), `kind` (node kind repeated), and `evidenceLinks` (provenance backlinks). Optional fields SHALL include `loadTarget`, `grouping`, and `metadata`. When present, `loadTarget` SHALL be a structured object with `kind` and optional `reference`. `file` routes SHALL use a repository-relative file `reference`; `doc_page` routes SHALL use a canonical docs `reference` in the form `project-docs/<slug>`; non-loadable kinds SHALL use `kind = "non_loadable"` with no `reference`.

#### Scenario: File route keeps node identity and exposes a loadable file reference

- **GIVEN** a graph node with `id = "file:src/main.rs"`, `kind = "file"`, `label = "src/main.rs"`, and an `EvidenceLink` with `sourceKind = "local_trace_row"`
- **WHEN** the route generator emits a route entry for this node
- **THEN** the entry's `id` is `"file:src/main.rs"`
- **AND** `subjectKind` is `"file"`
- **AND** `subject` is `"src/main.rs"`
- **AND** `label` is `"src/main.rs"`
- **AND** `target` is `"file:src/main.rs"`
- **AND** `loadTarget.kind` is `"file"`
- **AND** `loadTarget.reference` is `"src/main.rs"`
- **AND** `evidenceLinks` contains the same evidence link with `sourceKind = "local_trace_row"`

#### Scenario: Docs routes normalize to a canonical docs reference

- **GIVEN** a `doc_page` route whose usable docs reference is either `"graph"` or `"project-docs/graph"`
- **WHEN** the route generator emits a route entry
- **THEN** `loadTarget.kind` is `"doc_page"`
- **AND** `loadTarget.reference` is exactly `"project-docs/graph"`

#### Scenario: Route entry for doc pages preserves doc_reference evidence

- **GIVEN** a graph node with `id = "doc_page:graph"`, `kind = "doc_page"`, and an `EvidenceLink` with `sourceKind = "doc_reference"` and `docRef = "graph"`
- **WHEN** the route generator emits a route entry
- **THEN** the entry's `evidenceLinks` contains one link with `sourceKind = "doc_reference"`
- **AND** that link's `docRef` is `"graph"`
- **AND** `loadTarget.reference` is `"project-docs/graph"`

#### Scenario: Non-loadable subject kinds remain explicit

- **GIVEN** a graph node with `id = "search:auth"`, `kind = "search"`, and label `"auth"`
- **WHEN** the route generator emits a route entry
- **THEN** `target` remains `"search:auth"`
- **AND** `loadTarget.kind` is `"non_loadable"`
- **AND** `loadTarget` has no `reference`

#### Scenario: Malformed file subject fails loudly

- **GIVEN** a graph node whose `file:` subject is empty, absolute, or parent-traversing
- **WHEN** `scryrs route <PATH>` runs
- **THEN** the command exits with code 2
- **AND** stderr explains that file routes must resolve to a non-empty repository-relative path without parent traversal

#### Scenario: Doc page without a usable docs reference fails loudly

- **GIVEN** a `doc_page` graph node that cannot produce a usable `DocReference`-derived docs reference
- **WHEN** `scryrs route <PATH>` runs
- **THEN** the command exits with code 2
- **AND** stderr explains that doc_page routes must provide a canonical docs reference
