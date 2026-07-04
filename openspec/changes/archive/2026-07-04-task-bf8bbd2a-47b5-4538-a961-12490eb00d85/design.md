## Context

`scryrs propose` generates deterministic `ProposalDocument` candidates from hotspot and graph evidence. The curator engine (`crates/scryrs-curator`) applies concrete heuristics for five target types: `docs_note`, `skill`, `memory_patch`, `adr`, and `semantic_graph_grouping`. `debugging_playbook` is intentionally excluded from v1 generation — the module doc comment states this explicitly, and the test `no_debugging_playbook_proposals` locks in the absence.

Meanwhile, `ProposalTargetType::DebuggingPlaybook` already exists with `ProposedContent::Markdown` validation, and the downstream publishing adapter (`scryrs-adapter-markdown`) already handles accepted `debugging_playbook` decisions. The contract and publishing path are ready; only the generator is missing.

Hotspot entries carry `counts.eventType: HashMap<String, u32>` where `FailedLookup` events aggregate under the key `"FailedLookup"` (per `TraceEvent::payload_type_str()`). The scoring engine gives `FailedLookup` a base weight of 4 plus a failure bonus of 2, so each `FailedLookup` event contributes 6 to score. This means two `FailedLookup` events produce score >= 12 — a clear high-signal pattern.

The deterministic propose path has no access to raw trace events or failure-reason text. Available signals are limited to hotspot aggregates: `counts.eventType`, `counts.outcome`, `score`, `evidence.rowIds`, `subjectKind`, and `subject`. The generated playbook content must be placeholder-heavy and honest about this constraint.

## Goals / Non-Goals

### Goals

- Generate `debugging_playbook` proposals from hotspot entries where `counts.eventType["FailedLookup"] >= 2`.
- Keep the heuristic deterministic, template-based, and grounded in existing hotspot evidence plus cited row IDs.
- Preserve existing `ProposalDocument` validation, review-first storage boundaries (`.scryrs/proposals/` only), and no-mutation guarantees for graph/routes/docs.
- Document the threshold, template shape, and additive overlap behavior in spec and project docs.

### Non-Goals

- No LLM drafting, prompt generation, or new model-assisted `scryrs propose` behavior.
- No automatic acceptance, publishing, graph mutation, or route changes.
- No broad redesign of hotspot scoring or the proposal contract.
- No deduplication policy for overlapping `skill`/`memory_patch`/`debugging_playbook` proposals.
- No configuration surface (the threshold is hard-coded, not environment-variable driven).

## Decisions

### Decision 1: Threshold = `counts.eventType["FailedLookup"] >= 2`

Two of three refinement agents converge on >= 2 (architect and reviewer). The architect provides the strongest rationale: "the minimal defensible bar for 'repeated'" and "score is a secondary signal — two FailedLookup+F events produce score >= 12." The lead dev's threshold of >= 3 is more conservative but the task scenario calls for "repeated failed lookups" — which >= 2 satisfies. A single FailedLookup (count = 1) is explicitly excluded per the "low-signal failures do not spam" scenario.

### Decision 2: No subjectKind filter

The lead dev correctly identifies that `eventType["FailedLookup"]` is already specific enough — a host could theoretically have other subject kinds carrying FailedLookup events. Hard-coding `subjectKind == "symbol"` would over-constrain. The threshold on the event type key alone is sufficient.

### Decision 3: Additive overlap with skill/memory_patch is documented, not deduplicated

A hotspot with >= 2 FailedLookup events also satisfies the `skill` rule (failure outcome >= 1) and likely the `memory_patch` rule (score >= 4, failure-ratio >= 0.5). The result is multiple proposals for the same hotspot subject. All three agents agree this additive behavior is acceptable for v1 — the proposals serve different consumption paths (skill for agent context, playbook for human debugging). The overlap is documented in the spec and project docs. Deduplication is explicitly deferred to a future change.

### Decision 4: Threshold is hard-coded, not configurable

The lead dev raised the question of configurability via environment variable. For v1, the threshold is hard-coded at >= 2. This keeps the change minimal, preserves determinism, and avoids introducing a config surface that itself requires documentation, validation, and testing. Configuration can be added later if tuning evidence demands it.

### Decision 5: Template is placeholder-heavy by design

The deterministic propose path has no access to failure-reason text — only hotspot aggregates. The "Likely Causes" section uses `[TBD]` placeholder bullets, and "Suggested Investigation Steps" uses generic template text. This is deliberate and honest. The product intent is "durable recovery guidance," not diagnostic precision; the placeholder structure invites human review to fill in concrete causes. LLM-assisted curation can add richer content later.

### Decision 6: make_hotspot extended with event_type_failed_lookup parameter

Both test helpers (in `crates/scryrs-curator/src/lib.rs` and `crates/scryrs-cli/src/propose.rs`) currently hardcode `eventType: HashMap::new()`. Without extending the helper, tests for the new heuristic cannot set the triggering signal. A new parameter `event_type_failed_lookup: u32` populates `counts.eventType["FailedLookup"]`. This is a minimal, backward-compatible extension that preserves all existing call sites.

## Risks

| Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- |
| Overlap with skill/memory_patch proposals causes inbox spam | High | Low | Documented as intentional additive behavior. Maintainers can filter by target type. |
| Template quality is low due to limited input signal | Certain | Low | Placeholder sections clearly marked with `[TBD]`. LLM-assisted path can enrich content later. |
| Threshold of 2 may still generate low-value playbooks for transient failures | Low | Medium | The FailedLookup count is a proven signal — FailedLookup events are already weighted and scored by the hotspot engine. The >= 2 bar is conservative enough to exclude single-event noise. |

## Traceability

- **Task**: `bf8bbd2a-47b5-4538-a961-12490eb00d85` — CURATOR PRODUCT 04 — GENERATE DEBUGGING_PLAYBOOK PROPOSALS FROM REPEATED FAILED LOOKUPS v2
- **Dossier**: `2026-07-04T21:41:23.230Z` — confirms `debugging_playbook` already exists in contract/publishing, deterministic generation excludes it today
- **Decision 1-swarm-architect**: threshold >= 2, template with placeholder sections
- **Decision 1-swarm-lead-dev**: no subjectKind filter, template sections enumerated
- **Decision 1-swarm-reviewer**: commit to concrete threshold, document overlap, update make_hotspot
- **Evidence sources**: `crates/scryrs-curator/src/lib.rs`, `crates/scryrs-cli/src/propose.rs`, `crates/scryrs-types/src/lib.rs`, `crates/scryrs-core/src/scoring.rs`, `crates/scryrs-adapter-markdown/src/lib.rs`, `openspec/specs/proposal-generation/spec.md`, `openspec/specs/proposal-contract/spec.md`, `openspec/specs/curator-llm-assist/spec.md`, `.devagent/docs/docs/proposals.md`