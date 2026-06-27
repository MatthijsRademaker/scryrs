## Why

scryrs needs a trust-preserving proposal contract before any model-generated knowledge suggestion can become durable project knowledge. Today the repository has a placeholder `KnowledgeProposal` type, no stable inbox layout, and no explicit schema that lets a reviewer inspect evidence, rationale, target type, and proposed content without risking silent mutation of published docs, memory, or graph truth.

This change establishes the proposal contract foundation for docs notes, ADRs, skills, debugging playbooks, memory patches, and semantic graph grouping candidates. It preserves the product direction already stated in the vision and roadmap: promote repeated context into reviewable proposals first, and require explicit human acceptance before anything becomes recorded evidence or published documentation.

## What Changes

1. Add a new `proposal-contract` OpenSpec capability that defines a versioned `ProposalDocument` schema with an independent `PROPOSAL_SCHEMA_VERSION`.
2. Define the review artifact shape as executable shared contract types in `crates/scryrs-types`, replacing the current placeholder `KnowledgeProposal` with a schema that includes deterministic proposal identity, `targetType`, `title`, non-empty `rationale`, target-type-specific `proposedContent`, non-empty `evidence`, and `createdAt`.
3. Reuse the existing `EvidenceLink` vocabulary for proposal provenance so docs, memory, and graph proposals cite the same evidence source kinds already used by graph and route contracts.
4. Define six explicit proposal target types: `docs_note`, `adr`, `skill`, `debugging_playbook`, `memory_patch`, and `semantic_graph_grouping`.
5. Require `semantic_graph_grouping` proposals to carry explicit source graph node IDs plus evidence citations before a higher-level grouping such as `domain_term:auth` can be accepted.
6. Define a flat inbox layout at `.scryrs/proposals/` with one JSON file per proposal using a deterministic content-addressed filename derived from the proposal target type plus proposed content.
7. Explicitly defer review decision artifacts, accepted/rejected lifecycle mechanics, dashboard review UI, proposal generation commands, and any direct mutation of source-of-truth docs, memory, `graph.json`, or `routes.json`.
8. Include the migration note required by refinement: current placeholder consumers such as `scryrs-adapter-markdown` and `scryrs-curator` must be updated only as needed to compile against the new contract, without expanding into real proposal generation behavior.

## Impact

- **OpenSpec change**: adds `design.md`, replaces the proposal/tasks stubs, and introduces `specs/proposal-contract/spec.md`.
- **Shared contract surface**: `crates/scryrs-types/src/lib.rs` gains the authoritative proposal schema and its independent schema-version constant.
- **Compatibility updates**: existing placeholder consumers in `crates/scryrs-adapter-markdown` and `crates/scryrs-curator` need compatibility-only migration to the new contract.
- **Review workflow boundary**: proposal artifacts become explicit review inbox files under `.scryrs/proposals/`; they are not source-of-truth mutations and are not consumed automatically by graph build, route generation, or publishing flows.
- **CLI stability**: `propose` and `suggest-docs` remain unknown commands; this task adds no generator, no auto-merge behavior, and no new review command surface.