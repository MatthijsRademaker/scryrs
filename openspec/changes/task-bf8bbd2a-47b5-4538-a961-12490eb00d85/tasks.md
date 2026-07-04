## 1. Curator Heuristic Implementation

- [ ] 1.1 Add `debugging_playbook_proposal()` function in `crates/scryrs-curator/src/lib.rs` with threshold `counts.eventType.get("FailedLookup").copied().unwrap_or(0) >= 2`.
- [ ] 1.2 Render deterministic markdown template with sections: Subject header, Observed Failure Signal (count, score, row count), Likely Causes (`[TBD]` placeholder bullets), Evidence/Row IDs, Suggested Investigation Steps (generic template text).
- [ ] 1.3 Construct `ProposalDocument` with `targetType = DebuggingPlaybook`, `ProposedContent::Markdown(...)`, evidence links using `EvidenceSourceKind::HotspotSubject` with `row_ids` from the hotspot entry.
- [ ] 1.4 Wire `debugging_playbook_proposal()` into `generate_proposals()` as a new rule block following the same pattern as `skill_proposal()` and `memory_patch_proposal()`.
- [ ] 1.5 Update module doc comment: remove "debugging_playbook is intentionally excluded from V1 generation" and add it to the list of generated target types.

## 2. Test Infrastructure and Curator Unit Tests

- [ ] 2.1 Extend `make_hotspot` helper in `crates/scryrs-curator/src/lib.rs` with an `event_type_failed_lookup: u32` parameter that populates `counts.eventType["FailedLookup"]`.
- [ ] 2.2 Replace `no_debugging_playbook_proposals` test with:
  - [ ] 2.2a Positive threshold test: hotspot with `event_type_failed_lookup = 2` (and/or 3) generates a `debugging_playbook` proposal.
  - [ ] 2.2b Negative threshold test: hotspot with `event_type_failed_lookup = 1` does NOT generate a `debugging_playbook` proposal.
  - [ ] 2.2c Zero-count test: hotspot with `event_type_failed_lookup = 0` (default) does NOT generate a `debugging_playbook` proposal.
- [ ] 2.3 Add assertions that generated playbook content is non-empty markdown, includes the subject, includes row IDs in evidence, and validates via `ProposalDocument::validate()`.
- [ ] 2.4 Add deterministic ID test: same inputs produce same playbook proposal ID.

## 3. Integration Test Updates

- [ ] 3.1 Extend `make_hotspot` helper in `crates/scryrs-cli/src/propose.rs` with the same `event_type_failed_lookup: u32` parameter.
- [ ] 3.2 Add integration test: hotspot entries with `event_type_failed_lookup >= 2` produce `.scryrs/proposals/{id}.json` files with `targetType = debugging_playbook`.
- [ ] 3.3 Add integration test: hotspot entries with `event_type_failed_lookup = 1` do NOT produce `debugging_playbook` files.
- [ ] 3.4 Verify existing integration tests still pass (deterministic rerun, source-of-truth protection, upsert semantics, etc.).

## 4. OpenSpec Update

- [ ] 4.1 Update `openspec/specs/proposal-generation/spec.md`:
  - [ ] 4.1a Add a new `debugging_playbook` requirement with scenarios for generation-at-threshold, no-generation-below-threshold, and additive overlap.
  - [ ] 4.1b Modify the "V1 deterministic rules map evidence to target types" requirement to acknowledge the new debugging_playbook rule instead of forbidding it.
  - [ ] 4.1c Remove the "debugging_playbook is never generated" scenario.

## 5. Project Documentation Update

- [ ] 5.1 Update `.devagent/docs/docs/proposals.md`:
  - [ ] 5.1a Replace "Not generated in v1" with "Generated (requires FailedLookup count >= 2)" in the target types table.
  - [ ] 5.1b Add a new row to the heuristic rules table: `Debugging playbook` | `counts.eventType["FailedLookup"] >= 2` | `debugging_playbook` proposal.
  - [ ] 5.1c Document that `debugging_playbook` proposals are additive with `skill` and `memory_patch` proposals for the same hotspot subject.

## 6. Verification

- [ ] 6.1 Run `scripts/test` for the full workspace — all curator unit tests, propose integration tests, and existing tests must pass.
- [ ] 6.2 Run `scripts/check` — clippy and fmt must pass.
- [ ] 6.3 Manually verify that two `FailedLookup`-carrying hotspot entries produce a `debugging_playbook` proposal in `.scryrs/proposals/` with deterministic content.