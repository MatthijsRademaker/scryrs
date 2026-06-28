## 1. CLI Surface and Dispatch

- [x] 1.1 Add a grouped `proposals` root command to `crates/scryrs-cli` with `list`, `accept`, and `reject` subcommands.
- [x] 1.2 Keep `scryrs propose` unchanged and document the singular generation vs plural review naming split in help and docs.
- [x] 1.3 Ensure review commands are available on the unconditional CLI path rather than behind the `curator` feature gate.
- [x] 1.4 Implement command-specific usage and fail-loud diagnostics for missing arguments, invalid flags, unknown proposal IDs, and invalid state filters.

## 2. Proposal State Listing

- [x] 2.1 Implement `scryrs proposals list <PATH> [--state pending|accepted|rejected|all]` by scanning `.scryrs/proposals/`, `.scryrs/accepted/`, and `.scryrs/rejected/`.
- [x] 2.2 Validate each proposal document before listing it and fail with exit code `2` on malformed JSON, schema-version mismatch, semantic validation failure, or conflicting reviewed state.
- [x] 2.3 Emit deterministic JSON sorted by `proposalId` ascending, with `proposalId`, `title`, `targetType`, `createdAt`, and `state` fields.
- [x] 2.4 Validate any existing accepted/rejected review-decision artifacts encountered during listing and fail loudly on invalid reviewed artifacts.

## 3. Accept and Reject Commands

- [x] 3.1 Implement `scryrs proposals accept <PATH> <ID> --reviewer <NAME> --rationale <TEXT> --decided-at <RFC3339>`.
- [x] 3.2 Implement `scryrs proposals reject <PATH> <ID> --reviewer <NAME> --rationale <TEXT> --decided-at <RFC3339>`.
- [x] 3.3 Require the source proposal to deserialize as `ProposalDocument` and pass `ProposalDocument::validate()` before either command can write a decision.
- [x] 3.4 Write accepted decisions to `.scryrs/accepted/{proposalId}.json` with `targetType`, `acceptedContent`, and `sourceEvidence` copied from the proposal as specified by `ProposalReviewDecision`.
- [x] 3.5 Write rejected decisions to `.scryrs/rejected/{proposalId}.json` with `outcome = rejected`, copied `sourceEvidence`, and no accepted-content fields.
- [x] 3.6 Preserve `.scryrs/proposals/{proposalId}.json` unchanged for both commands.

## 4. Determinism, Conflicts, and Protected Boundaries

- [x] 4.1 Implement the idempotency/conflict matrix: byte-identical same-outcome reruns succeed; different same-outcome bytes or opposite-outcome states fail with exit code `2` and no overwrite.
- [x] 4.2 Treat simultaneous accepted and rejected artifacts for the same proposal ID as a conflicting terminal state that causes `proposals list` to fail loudly.
- [x] 4.3 Return exit code `2` for invalid proposal documents, missing required review metadata, unknown proposal IDs, and invalid list filters; return exit code `1` for serialization or filesystem write failures.
- [x] 4.4 Share or extract the existing inventory-based no-mutation test helper so proposal review commands prove they do not create, modify, or delete `.devagent/docs/`, `.scryrs/graph.json`, `.scryrs/routes.json`, or files under `.scryrs/proposals/`.

## 5. Help, Machine Surface, Docs, and Tests

- [x] 5.1 Update `scryrs --help`, `scryrs proposals --help`, and `scryrs --help-json` to document the grouped review surface, required metadata, and exit-code behavior.
- [x] 5.2 Represent `proposals` as a grouped command with nested subcommands in `--help-json` and bump `surfaceVersion` from `0.8.0` to `0.9.0`.
- [x] 5.3 Update committed help/help-json snapshots and command-surface tests.
- [x] 5.4 Add deterministic write, invalid-input, conflict-state, and protected-boundary tests for `list`, `accept`, and `reject`.
- [x] 5.5 Update `.devagent/docs/docs/cli-v0-contract.md` and `.devagent/docs/docs/proposals.md` to describe the final commands, review metadata, three-zone layout, deterministic behavior, exit codes, and deferred non-goals.
