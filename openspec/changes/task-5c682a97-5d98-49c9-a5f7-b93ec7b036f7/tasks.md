## 1. OpenSpec and Boundary Documentation

- [ ] 1.1 Add `openspec/changes/task-5c682a97-5d98-49c9-a5f7-b93ec7b036f7/specs/proposal-review-contract/spec.md` and update `specs/proposal-contract/spec.md` so the accepted/rejected review-decision boundary is canonical.
- [ ] 1.2 Update `.devagent/docs/docs/proposals.md` to explain `.scryrs/proposals/` as review inbox only and `.scryrs/accepted/` / `.scryrs/rejected/` as durable reviewed-artifact paths.
- [ ] 1.3 Update `.devagent/docs/docs/production-suite.md` to document the accepted/rejected artifact layout and restate that graph, route, docs, and memory publishing remain out of scope.

## 2. Shared Review-Decision Contract

- [ ] 2.1 Add `REVIEW_DECISION_SCHEMA_VERSION = "1.0.0"`, `ReviewOutcome`, and `ProposalReviewDecision` to `crates/scryrs-types/src/lib.rs` using camelCase serde wire fields.
- [ ] 2.2 Reuse `EvidenceLink`, `ProposalTargetType`, `ProposedContent`, and `SemanticGraphGrouping`; accepted outcomes carry `targetType` plus `acceptedContent`, while rejected outcomes carry no accepted-content payload.
- [ ] 2.3 Implement `ProposalReviewDecision::validate()` enforcing schema version, non-empty `proposalId`/`reviewer`/`decidedAt`/`rationale`/`sourceEvidence`, accepted-vs-rejected invariants, and target/content matching.

## 3. Tests and Preserved Invariants

- [ ] 3.1 Add serde round-trip tests for accepted and rejected `ProposalReviewDecision` documents.
- [ ] 3.2 Add validation tests for wrong schema version, empty required fields, empty provenance, accepted-without-content, rejected-with-content, and mismatched `targetType` / `acceptedContent`.
- [ ] 3.3 Add tests proving accepted `semantic_graph_grouping` content preserves exact `sourceNodeIds`, and keep existing `ProposalDocument` lifecycle-free behavior unchanged.

## 4. Scope Guardrails

- [ ] 4.1 Preserve `.scryrs/proposals/{proposalId}.json` as review-only inbox data and document `.scryrs/accepted/{proposalId}.json` / `.scryrs/rejected/{proposalId}.json` as separate reviewed artifacts.
- [ ] 4.2 Do not add `scryrs accept` / `scryrs reject`, dashboard review UI, graph ingestion, route changes, docs publishing, or memory mutation in this change.
