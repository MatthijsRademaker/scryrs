## Context

`scryrs proposals accept` currently has three behavioral pillars that block reviewed-content overrides:

1. **Argument parsing** (`execute_review_cli` in `proposals.rs`): only `--reviewer`, `--rationale`, and `--decided-at` are recognized; any other dashed token triggers "unexpected argument" exit 2.
2. **Decision construction** (`build_review_decision` in `proposals.rs`): always sets `acceptedContent = Some(proposal.proposed_content.clone())`, with no path for external content.
3. **Validation** (`validate_review_decision_matches_proposal` in `proposals.rs`): requires `acceptedContent == proposedContent` for all accepted outcomes, unconditionally.

Additionally, `dispatch::run_with_io` calls `execute_proposals_cli(&mut out, &mut err, &args[1..])` without passing the stdin handle, so `--content-stdin` has no data source.

The `scryrs-types` schema layer already permits divergent `acceptedContent` for Markdown targets (via `ProposalReviewDecision::validate()`), so no type or schema changes are needed. The existing manual-argument-parsing loop and the `record` command's stdin threading pattern both provide clean extension points.

## Goals / Non-Goals

### Goals
- Add optional, mutually exclusive `--content-file <PATH>` and `--content-stdin` flags to `scryrs proposals accept`.
- For Markdown targets, use override bytes as `acceptedContent` while leaving the proposal inbox unchanged.
- Preserve the no-override copy-from-proposal behavior when neither flag is supplied.
- Reject structured-target overrides loudly with exit code `2` and no artifact written.
- Relax `validate_review_decision_matches_proposal` for Markdown targets: require matching `proposalId`, `targetType`, and `sourceEvidence`, but permit accepted-content divergence.
- Thread stdin from `dispatch::run_with_io` through the proposals executor chain.
- Update help text, help-json surface version, OpenSpec, and project docs.
- Add test coverage for override paths, stdin, idempotency with overridden bytes, byte-different conflicts, and structured-target rejection.

### Non-Goals
- No `--content-file`/`--content-stdin` on `scryrs proposals reject` (these flags are accept-only).
- No structured JSON override support for `memory_patch` or `semantic_graph_grouping` in this task.
- No changes to proposal ID computation, proposal generation rules, or publishing behavior.
- No `ProposalReviewDecision` schema changes — the existing contract already supports divergent Markdown content.

## Decisions

### Decision 1: Content override flags are accept-only and mutually exclusive

**Choice**: Parse `--content-file <PATH>` and `--content-stdin` only in the `accept` path. Reject them on `reject` with a dedicated error. Enforce mutual exclusivity at parse time: supplying both flags fails with exit code `2`.

**Rationale**: The task only requests accept-side overrides. Keeping flags accept-only avoids ambiguity about what "reviewed content" means in a rejection. Mutual exclusivity avoids precedence questions between file and stdin.

### Decision 2: Markdown-vs-structured classification via a helper

**Choice**: Introduce `fn is_markdown_target_type(ProposalTargetType) -> bool` that returns true for `DocsNote`, `Adr`, `Skill`, and `DebuggingPlaybook`, and false for `MemoryPatch` and `SemanticGraphGrouping`. Use this helper in both `build_review_decision` (to select override content) and `validate_review_decision_matches_proposal` (to relax content equality).

**Rationale**: Centralizing the Markdown/structured split in one function prevents drift between the build and validation paths. The six target types are defined in the `ProposalTargetType` enum in `scryrs-types/src/lib.rs`.

### Decision 3: `--content-file` paths resolved relative to CWD

**Choice**: Resolve `--content-file` paths relative to the process working directory, consistent with how `record --file` works. Do not resolve relative to the repository PATH argument.

**Rationale**: CWD-relative is the established convention for `--file` flags in the codebase. Repository-relative resolution would introduce inconsistency and requires repo-root derivation before the file is even read.

### Decision 4: No-override default preserves copy-from-proposal

**Choice**: When neither `--content-file` nor `--content-stdin` is supplied for a Markdown target, `accept` continues to copy `proposal.proposedContent` into `acceptedContent`.

**Rationale**: This is the lower-risk compatibility path. The task text permits either compatibility or explicit-content-required, but the dossier assumption and all three accepted reviewer decisions favor backward compatibility. A future task could add a `--require-explicit-content` flag if desired.

### Decision 5: Stdin threading follows the `record` command pattern

**Choice**: Add `stdin: &mut impl Read` as a parameter to `execute_proposals_cli` and thread it through to `execute_review_cli`. In `dispatch::run_with_io`, pass `&mut stdin` to the proposals call. When `--content-stdin` is parsed, read all stdin bytes into a `String`, then wrap in `ProposedContent::Markdown`.

