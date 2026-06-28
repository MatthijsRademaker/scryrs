# curator-llm-assist Specification

## Purpose
TBD - created by archiving change task-74406814-51f9-491c-bfea-bcf6e9bf48c5. Update Purpose after archive.
## Requirements
### Requirement: Optional model-assisted curation is isolated from deterministic contracts

The system SHALL implement Foundation 01 model-assisted drafting and semantic grouping in a dedicated `scryrs-curator-llm` crate. This capability SHALL remain outside `scryrs-curator`, `scryrs-types`, and the default-feature surface of `scryrs-cli`. With default features, deterministic ingest, hotspot scoring, graph build, route generation, proposal validation, and deterministic proposal generation SHALL compile and run without depending on `scryrs-llm` or invoking a model.

#### Scenario: Default deterministic surfaces remain model-free

- **GIVEN** the workspace is built with default features
- **WHEN** deterministic ingest, hotspot scoring, graph build, route generation, proposal validation, and deterministic proposal generation compile and run
- **THEN** those paths do not depend on `scryrs-llm`
- **AND** those paths do not invoke a model

#### Scenario: Model-aware code lives outside the deterministic curator crate

- **GIVEN** the curator implementation for Foundation 01
- **WHEN** a maintainer inspects the crate boundaries
- **THEN** model-assisted APIs are exposed by `scryrs-curator-llm`
- **AND** `scryrs-curator` remains the deterministic proposal-generation crate

### Requirement: Foundation 01 is library-only

Foundation 01 SHALL expose library APIs only for drafting and grouping. It SHALL NOT add `scryrs propose --llm`, `scryrs propose-llm`, provider credential loading, hosted-provider integration, dashboard review UI, acceptance records, or automatic graph-build consumption.

#### Scenario: CLI surface is unchanged by this slice

- **GIVEN** Foundation 01 is implemented
- **WHEN** a caller inspects the CLI surface
- **THEN** no new model-assisted propose command or flag exists
- **AND** model-assisted behavior is available only through library APIs in this slice

### Requirement: Evidence packs are bounded and evidence-backed

The model-assist layer SHALL define an `EvidencePack` plus explicit budget configuration. `EvidencePack` SHALL admit only existing hotspot entries, graph nodes, proposal documents, and document evidence already present in deterministic artifacts. The builder SHALL assign stable input-local IDs to evidence entries, preserve exact graph node IDs for grouping validation, enforce `max_input_chars` and configured per-type caps before constructing any `ModelRequest`, and fail loudly on overflow. The capability SHALL NOT silently truncate input, synthesize new evidence, or fetch tools.

#### Scenario: Oversize evidence fails before any model call

- **GIVEN** evidence whose serialized pack would exceed the configured `max_input_chars` or per-type caps
- **WHEN** the caller builds an `EvidencePack`
- **THEN** the builder returns an explicit error before any `ModelRequest` is constructed
- **AND** no model call is attempted

#### Scenario: Evidence citations are keyed to the input pack

- **GIVEN** a valid `EvidencePack`
- **WHEN** the pack is prepared for a model-assisted run
- **THEN** each evidence entry has a stable input-local citation ID
- **AND** exact graph node IDs remain available for later output validation

### Requirement: Model requests are explicit, finite, and tool-free

For every drafting or grouping run, the capability SHALL construct a `ModelRequest` with explicit request bounds and disabled tools. Each request SHALL include a non-empty model ID, `max_input_chars`, positive `max_output_tokens`, finite `timeout_ms`, and `allow_tools = false`.

#### Scenario: Drafting request disables tools and carries explicit budgets

- **GIVEN** a valid drafting call
- **WHEN** the capability constructs its `ModelRequest`
- **THEN** the request sets `allow_tools = false`
- **AND** the request includes explicit `max_input_chars`
- **AND** the request includes positive `max_output_tokens`
- **AND** the request includes a finite `timeout_ms`

### Requirement: Model-assisted drafting returns reviewable proposal drafts grounded in input evidence

