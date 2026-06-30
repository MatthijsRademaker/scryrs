## MODIFIED Requirements

### Requirement: Optional model-assisted curation is isolated from deterministic contracts

The system SHALL implement model-assisted drafting and semantic grouping in a dedicated `scryrs-curator-llm` crate. This capability SHALL remain outside `scryrs-curator`, `scryrs-types`, and the default-feature surface of `scryrs-cli`. With default features, deterministic ingest, hotspot scoring, graph build, route generation, proposal validation, deterministic proposal generation, and proposal review SHALL compile and run without depending on `scryrs-curator-llm`, without depending on hosted-provider crates, and without invoking a model.

#### Scenario: Default deterministic and review surfaces remain model-free

- **GIVEN** the workspace is built with default features
- **WHEN** deterministic ingest, hotspot scoring, graph build, route generation, proposal validation, deterministic proposal generation, and `scryrs proposals list|accept|reject` compile and run
- **THEN** those paths do not depend on `scryrs-curator-llm` or hosted-provider crates
- **AND** those paths do not invoke a model

#### Scenario: Model-aware review UX remains opt-in

- **GIVEN** a maintainer inspects crate and feature boundaries
- **WHEN** they compare deterministic proposal generation with model-assisted review help
- **THEN** model-aware APIs are exposed by `scryrs-curator-llm`
- **AND** any user-facing model assist is reachable only through a non-default `scryrs proposals assist` review surface
- **AND** `scryrs propose` remains deterministic

### Requirement: Review-workflow model assist is the only supported user-facing UX in this slice

This slice SHALL expose user-facing model assistance only through the plural proposal review workflow. It SHALL NOT add `scryrs propose --llm`, provider credential loading, hosted-provider configuration, default provider selection, dashboard review UI, acceptance records, or automatic graph-build or route consumption.

#### Scenario: Deterministic proposal generation does not gain model flags

- **WHEN** a caller inspects the deterministic proposal-generation CLI surface
- **THEN** no `scryrs propose --llm` or equivalent model-assisted generation flag exists
- **AND** model-assisted behavior is described only as optional review assistance

#### Scenario: Provider configuration remains out of scope

- **WHEN** a caller inspects the assist contract for configuration options
- **THEN** they can set request and evidence bounds plus `--model <MODEL_ID>`
- **AND** they cannot configure hosted providers, credentials, or a default provider in this slice

### Requirement: Evidence packs are bounded, evidence-backed, and user-visible

The model-assist layer SHALL define an `EvidencePack` plus explicit budget configuration. `EvidencePack` SHALL admit only existing hotspot entries, graph nodes, proposal documents, and document evidence already present in deterministic artifacts. The builder SHALL assign stable input-local IDs to evidence entries, preserve exact graph node IDs for grouping validation, enforce `max_input_chars` and configured per-type caps before constructing any `ModelRequest`, and fail loudly on overflow. The capability SHALL NOT silently truncate input, synthesize new evidence, or fetch tools. Any user-facing assist help or API contract SHALL expose the default caps and SHALL explain that returned evidence must resolve to cited input-pack entries.

#### Scenario: Oversize evidence fails before any model call

- **GIVEN** evidence whose serialized pack would exceed the configured `max_input_chars` or per-type caps
- **WHEN** the caller builds an `EvidencePack`
- **THEN** the builder returns an explicit error before any `ModelRequest` is constructed
- **AND** no model call is attempted

#### Scenario: Assist help exposes default bounds and citation rules

- **GIVEN** the assist surface is available
- **WHEN** a reviewer inspects command help or equivalent API documentation
- **THEN** the default caps for `max_input_chars`, `max_hotspots`, `max_graph_nodes`, `max_proposals`, and `max_documents` are visible
- **AND** the documentation states that output evidence must resolve to cited entries from the input pack