**Rationale**: `execute_record` (at `record.rs`) already accepts `stdin: &mut impl Read` following the same pattern. No new abstractions needed.

### Decision 6: Validation relaxation hits both `write_review_decision` and `load_review_decisions` paths

**Choice**: Modify `validate_review_decision_matches_proposal` to check the target type: for Markdown targets, skip the `acceptedContent == proposedContent` equality check while still enforcing `proposalId`, `targetType`, and `sourceEvidence` matching. For structured targets, keep the existing strict equality.

**Rationale**: This function is called from both `write_review_decision` (accept write path) and `load_review_decisions` (list read path). If the relaxation is incomplete on either path, `list` would reject accepted artifacts that `accept` just wrote — an inconsistency that would produce silent data loss.

### Decision 7: SURFACE_VERSION bump to 0.17.0

**Choice**: Bump `SURFACE_VERSION` in `help_json.rs` from `0.16.0` to `0.17.0`.

**Rationale**: Adding two new optional flags to the `accept` subcommand is an additive CLI surface change. The existing convention treats `SURFACE_VERSION` as incrementing on any surface change. Stale `surfaceVersion` references in `openspec/specs/proposal-review-cli/spec.md` (currently `0.9.0`) must also be updated to `0.17.0` as part of the docs sweep.

### Decision 8: Existing test `proposals_list_invalid_review_artifact_exits_2` migrates to structured target

**Choice**: Rewrite this test to use a `MemoryPatch` target with differing `acceptedContent`. Add a new separate test that validates relaxed-Markdown-acceptedContent passes through the list path.

**Rationale**: The current test uses a `docs_note` (Markdown) target with altered `acceptedContent` and asserts exit 2. Under the new relaxed behavior, this test would incorrectly fail. Switching to `MemoryPatch` preserves the strict-path coverage. A new test ensures the relaxed Markdown path is also verified.

## Risks / Trade-offs

- **Risk: Both `write_review_decision` and `load_review_decisions` must apply the same relaxation logic.** Mitigation: Both call `validate_review_decision_matches_proposal`, so modifying that single function covers both paths.
- **Risk: Insta snapshot tests and `SURFACE_VERSION` assertions across multiple files break.** Mitigation: Update all pinned snapshots (`--help`, `--help-json`) and `SURFACE_VERSION` assertions in `dispatch_tests.rs` and `init_tests.rs` atomically.
- **Risk: `--content-stdin` with no piped data produces an empty string, which fails `ProposedContent::validate()` with "markdown must be non-empty" — exit 2, which is correct but the error may confuse users.** Mitigation: Consider a dedicated guard before validation with a message like "no content received on stdin."
- **Risk: Byte-identical idempotency with overridden content works correctly because identical Markdown produces identical serialized JSON, but any metadata change combined with the same content file still changes serialized bytes and triggers legitimate conflict.** This is existing behavior preserved correctly.

## Open Questions

- Exact error message wording for "no content received on stdin" vs "markdown must be non-empty" — resolved at implementation time.
- Whether `<MotionConfig>` wraps the whole app — N/A for this change (frontend note, not applicable here).

## Migration Plan

1. Add stdin parameter to `execute_proposals_cli` and thread through internal calls.
2. Add `is_markdown_target_type` helper.
3. Parse `--content-file` and `--content-stdin` in `execute_review_cli`, with mutual-exclusivity enforcement and accept-only scoping.
4. Thread optional override content through `build_review_decision`.
5. Relax `validate_review_decision_matches_proposal` for Markdown targets.
6. Update help text and help-json, bump `SURFACE_VERSION`.
7. Add and migrate tests.
8. Update docs and OpenSpec.

Rollback is a straight revert of the CLI changes; no storage schema, type contract, or protocol changes are involved.

## Traceability

- Task: `c5c41ae5-cd2f-448c-b418-8f92621e7eaf`
- Dossier: `2026-07-04T21:26:43.120Z`
- Accepted decisions: `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, `1-swarm-reviewer-recommendation`
- Round evidence: `round:1:agent:swarm-architect`, `round:1:agent:swarm-lead-dev`, `round:1:agent:swarm-reviewer`
- Interpreted source boundaries: `openspec/specs/proposal-review-cli/spec.md`, `crates/scryrs-cli/src/proposals.rs`, `crates/scryrs-cli/src/dispatch.rs`, `crates/scryrs-cli/src/help_text.rs`, `crates/scryrs-cli/src/help_json.rs`, `crates/scryrs-cli/src/proposals_tests.rs`, `crates/scryrs-types/src/lib.rs`, `.devagent/docs/docs/cli-v0-contract.md`, `.devagent/docs/docs/proposals.md`