The capability SHALL expose a drafting API that accepts an existing `ProposalDocument` plus an `EvidencePack` and returns a reviewable `ProposalDocument` with the same `targetType` and target/content shape. The response contract SHALL be structured, shall cite input-local evidence IDs, and shall permit the parser to reconstruct proposal evidence only from cited `EvidencePack` entries. Drafting SHALL NOT change deterministic source data.

#### Scenario: Valid drafting response preserves proposal shape

- **GIVEN** an existing deterministic `ProposalDocument` and a valid `EvidencePack`
- **WHEN** model-assisted drafting succeeds
- **THEN** the returned draft is a `ProposalDocument`
- **AND** the returned draft has the same `targetType` as the input proposal
- **AND** the returned draft preserves the input proposal's target/content shape
- **AND** every evidence entry on the returned draft comes from cited `EvidencePack` entries

#### Scenario: Uncited drafting claim is rejected

- **GIVEN** a drafting response that includes claims not backed by cited input evidence IDs
- **WHEN** the parser validates the response against the `EvidencePack`
- **THEN** the drafting run fails with an explicit error
- **AND** no review artifact is produced by this library-only slice

### Requirement: Model-assisted grouping suggestions are reviewable semantic grouping proposals with exact source identities

The capability SHALL expose a grouping API that accepts deterministic graph and hotspot evidence through an `EvidencePack` and returns zero or more `ProposalDocument` suggestions with `targetType = semantic_graph_grouping`. Each suggestion SHALL include non-empty exact `sourceNodeIds`, `targetGroupNodeId`, `targetGroupLabel`, title, rationale, and evidence reconstructed from cited input-local evidence IDs. Suggestions SHALL remain review-only proposals and SHALL NOT mutate `.scryrs/graph.json` or `.scryrs/routes.json`.

#### Scenario: Valid grouping suggestion cites exact source node IDs

- **GIVEN** graph nodes such as `file:auth`, `search:auth`, and `symbol:auth` are present in the input `EvidencePack`
- **WHEN** model-assisted grouping succeeds
- **THEN** each returned `semantic_graph_grouping` proposal contains the exact contributing node IDs in `sourceNodeIds`
- **AND** each returned proposal includes `targetGroupNodeId`
- **AND** each returned proposal includes `targetGroupLabel`
- **AND** each returned proposal's evidence is reconstructed from cited input evidence IDs

#### Scenario: Hallucinated source node ID is rejected

- **GIVEN** a grouping response that references a source node ID not present in the input `EvidencePack`
- **WHEN** the parser validates the response
- **THEN** the grouping run fails with an explicit error
- **AND** no grouping proposal is produced by this library-only slice

### Requirement: Invalid model output fails the entire model-assisted run

Malformed structured output, unknown evidence IDs, unknown source node IDs, missing evidence, or target/content mismatches SHALL fail the entire drafting or grouping run. The capability SHALL NOT return partial success sets, silently skip invalid candidates, or emit diagnostic proposal artifacts in Foundation 01.

#### Scenario: One invalid candidate aborts the entire grouping run

- **GIVEN** a grouping response containing multiple candidates
- **AND** one candidate references unknown evidence or mismatched content
- **WHEN** the response is validated
- **THEN** the entire run fails with an explicit error
- **AND** no partial success list is returned

### Requirement: Accepted grouping lifecycle remains out of scope for Foundation 01

Foundation 01 SHALL treat model-assisted output as proposal input only. It SHALL NOT define acceptance records, automatic graph mutations, automatic route changes, or deterministic graph-build consumption of proposed groupings. Converting an accepted grouping into `recorded_evidence` for later deterministic graph builds remains follow-up work.

#### Scenario: Proposed grouping is not authoritative graph input

- **GIVEN** model-assisted grouping has produced one or more reviewable proposals
- **WHEN** graph build or route generation runs
- **THEN** those proposals are not treated as authoritative graph or route input
- **AND** explicit future acceptance work is still required before any grouping can become recorded evidence

