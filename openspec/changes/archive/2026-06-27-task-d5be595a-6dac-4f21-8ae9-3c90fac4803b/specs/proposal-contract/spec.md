## ADDED Requirements

### Requirement: Proposal documents are versioned, reviewable inbox artifacts

The system SHALL define a versioned `ProposalDocument` contract for reviewable knowledge suggestions. Each serialized proposal document SHALL include `schemaVersion`, `id`, `targetType`, `title`, `rationale`, `proposedContent`, `evidence`, and `createdAt`. `schemaVersion` SHALL equal `PROPOSAL_SCHEMA_VERSION` and SHALL be versioned independently from `SCHEMA_VERSION`, `HOTSPOT_SCHEMA_VERSION`, `GRAPH_SCHEMA_VERSION`, and `ROUTE_SCHEMA_VERSION`.

#### Scenario: Serialized proposal document carries required top-level fields

- **GIVEN** a proposal artifact produced under this contract
- **WHEN** a reviewer inspects the serialized JSON
- **THEN** it contains `schemaVersion`
- **AND** it contains `id`
- **AND** it contains `targetType`
- **AND** it contains `title`
- **AND** it contains `rationale`
- **AND** it contains `proposedContent`
- **AND** it contains `evidence` as a JSON array
- **AND** it contains `createdAt`

#### Scenario: Proposal schema version is independent from other artifact contracts

- **GIVEN** the workspace already defines `SCHEMA_VERSION`, `HOTSPOT_SCHEMA_VERSION`, `GRAPH_SCHEMA_VERSION`, and `ROUTE_SCHEMA_VERSION`
- **WHEN** the proposal contract is versioned
- **THEN** it defines a separate `PROPOSAL_SCHEMA_VERSION`
- **AND** every serialized proposal document uses `PROPOSAL_SCHEMA_VERSION` for `schemaVersion`

### Requirement: Proposal documents support the defined target types and content shapes

The contract SHALL support exactly six `targetType` values serialized as snake_case strings: `docs_note`, `adr`, `skill`, `debugging_playbook`, `memory_patch`, and `semantic_graph_grouping`. `proposedContent` SHALL be target-type-specific content: non-empty markdown for `docs_note`, `adr`, `skill`, and `debugging_playbook`; a structured JSON object for `memory_patch`; and a structured grouping object for `semantic_graph_grouping`.

#### Scenario: Markdown-backed proposal targets carry non-empty markdown content

- **GIVEN** a proposal with `targetType = "docs_note"`, `"adr"`, `"skill"`, or `"debugging_playbook"`
- **WHEN** the proposal is serialized
- **THEN** `proposedContent` contains non-empty markdown text for that target

#### Scenario: Memory patch proposals carry structured patch content

- **GIVEN** a proposal with `targetType = "memory_patch"`
- **WHEN** the proposal is serialized
- **THEN** `proposedContent` is a structured JSON object rather than a flat markdown string

#### Scenario: Semantic graph grouping proposals carry structured grouping content

- **GIVEN** a proposal with `targetType = "semantic_graph_grouping"`
- **WHEN** the proposal is serialized
- **THEN** `proposedContent` includes `sourceNodeIds`
- **AND** `proposedContent` includes `targetGroupNodeId`
- **AND** `proposedContent` includes `targetGroupLabel`

### Requirement: Rationale and evidence are mandatory for every proposal

Every proposal document SHALL require non-empty `rationale`, non-empty `proposedContent`, and non-empty `evidence`. Proposal evidence SHALL reuse the existing `EvidenceLink` contract and SHALL NOT define a second provenance vocabulary.

#### Scenario: Proposal with empty rationale is invalid

- **GIVEN** a serialized proposal document whose `rationale` is empty
- **WHEN** the document is validated against the proposal contract
- **THEN** validation fails
- **AND** the proposal is rejected as invalid

#### Scenario: Proposal with empty evidence is invalid

- **GIVEN** a serialized proposal document whose `evidence` array is empty
- **WHEN** the document is validated against the proposal contract
- **THEN** validation fails
- **AND** the proposal is rejected as invalid

