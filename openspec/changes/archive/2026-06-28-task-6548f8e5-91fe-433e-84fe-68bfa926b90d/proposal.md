## Why

scryrs already has deterministic proposal generation into `.scryrs/proposals/` and a versioned `ProposalReviewDecision` contract, but reviewers still cannot move inbox artifacts into accepted or rejected review states through the CLI. That gap forces manual file surgery and leaves pending, accepted, and rejected work indistinguishable in normal CLI workflows.

This change closes the review loop without changing the existing review-first trust boundary: proposal review stays inside `.scryrs/`, reviewed decisions are recorded as separate artifacts, and no source-of-truth docs, graph, routes, or memory surfaces are mutated by accept/reject commands.

## What Changes

1. Add a grouped proposal review CLI surface: `scryrs proposals list`, `scryrs proposals accept`, and `scryrs proposals reject`.
2. Finalize required review metadata for `accept` and `reject`: `--reviewer`, `--rationale`, and `--decided-at <RFC3339>` are mandatory and have no defaults.
3. Implement deterministic proposal state listing from `.scryrs/proposals/`, `.scryrs/accepted/`, and `.scryrs/rejected/`, with machine-readable JSON sorted by proposal ID and optional state filtering.
4. Accept a valid proposal by writing `.scryrs/accepted/{proposalId}.json` as a valid `ProposalReviewDecision` that preserves `targetType`, copies `proposedContent` into `acceptedContent`, and reuses proposal evidence as `sourceEvidence`.
5. Reject a valid proposal by writing `.scryrs/rejected/{proposalId}.json` as a valid `ProposalReviewDecision` with outcome `rejected`, no `targetType`, and no `acceptedContent`, without deleting or mutating the proposal inbox artifact.
6. Define deterministic idempotency and conflict handling for repeated reviews, conflicting terminal states, unknown IDs, malformed proposal documents, invalid metadata, and invalid list filters.
7. Update `--help`, `--help-json`, committed snapshots, CLI docs, and proposal docs to document the final command names, machine surface, exit codes, metadata requirements, and the non-mutating review boundary.

## Impact

- **CLI surface**: `crates/scryrs-cli` gains a new grouped `proposals` root command and a review workflow module following the existing proposal command patterns.
- **Machine-readable help**: `--help-json` is extended to describe the grouped review surface and its nested subcommands, with `surfaceVersion` bumped to `0.9.0`.
- **Artifacts**: review commands write only `.scryrs/accepted/{proposalId}.json` and `.scryrs/rejected/{proposalId}.json`; `.scryrs/proposals/{proposalId}.json` remains review-only input.
- **Tests**: add deterministic write, invalid input, conflict-state, help/help-json snapshot, and no-mutation coverage, reusing a shared inventory-based guard for protected paths.
- **Protected boundaries**: no docs, graph, or route artifacts are mutated, and no accepted evidence is published into source-of-truth surfaces in this task.
