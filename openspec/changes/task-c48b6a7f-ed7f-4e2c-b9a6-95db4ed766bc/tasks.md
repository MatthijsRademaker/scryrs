## 1. CLI Surface and Dispatch

- [ ] 1.1 Add an unconditional `proposals` clap namespace with `list`, `accept`, and `reject` subcommands in `crates/scryrs-cli/src/dispatch.rs`
- [ ] 1.2 Extend the root unknown-command allowlist and the nested missing-argument / unknown-argument error routing for `proposals`
- [ ] 1.3 Add a dedicated `crates/scryrs-cli/src/proposal_review.rs` module and wire dispatch to it without `curator` feature gating

## 2. Accept and Reject Command Implementation

- [ ] 2.1 Resolve the repository path, load `.scryrs/proposals/{id}.json`, and deserialize `ProposalDocument`
- [ ] 2.2 Enforce proposal validation, filename/id consistency, and computed content-address consistency before any decision write
- [ ] 2.3 Implement `scryrs proposals accept` so it writes `.scryrs/accepted/{id}.json` using `ProposalReviewDecision` with copied `targetType`, `acceptedContent`, and `sourceEvidence`
- [ ] 2.4 Implement `scryrs proposals reject` so it writes `.scryrs/rejected/{id}.json` using `ProposalReviewDecision` with copied `sourceEvidence` and no accepted payload fields
- [ ] 2.5 Reject unknown IDs, invalid `--decided-at`, malformed proposal JSON, invalid proposal documents, and any pre-existing accepted/rejected decision artifact with exit code 2 diagnostics

## 3. Proposal State Listing

- [ ] 3.1 Implement `scryrs proposals list <PATH>` with default `--state all`
- [ ] 3.2 Emit a deterministic JSON envelope sorted by proposal ID and carrying proposal-backed rows for `pending`, `accepted`, and `rejected`
- [ ] 3.3 Surface orphan decision artifacts as `orphan_accepted` and `orphan_rejected` rows when listing all states
- [ ] 3.4 Fail loudly on malformed proposal artifacts, invalid decision artifacts, and conflicting accepted+rejected decisions for the same ID

## 4. Help, Snapshots, and Documentation

- [ ] 4.1 Update human help text and command-specific usage output for `scryrs proposals ...`
- [ ] 4.2 Update `--help-json` to document the `proposals` namespace and bump `surfaceVersion` from `0.8.0` to `0.9.0`
- [ ] 4.3 Refresh help and help-json snapshots plus dispatch tests for the new command surface
- [ ] 4.4 Update `.devagent/docs/docs/cli-v0-contract.md`, `.devagent/docs/docs/proposals.md`, and `README.md` to document the review workflow and distinguish `propose` from `proposals`

## 5. Determinism and Safety Tests

- [ ] 5.1 Extract or share the file-inventory / protected-path test helpers needed by both `propose` and proposal-review tests
- [ ] 5.2 Add success-path tests for accept, reject, list filters, orphan states, deterministic JSON bytes, and deterministic stdout summaries
- [ ] 5.3 Add failure-path tests for missing required arguments, unknown IDs, malformed JSON, invalid proposal invariants, filename/id mismatch, content-address mismatch, invalid `decidedAt`, and existing decision conflicts
- [ ] 5.4 Add protected-path tests proving review commands only create `.scryrs/accepted/` or `.scryrs/rejected/` artifacts and never mutate `.scryrs/proposals/`, `.scryrs/graph.json`, `.scryrs/routes.json`, `.devagent/docs/`, or representative memory/docs source paths