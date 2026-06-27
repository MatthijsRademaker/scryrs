# graph-domain-docs Specification

## Purpose
TBD - created by archiving change task-9321f2d6-9ce1-47a5-93fe-96891b18935d. Update Purpose after archive.
## Requirements
### Requirement: A dedicated domain-oriented graph page exists

The system SHALL include a new documentation page at `.devagent/docs/docs/graph.md` that explains the graph concept in domain terms before referencing any schema, architecture, or implementation detail.

#### Scenario: Page file exists with required structure
- **GIVEN** the project docs tree at `.devagent/docs/docs/`
- **WHEN** a reader navigates to the graph page
- **THEN** the page opens with a problem statement explaining what user/developer pain graph knowledge surfaces (agents and docs systems need explainable, reusable context instead of rediscovering relationships every session)
- **AND** the page defines what a scryrs graph represents: route-ready knowledge connecting docs, concepts, and evidence-backed relationships
- **AND** the page explains why explainable routing needs graph structure instead of flat keyword lists

### Requirement: The page explains core graph concepts in plain language

The page SHALL describe nodes, directed edges, evidence links, and evidence source kinds in domain terms using current truth sources.

#### Scenario: Node and edge concepts are explained in domain terms
- **GIVEN** the graph.md page
- **WHEN** a reader inspects the concept explanation section
- **THEN** nodes are explained as stable, identifiable concepts (files, symbols, doc pages, domain terms) with labels, tags, and aliases for search and filtering
- **AND** directed edges are explained as relationship assertions between nodes with string-backed relationship kinds and evidence provenance
- **AND** the explanation makes clear that direction is meaningful (source → target)

#### Scenario: Evidence links are explained with all five source kinds
- **GIVEN** the graph.md page
- **WHEN** a reader inspects the evidence links explanation
- **THEN** the five `EvidenceSourceKind` variants are described: `hotspot_subject` (hotspot evidence), `local_trace_row` (local session trace rows), `server_trace_row` (live server trace rows), `doc_reference` (documentation citations), `recorded_evidence` (recorded evidence descriptors)
- **AND** each variant's typical provenance is explained in plain language

#### Scenario: Deterministic output is explained
- **GIVEN** the graph.md page
- **WHEN** a reader inspects the concept explanation
- **THEN** the page explains that graph output is deterministic (same input → same output) with nodes sorted by id, edges sorted by id, and evidence links sorted by a documented tie-break chain

### Requirement: The page distinguishes current implementation from future roadmap

The page SHALL include a "Current Implementation Boundary" section explicitly stating that only the `KnowledgeGraphDocument` contract and `KnowledgeGraph` container (validation and materialization) are shipped, and that no `scryrs graph` CLI command, graph build pipeline, route-manifest generator, docs crawler, adapter integration, server endpoints, or runtime retrieval behavior exists today.

#### Scenario: Implementation boundary lists shipped vs deferred
- **GIVEN** the graph.md page
- **WHEN** a reader inspects the implementation boundary section
- **THEN** a table or explicit list distinguishes shipped (KnowledgeGraphDocument contract, KnowledgeGraph container/validation/materialization, deterministic serialization) from deferred (graph build, route-manifest generator, docs crawler, runtime retrieval, scryrs graph CLI command)
- **AND** every deferred item carries an explicit "Future Phase 5+" label

#### Scenario: No scryrs graph command is implied as existing
- **GIVEN** the graph.md page
- **WHEN** a reader reads the page from start to finish
- **THEN** the page never describes `scryrs graph build` or any graph CLI subcommand as a shipped feature
- **AND** the page never implies that issuing `scryrs graph` at the command line will produce output

### Requirement: The page explains the graph's role in the product loop

The page SHALL describe how graph work conceptually fits the Observe → Detect → Promote → Route product loop and how hotspot/document evidence feeds future routing outputs, while labeling build/manifests/runtime features as future Phase 5+ scope.

#### Scenario: Product loop connection is explained
- **GIVEN** the graph.md page
- **WHEN** a reader inspects the product-loop section
- **THEN** the page explains that graph is the foundational structure that makes Route possible — traces and hotspots (Observe→Detect) provide evidence, graph organizes that evidence into structured relationships, and route manifests (future) use those relationships to tell agents what context to load
- **AND** the page explicitly labels graph build, route manifests, and runtime retrieval as "Future Phase 5+" work

#### Scenario: Connection to hotspots and traces is explained
- **GIVEN** the graph.md page
- **WHEN** a reader inspects the evidence-flow description
- **THEN** the page explains that hotspot evidence (score, row IDs, subject identity) becomes evidence links on graph nodes, providing provenance from agent behavior to graph relationships
- **AND** the page explains that doc references and recorded evidence complement trace-based evidence for concepts not yet appearing in agent sessions

### Requirement: The page includes an illustrative JSON example

The page SHALL include a small annotated JSON example of `KnowledgeGraphDocument` showing at least one node, one directed edge, and one evidence link, with a prominent callout referencing `openspec/specs/graph-contract/spec.md` and `crates/scryrs-types/src/lib.rs` for the complete wire contract.

