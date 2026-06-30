## Context

The repository already has both halves of the foundation this task depends on:

- `crates/scryrs-curator-llm` already exposes bounded, strict `draft_proposal(...)` and `suggest_grouping(...)` APIs over an `EvidencePack`, with fail-whole-run validation for malformed JSON, unknown citations, unknown source node IDs, missing evidence, and content-shape mismatches.
- `scryrs proposals list|accept|reject` already defines the review-loop surface for pending proposals, while the documented naming split keeps `scryrs propose` as deterministic proposal generation.

The missing piece is the opt-in reviewer UX contract. The risk is not the model call itself; it is exposing model assistance in a place that preserves the existing review-first and truth-boundary rules:

- pending proposals remain review-only inbox artifacts;
- accepted decisions alone can later influence graph and publishing flows;
- default deterministic builds and commands remain model-free;
- provider credential UX is still out of scope.

## Goals / Non-Goals

### Goals

- Define a reviewer-facing model-assist surface under the plural `scryrs proposals` review workflow.
- Specify the command shape, feature gate, output semantics, evidence-loading policy, and flags-only configuration boundary.
- Make EvidencePack budgets, per-type caps, and citation rules visible in help and docs.
- Preserve `ProposalDocument` target type and content shape for draft assistance.
- Keep grouping suggestions review-only `semantic_graph_grouping` proposal candidates with exact source node IDs.
- Require fail-whole-run behavior with no partial writes and no protected-path mutation.
- Keep default CLI builds and deterministic paths free of `scryrs-curator-llm`, provider crates, and model invocation.

### Non-Goals

- No `scryrs propose --llm` or any model behavior under deterministic proposal generation.
- No provider credential UX, hosted-provider configuration, or default provider selection.
- No dashboard review UI.
- No automatic accept/reject decisions, auto-publishing, automatic graph mutation, or automatic route mutation.
- No new lifecycle metadata such as `replaces` or `sourceProposalId` on `ProposalDocument`.
- No change to the rule that pending proposals and rejected decisions are ignored by graph build and publishing.

## Decisions

### Decision 1: Place model assistance under `scryrs proposals assist`, not `scryrs propose`

The user-facing UX contract is a grouped `assist` surface nested under the existing plural `proposals` review CLI:

- `scryrs proposals assist draft <PATH> <ID> --model <MODEL_ID> ...`
- `scryrs proposals assist group <PATH> --model <MODEL_ID> ...`

This keeps model behavior inside the review workflow and preserves the documented split between deterministic generation (`propose`) and review operations (`proposals`). The assist surface is optional and feature-gated; it is not part of default CLI behavior.

### Decision 2: Gate the assist surface behind a new non-default `curator-llm` feature

`crates/scryrs-cli` adds a new `curator-llm` Cargo feature that pulls in `scryrs-curator-llm` and is included in `full`, but not in `default`. Default-feature builds must continue to compile and run without `scryrs-curator-llm`, hosted-provider crates, provider configuration, or model invocation.

Help, `--help-json`, and snapshots must reflect that the assist commands are feature-aware: present when built with `curator-llm`, absent otherwise.

### Decision 3: Resolve content-addressed identity by making stdout the default and `--write` explicit

A model-drafted replacement changes `proposedContent`, so it gets a new content-addressed proposal ID. The contract therefore resolves the identity problem as follows:

- `draft` emits the validated replacement `ProposalDocument` JSON to stdout by default.
- `--write` is required before any file is created.
- `--write` creates a new `.scryrs/proposals/{newId}.json` inbox artifact.
- The source `.scryrs/proposals/{sourceId}.json` file is never mutated or overwritten.
- The contract does not add `replaces` or `sourceProposalId` fields to `ProposalDocument`; the UX instead documents that rewritten drafts are new pending proposals.

The same stdout-default / `--write`-explicit rule applies to grouping suggestions.

### Decision 4: Use explicit evidence-scope flags for draft assistance and deterministic graph/hotspot loading for grouping

The draft assist command builds its `EvidencePack` from the source proposal evidence by default. Broader deterministic evidence is opt-in through explicit flags:

- `--include-hotspots`
- `--include-graph`

Any included evidence is still bounded by `EvidencePackConfig` caps.

The grouping assist command is purpose-built for semantic grouping, so it loads deterministic graph and hotspot evidence from the repository path and subjects that evidence to the same bounded-pack limits.

This resolves the refinement conflict in favor of explicit scope expansion for drafts while keeping grouping tied to the graph/hotspot artifacts it requires.

### Decision 5: Use flags only for non-secret request and EvidencePack configuration

The contract exposes model ID and bounded-request controls through command flags only. The user-visible controls include:

- `--model <MODEL_ID>` (required)
- `--max-input-chars <N>`
- `--max-hotspots <N>`
- `--max-graph-nodes <N>`
- `--max-proposals <N>`
- `--max-documents <N>`
- `--max-output-tokens <N>`
- `--timeout-ms <N>`