#### Scenario: Proposal evidence uses the graph evidence vocabulary

- **GIVEN** a valid proposal document
- **WHEN** a consumer inspects the `evidence` field
- **THEN** each evidence entry conforms to `EvidenceLink`
- **AND** the proposal uses existing evidence source kinds such as `hotspot_subject`, `local_trace_row`, `server_trace_row`, `doc_reference`, or `recorded_evidence`

### Requirement: Semantic graph grouping proposals cite explicit source nodes and remain pending until acceptance

A `semantic_graph_grouping` proposal SHALL require a non-empty `sourceNodeIds` list containing exact source graph node IDs plus at least one evidence citation justifying the grouping. The proposal contract SHALL treat the grouping as review-only; explicit acceptance is required before the grouping can become recorded graph evidence.

#### Scenario: Semantic grouping proposal without source nodes is invalid

- **GIVEN** a proposal with `targetType = "semantic_graph_grouping"`
- **AND** its `proposedContent.sourceNodeIds` field is empty
- **WHEN** the document is validated against the proposal contract
- **THEN** validation fails
- **AND** the proposal is rejected as invalid

#### Scenario: Semantic grouping proposal cites exact hotspot-backed node identities

- **GIVEN** source graph nodes `file:auth`, `search:auth`, and `symbol:auth`
- **WHEN** a proposal suggests a higher-level grouping such as `domain_term:auth`
- **THEN** the proposal cites those exact source node IDs in `sourceNodeIds`
- **AND** the proposal includes evidence citations justifying the grouping
- **AND** the grouping does not become recorded graph evidence merely by being proposed

### Requirement: Proposal inbox layout is deterministic and non-authoritative

Proposal artifacts SHALL be stored as individual JSON files under `.scryrs/proposals/`. Each filename stem SHALL equal the proposal document `id`, and each `id` SHALL be a deterministic SHA-256 content address derived from the proposal `targetType` plus the canonical serialized `proposedContent`. Proposal inbox files are review artifacts only and SHALL NOT directly mutate published docs, ADRs, skills, playbooks, memory truth, `.scryrs/graph.json`, or `.scryrs/routes.json`.

#### Scenario: Equivalent proposal content yields the same inbox identity

- **GIVEN** two proposal documents with the same `targetType` and the same canonical `proposedContent`
- **WHEN** their proposal IDs are computed
- **THEN** the resulting `id` values are identical
- **AND** the resulting inbox filename stems are identical

#### Scenario: Proposal artifacts do not change source-of-truth outputs

- **GIVEN** a proposal JSON file exists under `.scryrs/proposals/`
- **WHEN** a reviewer or tool inspects the repository's published docs, memory truth, `.scryrs/graph.json`, or `.scryrs/routes.json`
- **THEN** none of those source-of-truth artifacts have been mutated merely by the proposal file's existence

### Requirement: Scope is limited to contract and inbox definition

This change SHALL define only the proposal contract foundation and inbox layout. It SHALL NOT introduce proposal-generation commands, review decision artifacts, auto-merge behavior, dashboard review UI, or automatic proposal consumption by graph build or route generation. Existing CLI behavior SHALL remain unchanged.

#### Scenario: Proposal commands remain unavailable

- **WHEN** this change is implemented
- **THEN** `propose` is not registered as a CLI command
- **AND** `suggest-docs` is not registered as a CLI command

#### Scenario: Proposal documents carry no acceptance lifecycle fields

- **GIVEN** a proposal document produced by this change
- **WHEN** a consumer inspects the serialized JSON
- **THEN** it does not rely on status, reviewer, acceptance, or rejection fields to be valid
- **AND** review decision mechanics remain a follow-up concern

#### Scenario: Graph build and route generation do not consume proposal inbox files

- **GIVEN** one or more proposal files exist under `.scryrs/proposals/`
- **WHEN** graph build or route generation runs
- **THEN** those commands do not treat proposal inbox files as authoritative graph or route input
- **AND** explicit acceptance remains required before proposal content can become recorded evidence or route-affecting graph truth