#### Scenario: JSON example shows core types concretely
- **GIVEN** the graph.md page
- **WHEN** a reader inspects the JSON example
- **THEN** the example contains a `KnowledgeGraphDocument` with `schemaVersion`, `metadata`, `nodes`, and `edges`
- **AND** at least one node has `id`, `label`, `kind`, `tags`, and `evidenceLinks`
- **AND** at least one edge has `id`, `sourceNodeId`, `targetNodeId`, `relationship`, and `evidenceLinks`
- **AND** at least one evidence link has `sourceKind`, `subject`, and `rowIds`
- **AND** a callout states that the graph-contract spec is the canonical source for full wire-shape details

### Requirement: The page is discoverable from docs navigation

The docs navigation at `.devagent/docs/docs/_nav.json` SHALL include an entry linking to the new graph page.

#### Scenario: Nav entry exists under Technical section
- **GIVEN** the updated `_nav.json` file
- **WHEN** a reader opens the docs navigation
- **THEN** an entry with text "Graph" and link "/graph" appears under the "Technical" navigation section
- **AND** the entry appears after the Hotspots entry

### Requirement: Existing related pages cross-link to the new graph page

At minimum, `vision.md`, `architecture.mdx`, `roadmap.mdx`, `hotspots.md`, and `trace-hook-contract.md` SHALL include a link to the new graph page in their Related Pages sections.

#### Scenario: vision.md links to graph
- **GIVEN** the updated vision.md file
- **WHEN** a reader inspects the Related Pages section
- **THEN** a link to `./graph.md` is present with a brief description

#### Scenario: architecture.mdx links to graph
- **GIVEN** the updated architecture.mdx file
- **WHEN** a reader inspects the Related Pages section
- **THEN** a link to `./graph.md` is present with a brief description

#### Scenario: roadmap.mdx links to graph
- **GIVEN** the updated roadmap.mdx file
- **WHEN** a reader inspects the Related Pages section
- **THEN** a link to `./graph.md` is present with a brief description

#### Scenario: hotspots.md links to graph
- **GIVEN** the updated hotspots.md file
- **WHEN** a reader inspects the Related Pages section
- **THEN** a link to `./graph.md` is present with a brief description

#### Scenario: trace-hook-contract.md links to graph
- **GIVEN** the updated trace-hook-contract.md file
- **WHEN** a reader inspects the Related Pages section
- **THEN** a link to `./graph.md` is present with a brief description

### Requirement: All documentation claims are verified against source of truth

Every claim in the graph documentation page that describes graph types, validation behavior, deterministic ordering, implementation scope, or product-loop placement SHALL be verified against the current source-of-truth files before the page is considered complete.

#### Scenario: Type claims match scryrs-types
- **GIVEN** the graph.md page makes a claim about `GraphNode`, `GraphEdge`, `EvidenceLink`, `EvidenceSourceKind`, or `KnowledgeGraphDocument` fields
- **WHEN** a reviewer compares the claim against `crates/scryrs-types/src/lib.rs`
- **THEN** the claim matches the implemented Rust type definitions including field names, types, and serialization attributes

#### Scenario: Implementation boundary claims match scryrs-graph
- **GIVEN** the graph.md page makes a claim about what `KnowledgeGraph` implements or does not implement
- **WHEN** a reviewer compares the claim against `crates/scryrs-graph/src/lib.rs`
- **THEN** the claim matches the documented non-goals: no graph build, CLI commands, route-manifest generation, docs crawling, adapter integration, server endpoints, or runtime retrieval

#### Scenario: Contract claims match the graph-contract spec
- **GIVEN** the graph.md page makes a claim about deterministic ordering, validation rules, schema versioning, or evidence link sorting
- **WHEN** a reviewer compares the claim against `openspec/specs/graph-contract/spec.md`
- **THEN** the claim is consistent with the canonical requirement scenarios

### Requirement: The docs site builds successfully

The Rspress documentation site SHALL build successfully from `.devagent/docs/` with no broken links or build errors after adding the new page and cross-links.

#### Scenario: bun run build completes without errors
- **GIVEN** the updated docs tree including graph.md, modified _nav.json, and modified existing pages
- **WHEN** `bun run build` is executed in `.devagent/docs/`
- **THEN** the build completes with exit code 0
- **AND** no broken link warnings appear in the build output
- **AND** the new `/graph` route is present in the generated site

### Requirement: No production code, OpenSpec specs, or non-doc artifacts are modified

This change SHALL NOT modify any Rust source code, Cargo configuration, OpenSpec specification files (outside of this change's own specs directory), LikeC4 architecture diagrams, test fixtures, CI configuration, or the root README.md.

#### Scenario: Only documentation files under .devagent/docs/docs/ are changed
- **GIVEN** the diff of this change
- **WHEN** a reviewer inspects changed files
- **THEN** all changed files are under `.devagent/docs/docs/`
- **AND** no files in `crates/`, `openspec/specs/`, `.devagent/architecture/`, `.github/`, or the repository root (except this change's OpenSpec artifacts) are modified

#### Scenario: Graph schema, validation, and contract behavior is unchanged
- **GIVEN** the existing test suite for `scryrs-graph` and `scryrs-types`
- **WHEN** tests are run after this documentation change
- **THEN** all tests pass with identical results as before the change
- **AND** no snapshot files require updating

