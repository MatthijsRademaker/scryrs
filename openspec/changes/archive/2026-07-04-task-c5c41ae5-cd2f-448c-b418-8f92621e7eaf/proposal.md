## Why

`scryrs proposals accept` currently hard-copies `proposal.proposedContent` into `acceptedContent` and rejects any accepted artifact whose content differs from the proposal. The proposals command chain never receives stdin from `run_with_io`. This forces reviewers who have edited Markdown to hand-edit JSON or bypass the CLI entirely, breaking the intended review-first workflow and keeping human-reviewed wording out of accepted knowledge.

## What Changes

- Add optional, mutually exclusive `--content-file <PATH>` and `--content-stdin` flags to `scryrs proposals accept`. Both flags are accept-only; they are rejected with exit code `2` on `scryrs proposals reject`.
- For Markdown-backed target types (`docs_note`, `adr`, `skill`, `debugging_playbook`), reviewed Markdown bytes from either flag become the decision's `acceptedContent`. The source `.scryrs/proposals/{id}.json` remains byte-identical.
- When neither override flag is supplied, `accept` preserves the existing copy-from-proposal behavior.
- For structured target types (`memory_patch`, `semantic_graph_grouping`), supplying either override flag fails with exit code `2` and a clear unsupported-target error; no artifact is written.
- Relax `validate_review_decision_matches_proposal` for Markdown targets: require matching `proposalId`, `targetType`, and `sourceEvidence`, but allow `acceptedContent` to differ from `proposedContent`. Structured targets retain strict equality.
- Thread a `stdin: &mut impl Read` handle from `dispatch::run_with_io` through `execute_proposals_cli` into `execute_review_cli` and `write_review_decision`, following the same pattern as `record`.
- Update human help (`help_text.rs`), machine-readable help (`help_json.rs` with `SURFACE_VERSION` bumped from `0.16.0` to `0.17.0`), the canonical OpenSpec (`openspec/specs/proposal-review-cli/spec.md`), and project docs (`.devagent/docs/docs/cli-v0-contract.md`, `.devagent/docs/docs/proposals.md`) to describe the new accept syntax and reviewed-content behavior.
- Add test coverage for `--content-file` success, `--content-stdin` success, proposal inbox immutability with override content, byte-identical idempotency with overridden content, byte-different conflict from changed reviewed Markdown, and structured-target rejection. Migrate `proposals_list_invalid_review_artifact_exits_2` to use a structured target so the strict-equality path remains tested.

## Capabilities

### Modified Capabilities

- `proposal-review-cli`: The `accept` subcommand gains optional `--content-file`/`--content-stdin` flags. Accepted-content equality with the proposal is relaxed for Markdown targets. Structured-target overrides are explicitly rejected. Help text, help-json surface version, and CLI contract docs are updated accordingly.

## Impact

- **CLI code**: `crates/scryrs-cli/src/proposals.rs` (argument parsing, decision builder, validation relaxation, stdin threading), `crates/scryrs-cli/src/dispatch.rs` (stdin threading to proposals).
- **CLI tests**: `crates/scryrs-cli/src/proposals_tests.rs` (new override-path tests, structured-target migration of existing test, stdin-path tests via `run_with_io`). Insta snapshots for `--help` and `--help-json` must be regenerated.
- **Help surface**: `crates/scryrs-cli/src/help_text.rs` (human help for accept subcommand), `crates/scryrs-cli/src/help_json.rs` (machine-readable surface, `SURFACE_VERSION` bump to `0.17.0`).
- **Docs**: `.devagent/docs/docs/cli-v0-contract.md` and `.devagent/docs/docs/proposals.md`.
- **Spec**: `openspec/specs/proposal-review-cli/spec.md` (add override scenarios, relax acceptedContent equality for Markdown, fix stale `surfaceVersion` from `0.9.0` to `0.17.0`).
- **Out of scope**: No change to `scryrs proposals reject`, no structured JSON override support, no changes to proposal ID computation, proposal generation rules, or publishing behavior. No `ProposalReviewDecision` schema changes.