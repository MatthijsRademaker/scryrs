## Why

scryrs already has deterministic proposal inbox artifacts under `.scryrs/proposals/` and an implemented `ProposalReviewDecision` contract, but reviewers still have no CLI way to record accept/reject outcomes. That forces manual file creation in `.scryrs/accepted/` and `.scryrs/rejected/`, which is error-prone and undermines the review-first boundary between proposal inbox artifacts and source-of-truth knowledge.

This change adds an explicit review CLI so documentation reviewers can list proposal state, accept valid proposals, and reject out-of-direction proposals without editing files by hand. The review commands must create durable reviewed-evidence artifacts, fail loudly on invalid inputs, and leave proposal inbox files plus docs, graph, routes, and memory truth untouched.

## What Changes

1. **Add an explicit proposal review namespace**: register `scryrs proposals list <PATH> [--state pending|accepted|rejected|all]`, `scryrs proposals accept <PATH> <ID> --reviewer <NAME> --rationale <TEXT> --decided-at <RFC3339>`, and `scryrs proposals reject <PATH> <ID> --reviewer <NAME> --rationale <TEXT> --decided-at <RFC3339>` under a new plural `proposals` command group.

2. **Implement review behavior in a dedicated CLI module**: add a non-feature-gated `proposal_review.rs` flow that reads `.scryrs/proposals/{id}.json`, validates proposal invariants, enforces filename/id and content-address consistency, constructs `ProposalReviewDecision`, and writes only `.scryrs/accepted/{id}.json` or `.scryrs/rejected/{id}.json`.

3. **Define deterministic state listing and review outcomes**: `list` derives `pending`, `accepted`, and `rejected` from inbox and decision artifacts, defaults `--state` to `all`, and surfaces orphan decisions as distinct `orphan_accepted` and `orphan_rejected` states instead of hiding them. Ambiguous accepted+rejected conflicts and malformed artifacts fail loudly.

4. **Preserve the review-first artifact boundary**: accept/reject commands copy reviewed evidence into separate decision artifacts, do not delete or mutate `.scryrs/proposals/{id}.json`, and do not update `.scryrs/graph.json`, `.scryrs/routes.json`, `.devagent/docs/`, README-authored knowledge, or memory truth.

5. **Update discovery and documentation surfaces**: extend human help, `--help-json`, dispatch snapshots, CLI docs, proposals docs, and README command references to document the new namespace and its argument contract. Bump the machine surface version from `0.8.0` to `0.9.0` for the additive CLI surface.

6. **Add deterministic and protected-path tests**: cover state filters, orphan states, deterministic JSON bytes, deterministic stdout summaries, fail-loud validation paths, and protected-path invariants proving review commands only create reviewed-evidence artifacts.

## Impact

- **CLI dispatch and implementation**: `crates/scryrs-cli/src/dispatch.rs` grows a nested `proposals` subcommand group and routes to a new unconditional `crates/scryrs-cli/src/proposal_review.rs` module.
- **Review artifact behavior**: the CLI becomes a consumer of the existing `ProposalReviewDecision` contract in `crates/scryrs-types`, treating the review-contract dependency as already resolved.
- **Machine and human discovery surfaces**: `crates/scryrs-cli/src/help_text.rs`, `crates/scryrs-cli/src/help_json.rs`, help/help-json snapshots, and dispatch tests all change to reflect the new commands and `surfaceVersion` `0.9.0`.
- **Documentation updates**: `.devagent/docs/docs/cli-v0-contract.md`, `.devagent/docs/docs/proposals.md`, and `README.md` must describe the new review workflow and the distinction between `scryrs propose` (generation) and `scryrs proposals ...` (review).
- **Safety boundary preserved**: no review command mutates source-of-truth docs, graph, routes, memory truth, or the original proposal inbox artifacts.