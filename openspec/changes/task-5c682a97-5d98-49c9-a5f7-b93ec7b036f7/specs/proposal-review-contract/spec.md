## ADDED Requirements

### Requirement: Proposal review decisions are versioned durable artifacts

The system SHALL define a versioned `ProposalReviewDecision` contract for explicit proposal review outcomes. Each serialized review decision SHALL include `schemaVersion`, `proposalId`, `reviewer`, `decidedAt`, `rationale`, `sourceEvidence`, and `outcome`. `schemaVersion` SHALL equal `REVIEW_DECISION_SCHEMA_VERSION`, and `REVIEW_DECISION_SCHEMA_VERSION` SHALL be versioned independently from `SCHEMA_VERSION`, `HOTSPOT_SCHEMA_VERSION`, `GRAPH_SCHEMA_VERSION`, `ROUTE_SCHEMA_VERSION`, and `PROPOSAL_SCHEMA_VERSION`.

#### Scenario: Serialized review decision carries required common fields

- **GIVEN** a review decision artifact produced under this contract
- **WHEN** a reviewer inspects the serialized JSON
- **THEN** it contains `schemaVersion`
- **AND** it contains `proposalId`
- **AND** it contains `reviewer`
- **AND** it contains `decidedAt`
- **AND** it contains `rationale`
- **AND** it contains `sourceEvidence` as a JSON array
- **AND** it contains `outcome`

#### Scenario: Review decision schema version is independent from other artifact contracts

- **GIVEN** the workspace already defines `SCHEMA_VERSION`, `HOTSPOT_SCHEMA_VERSION`, `GRAPH_SCHEMA_VERSION`, `ROUTE_SCHEMA_VERSION`, and `PROPOSAL_SCHEMA_VERSION`
- **WHEN** the review decision contract is versioned
- **THEN** it defines a separate `REVIEW_DECISION_SCHEMA_VERSION`
- **AND** every serialized review decision uses `REVIEW_DECISION_SCHEMA_VERSION` for `schemaVersion`

### Requirement: Review outcomes are explicit and outcome-specific

The contract SHALL support exactly two review outcomes serialized as snake_case strings: `accepted` and `rejected`. An accepted review decision SHALL include `targetType` plus `acceptedContent`. A rejected review decision SHALL record explicit rejection metadata without requiring accepted content.

#### Scenario: Accepted review decision carries reviewed content

- **GIVEN** a proposal review decision with `outcome = "accepted"`
- **WHEN** the decision is serialized
- **THEN** it includes `targetType`
- **AND** it includes `acceptedContent`
- **AND** `acceptedContent` is target-type-specific reviewed content

#### Scenario: Rejected review decision omits accepted content

- **GIVEN** a proposal review decision with `outcome = "rejected"`
- **WHEN** the decision is serialized
- **THEN** it does not require `targetType`
- **AND** it does not require `acceptedContent`
- **AND** the rejection remains a valid durable review artifact

### Requirement: Review decision provenance and rationale are mandatory

Every review decision SHALL require non-empty `proposalId`, non-empty `reviewer`, non-empty `decidedAt`, non-empty `rationale`, and non-empty `sourceEvidence`. `sourceEvidence` SHALL reuse the existing `EvidenceLink` contract and SHALL NOT define a second provenance vocabulary.

#### Scenario: Empty provenance is invalid

- **GIVEN** a serialized review decision whose `sourceEvidence` array is empty
- **WHEN** the document is validated against the review decision contract
- **THEN** validation fails
- **AND** the review decision is rejected as invalid

#### Scenario: Review decision provenance uses the existing evidence vocabulary

- **GIVEN** a valid review decision
- **WHEN** a consumer inspects the `sourceEvidence` field
- **THEN** each provenance entry conforms to `EvidenceLink`
- **AND** the review decision uses existing evidence source kinds such as `hotspot_subject`, `local_trace_row`, `server_trace_row`, `doc_reference`, or `recorded_evidence`

### Requirement: Accepted content preserves proposal content shape and exact semantic grouping source nodes

Accepted review decisions SHALL reuse the existing `ProposedContent` shapes. `targetType` and `acceptedContent` SHALL match the same target-type rules used by `ProposalDocument`. When `targetType = "semantic_graph_grouping"`, `acceptedContent` SHALL preserve the exact `sourceNodeIds`, `targetGroupNodeId`, and `targetGroupLabel` carried by the reviewed grouping content.

#### Scenario: Accepted semantic grouping preserves exact source node IDs

- **GIVEN** source graph nodes `file:auth`, `search:auth`, and `symbol:auth`
- **AND** a reviewer accepts a `semantic_graph_grouping` proposal for `domain_term:auth`
- **WHEN** the accepted review decision is serialized
- **THEN** `acceptedContent.sourceNodeIds` contains `["file:auth", "search:auth", "symbol:auth"]`
- **AND** those source node IDs are preserved exactly rather than inferred or remapped

#### Scenario: Mismatched accepted content is invalid

- **GIVEN** a review decision with `outcome = "accepted"`
- **AND** its `targetType` and `acceptedContent` do not match
- **WHEN** the document is validated against the review decision contract
- **THEN** validation fails
- **AND** the review decision is rejected as invalid

### Requirement: Reviewed artifacts are stored separately from the proposal inbox

Proposal review outcomes SHALL be recorded as separate artifacts rather than by mutating proposal inbox files. Accepted decisions SHALL be stored under `.scryrs/accepted/{proposalId}.json`. Rejected decisions SHALL be stored under `.scryrs/rejected/{proposalId}.json`. The original proposal inbox artifact under `.scryrs/proposals/{proposalId}.json` SHALL remain a review-only record.

#### Scenario: Accepting a proposal produces separate reviewed evidence

- **GIVEN** a valid proposal document exists under `.scryrs/proposals/`
- **WHEN** a reviewer accepts it
- **THEN** scryrs records `.scryrs/accepted/{proposalId}.json`
- **AND** the accepted artifact contains proposal ID, reviewer decision metadata, source evidence, and accepted content
- **AND** the original proposal inbox artifact is not mutated to carry lifecycle state

#### Scenario: Rejecting a proposal is explicit and non-mutating

- **GIVEN** a valid proposal document exists under `.scryrs/proposals/`
- **WHEN** a reviewer rejects it
- **THEN** scryrs records `.scryrs/rejected/{proposalId}.json`
- **AND** the rejection does not delete or mutate `.scryrs/graph.json`, `.scryrs/routes.json`, `.devagent/docs/`, or memory truth
- **AND** the original proposal inbox artifact remains review-only
