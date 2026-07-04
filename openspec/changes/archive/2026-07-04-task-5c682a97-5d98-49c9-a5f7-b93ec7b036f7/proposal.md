## Why

scryrs can already generate deterministic `ProposalDocument` inbox artifacts under `.scryrs/proposals/`, but it still lacks a durable contract for what happens after review. Without a separate accepted/rejected artifact, review outcomes stay ephemeral or force silent mutation of proposal inbox files or downstream truth surfaces.

This change defines the next contract boundary only: explicit reviewed-evidence artifacts for accepted and rejected proposal outcomes, while preserving `ProposalDocument` as a review-only inbox artifact. It also keeps the existing trust boundary intact: no graph ingestion, route updates, docs publishing, or memory mutation ship in this task.

## What Changes

1. Add a new `proposal-review-contract` OpenSpec capability that defines a unified `ProposalReviewDecision` schema with `accepted` and `rejected` outcomes.
2. Add executable shared-contract support in `crates/scryrs-types` for `REVIEW_DECISION_SCHEMA_VERSION = "1.0.0"`, reviewer decision metadata, mandatory `sourceEvidence`, and outcome-dependent accepted content.
3. Reuse existing `EvidenceLink`, `ProposalTargetType`, `ProposedContent`, and `SemanticGraphGrouping` types so reviewed artifacts preserve current proposal content shapes and provenance vocabulary.
4. Require accepted `semantic_graph_grouping` decisions to carry exact `sourceNodeIds` through the accepted content payload.
5. Define separate reviewed-artifact paths: `.scryrs/accepted/{proposalId}.json` for accepted decisions and `.scryrs/rejected/{proposalId}.json` for rejected decisions.
6. Modify the existing `proposal-contract` capability so `ProposalDocument` remains lifecycle-free and `.scryrs/proposals/` stays a review-only inbox separate from accepted/rejected artifacts.
7. Update proposal/product documentation to explain the inbox vs reviewed-evidence boundary.
8. Explicitly defer accept/reject CLI commands, dashboard review UX, graph ingestion of accepted evidence, route changes, docs publishing, and memory mutation.

## Impact

- **Shared contract surface**: `crates/scryrs-types/src/lib.rs` gains the new review-decision types, version constant, validation, and tests.
- **OpenSpec**: adds `specs/proposal-review-contract/spec.md` and updates `specs/proposal-contract/spec.md`.
- **Docs**: `.devagent/docs/docs/proposals.md` and `.devagent/docs/docs/production-suite.md` describe the three-zone review boundary.
- **Truth path**: proposal inbox files remain unchanged; accepted/rejected artifacts become durable review evidence only.
- **Deferred work**: no CLI review workflow, graph consumption, publishing, or source-of-truth mutation is introduced here.
