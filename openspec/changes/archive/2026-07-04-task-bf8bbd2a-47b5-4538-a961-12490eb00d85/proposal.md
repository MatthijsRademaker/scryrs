## Why

`debugging_playbook` already exists in the proposal contract (`ProposalTargetType::DebuggingPlaybook`) and publishing path (`scryrs-adapter-markdown`), but deterministic proposal generation explicitly excludes it today. Repeated `FailedLookup` dead ends show up in hotspot evidence — high-score, high-count, repeated failures for the same subject — yet they never become durable recovery guidance. This closes that gap by adding a deterministic, evidence-backed playbook heuristic to `scryrs propose` without invoking model-assisted drafting or changing review-first boundaries.

## What Changes

- **New deterministic heuristic** `debugging_playbook_proposal()` in `crates/scryrs-curator/src/lib.rs` keyed on `counts.eventType["FailedLookup"] >= 2`. The rule follows the identical pattern as existing `skill_proposal()` and `memory_patch_proposal()`: iterate hotspots, check a signal threshold, render a markdown template, construct `EvidenceLink` evidence, and return `Some(ProposalDocument)`.
- **Deterministic markdown template** with five sections: Subject header, Observed Failure Signal (FailedLookup count, score, row count), Likely Causes (placeholder bullets — `[TBD]` markers), Evidence/Row IDs (cited via `EvidenceLink` with `sourceKind = hotspot_subject`), and Suggested Investigation Steps (generic template text).
- **Test infrastructure update**: `make_hotspot` helper in both `crates/scryrs-curator/src/lib.rs` and `crates/scryrs-cli/src/propose.rs` extended to accept `event_type_failed_lookup: u32` so `counts.eventType["FailedLookup"]` can be populated.
- **Existing test repurposed**: `no_debugging_playbook_proposals` replaced with threshold-positive and threshold-negative tests (generation at >= 2, no generation at 1).
- **Spec update**: `openspec/specs/proposal-generation/spec.md` — replace "No `debugging_playbook` proposals SHALL be generated" and the "debugging_playbook is never generated" scenario with a new requirement and scenario describing the threshold rule, template shape, and anti-spam guard.
- **Doc update**: `.devagent/docs/docs/proposals.md` — replace "Not generated in v1" with "Generated (requires FailedLookup count >= 2)" in the target types table, add the rule to the heuristic rules table, and document the additive overlap behavior with `skill` proposals.
- **No changes** to: `ProposalDocument` contract, publishing path, graph/routes, CLI command surface, LLM-assisted curator path, review CLI.

## Impact

- **Affected crates**: `scryrs-curator` (new heuristic rule + tests), `scryrs-cli` (test helper only — integration tests exercise the new rule through the existing command path).
- **Affected specs**: `proposal-generation` (new requirement and scenario, removal of exclusion clause).
- **Affected docs**: `.devagent/docs/docs/proposals.md` (target-type table and heuristic rules table).
- **No breaking changes**: the new heuristic is purely additive — it generates one additional proposal type for qualifying hotspots. All existing proposals continue to be generated as before.
- **Inbox growth**: hotspots with >= 2 FailedLookup events will now produce both a `debugging_playbook` and a `skill` proposal (since FailedLookup = failure outcome). This additive overlap is documented as intentional v1 behavior.