## 1. Argument Parsing and Flag Infrastructure

- [ ] 1.1 Add `is_markdown_target_type` helper to `proposals.rs` (returns true for `DocsNote`, `Adr`, `Skill`, `DebuggingPlaybook`)
- [ ] 1.2 Parse `--content-file <PATH>` in `execute_review_cli` argument loop: read file bytes, validate non-empty, wrap in `ProposedContent::Markdown`
- [ ] 1.3 Parse `--content-stdin` in `execute_review_cli` argument loop: drain stdin into string, validate non-empty, wrap in `ProposedContent::Markdown`
- [ ] 1.4 Enforce mutual exclusivity of `--content-file` and `--content-stdin` at parse time with exit code `2`
- [ ] 1.5 Reject `--content-file` and `--content-stdin` on `reject` with a dedicated error and exit code `2`
- [ ] 1.6 Reject `--content-file` and `--content-stdin` for structured target types with exit code `2` and a clear unsupported-target error
- [ ] 1.7 Resolve `--content-file` paths relative to CWD (not repo root)

## 2. Stdin Threading

- [ ] 2.1 Add `stdin: &mut impl Read` parameter to `execute_proposals_cli`
- [ ] 2.2 Thread `stdin` through `execute_proposals_cli` into `execute_review_cli`
- [ ] 2.3 Pass `&mut stdin` from `dispatch::run_with_io` to `execute_proposals_cli`

## 3. Decision Construction and Validation

- [ ] 3.1 Thread optional override `Option<ProposedContent>` through `build_review_decision`
- [ ] 3.2 In `build_review_decision`: when override is `Some` and target is Markdown, use override as `acceptedContent`; when override is `None`, preserve copy-from-proposal
- [ ] 3.3 Relax `validate_review_decision_matches_proposal`: for Markdown targets, skip `acceptedContent == proposedContent` check while preserving `proposalId`, `targetType`, and `sourceEvidence` matching; structured targets remain strict
- [ ] 3.4 Verify `load_review_decisions` path (list command) also applies relaxed validation for Markdown targets via the same function

## 4. Help, Machine Surface, and Documentation

- [ ] 4.1 Update `write_proposals_help` and `write_review_help` in `help_text.rs` to document `--content-file` and `--content-stdin` on the accept subcommand
- [ ] 4.2 Update `help_json.rs` to include `--content-file` and `--content-stdin` flags in the accept subcommand entry
- [ ] 4.3 Bump `SURFACE_VERSION` in `help_json.rs` from `0.16.0` to `0.17.0`
- [ ] 4.4 Update `openspec/specs/proposal-review-cli/spec.md`: add override scenarios, relax acceptedContent equality for Markdown targets, fix stale `surfaceVersion` references (`0.9.0` → `0.17.0`)
- [ ] 4.5 Update `.devagent/docs/docs/cli-v0-contract.md`: document `--content-file`/`--content-stdin` flags, note that acceptedContent may now differ from proposedContent for Markdown targets
- [ ] 4.6 Update `.devagent/docs/docs/proposals.md`: update Review CLI section to document content override flags and relaxed Markdown content behavior

## 5. Tests

- [ ] 5.1 Add test: `proposals_accept_content_file_success` — creates accepted artifact with overridden Markdown content, verifies proposal inbox unchanged
- [ ] 5.2 Add test: `proposals_accept_content_stdin_success` — verifies stdin path via `run_with_io` produces correct accepted artifact
- [ ] 5.3 Add test: `proposals_accept_content_file_and_stdin_mutually_exclusive` — supplying both flags exits `2`
- [ ] 5.4 Add test: `proposals_reject_rejects_content_override_flags` — `--content-file` and `--content-stdin` on reject exit `2`
- [ ] 5.5 Add test: `proposals_accept_content_override_structured_target_rejected` — structured target with override exits `2`, no artifact written
- [ ] 5.6 Add test: `proposals_accept_overridden_content_idempotent` — re-accepting with same overridden content succeeds as no-op
- [ ] 5.7 Add test: `proposals_accept_overridden_content_conflict_different_bytes` — re-accepting with different override bytes fails exit `2`
- [ ] 5.8 Add test: `proposals_list_relaxed_markdown_accepted_content_passes` — list succeeds when Markdown acceptedContent differs from proposedContent
- [ ] 5.9 Migrate `proposals_list_invalid_review_artifact_exits_2` to use `MemoryPatch` target to preserve strict-equality coverage
- [ ] 5.10 Regenerate insta snapshots for `--help` and `--help-json`
- [ ] 5.11 Update `SURFACE_VERSION` assertion in `dispatch_tests.rs` (and any other test files that assert `0.16.0`)

## 6. Verification

- [ ] 6.1 Run `scripts/precommit-run` — all tests pass, no snapshot drift
- [ ] 6.2 Run `openspec validate --strict openspec/specs/proposal-review-cli/spec.md` — spec is valid
- [ ] 6.3 Manual smoke: `scryrs proposals accept . <ID> --content-file reviewed.md --reviewer me --rationale "edited" --decided-at <RFC3339>` creates valid accepted artifact with reviewed content
- [ ] 6.4 Manual smoke: `echo "# Reviewed" | scryrs proposals accept . <ID> --content-stdin --reviewer me --rationale "edited" --decided-at <RFC3339>` creates valid accepted artifact