The default help and docs must surface the current default caps from `EvidencePackConfig::default()` (`32000`, `50`, `100`, `20`, `20`) and the citation rule that all returned evidence must resolve to cited entries from the input pack. This change does not add config-file keys, env-var controls, or provider secrets UX.

### Decision 6: Preserve fail-whole-run validation and no-partial-write behavior

Assist commands inherit the strict library validation contract and extend it to CLI writes:

- malformed structured output,
- unknown citations,
- unknown source node IDs,
- empty evidence,
- target/content-shape mismatch,
- over-budget input or output,
- missing required deterministic inputs,
- or unavailable provider integration

must fail the entire assist run before any inbox artifact is written. The contract forbids partial success sets, skipped invalid candidates, synthetic evidence, or best-effort writes.

### Decision 7: Keep assist output review-only and non-authoritative

Assist output is always a proposal candidate, never accepted truth. The commands must not create, modify, or delete:

- `.scryrs/accepted/`
- `.scryrs/rejected/`
- `.scryrs/graph.json`
- `.scryrs/routes.json`
- `.devagent/docs/`
- memory-truth surfaces

Grouping suggestions remain pending `semantic_graph_grouping` proposals until a separate explicit acceptance step occurs.

### Decision 8: Document the provider gap instead of inventing provider UX

The repository exposes only the provider-neutral `ModelClient` boundary today. This change therefore defines the request/response UX contract but does not add provider credential loading, hosted-provider setup, or default provider selection. The design must document that gap explicitly and require the command to fail loudly rather than fabricate a half-working provider path.

### Decision 9: Update docs and machine-readable help as part of the contract

The implementation brief includes documentation and machine-surface work, not just code paths. The final contract must update:

- human-readable CLI help,
- feature-aware `--help-json`,
- committed command/help snapshots,
- `.devagent/docs/docs/proposals.md`,
- `.devagent/docs/docs/cli-v0-contract.md`,
- `.devagent/docs/docs/production-suite.md`

so that reviewers are told the assist surface is optional, bounded, and non-authoritative.

## Risks

- **Provider integration is still absent**: the UX can be fully specified now, but it must not smuggle in credential or hosted-provider work that the repository does not support yet.
- **Rewritten drafts have no schema-level link to their source proposal**: the contract handles this by making stdout the default and by keeping any persisted draft as a new inbox artifact rather than mutating schema or source files.
- **Feature-aware help/help-json can drift**: the machine-readable surface must conditionally describe `assist` when the feature is enabled and omit it otherwise.
- **Evidence scope can sprawl if defaults are vague**: the contract resolves this by keeping draft evidence narrow by default and making broader evidence inclusion explicit.

## Conflict Resolution

- **Command placement**: refinement consistently rejected `scryrs propose --llm`; accepted decisions converged on `scryrs proposals assist`. This spec adopts the plural review surface.
- **Draft output semantics**: refinement flagged content-addressed ID churn as a blocker. Accepted architect, lead-dev, and reviewer evidence all supported stdout-first output with explicit persistence. This spec resolves the gap as stdout by default plus `--write` for new inbox files.
- **Evidence loading policy**: architect evidence favored proposal-plus-graph/hotspot context, while lead-dev evidence demanded explicit scope instead of implicit loading. This spec resolves the tension by defaulting draft assist to source-proposal evidence and using explicit `--include-hotspots` / `--include-graph` flags for broader deterministic evidence, while keeping grouping tied to graph and hotspot inputs.
- **Configuration channel**: reviewer evidence required a specific choice. Accepted architect evidence pointed to help-visible flags. This spec resolves the channel as flags only, with no config-file or env-var layer.
- **Provider gap**: lead-dev evidence identified missing provider implementation as real. This spec preserves that boundary by specifying the UX contract and requiring loud failure/documentation rather than inventing provider setup.

## Traceability

- Task: `013acdb6-bea4-4207-9e33-ceac3dc063f0`
- Dossier: `2026-06-30T05:19:41.929Z`
- Accepted decisions: `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, `1-swarm-reviewer-recommendation`
- Round evidence: `round:1:agent:swarm-architect`, `round:1:agent:swarm-lead-dev`, `round:1:agent:swarm-reviewer`
- Interpreted source boundaries: `openspec/specs/curator-llm-assist/spec.md`, `openspec/specs/proposal-review-cli/spec.md`, `openspec/specs/proposal-contract/spec.md`, `openspec/specs/graph-build/spec.md`, `openspec/specs/markdown-publishing-adapter/spec.md`, `.devagent/docs/docs/proposals.md`, `.devagent/docs/docs/cli-v0-contract.md`, `.devagent/docs/docs/production-suite.md`, `crates/scryrs-curator-llm/src/lib.rs`, `crates/scryrs-llm/src/lib.rs`, `crates/scryrs-cli/Cargo.toml`, `crates/scryrs-cli/src/proposals.rs`, `crates/scryrs-cli/src/help_json.rs`