## 1. OpenSpec Artifacts

- [x] 1.1 Replace the change stubs with final `proposal.md`, `design.md`, and implementation `tasks.md` content for the proposal-contract foundation.
- [x] 1.2 Add `openspec/changes/task-d5be595a-6dac-4f21-8ae9-3c90fac4803b/specs/proposal-contract/spec.md` defining the versioned `ProposalDocument` contract, required fields, allowed target types, semantic grouping requirements, inbox layout, and review-first guardrails.

## 2. Shared Contract Implementation

- [x] 2.1 Replace the placeholder `KnowledgeProposal` in `crates/scryrs-types/src/lib.rs` with executable serde types for `ProposalDocument`, target-type enums, target-type-specific `proposedContent`, and an independent `PROPOSAL_SCHEMA_VERSION`.
- [x] 2.2 Reuse the existing `EvidenceLink` type for proposal evidence instead of introducing a second provenance vocabulary.
- [x] 2.3 Add contract tests proving proposals with empty `rationale`, empty `evidence`, or empty `proposedContent` are invalid, and proving `semantic_graph_grouping` proposals require non-empty `sourceNodeIds`.

## 3. Compatibility-Only Migration

- [x] 3.1 Update `crates/scryrs-adapter-markdown` to compile and test against the new shared proposal contract without adding publishing or review-workflow behavior.
- [x] 3.2 Update `crates/scryrs-curator` only as needed to remain a placeholder surface against the new contract; do not add proposal file generation, CLI registration, or any auto-apply behavior.
- [x] 3.3 Add or update migration notes in code/docs where the old placeholder `KnowledgeProposal` surface is being replaced.

## 4. Inbox Layout and Scope Guardrails

- [x] 4.1 Define `.scryrs/proposals/` as the flat proposal inbox with one JSON file per proposal and deterministic content-addressed filenames.
- [x] 4.2 Ensure the spec and implementation state that proposal files are review artifacts only and never directly mutate docs, ADRs, skills, playbooks, memory truth, `graph.json`, or `routes.json`.
- [x] 4.3 Preserve current CLI/help/dispatch behavior so `propose` and `suggest-docs` remain unknown commands and no new proposal-generation surface ships in this task.
- [x] 4.4 Do not introduce review decision artifacts, accepted/rejected inbox mechanics, or dashboard review UI in this change.