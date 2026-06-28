## Why

scryrs already has deterministic hotspot, graph, route, and proposal foundations plus an optional `scryrs-llm` transport boundary. This slice should improve prose quality and graph usability without putting ingest, policy, scoring, graph materialization, route generation, or proposal truth behind models.

Foundation 01 therefore needs a tightly bounded, opt-in model-assist layer that only consumes existing evidence, produces reviewable proposal drafts or grouping suggestions, and rejects uncited or over-broad output before it can enter any review flow.

## What Changes

1. **New optional capability**: add a new `curator-llm-assist` capability implemented in a dedicated `scryrs-curator-llm` crate rather than feature-gating model logic inside `scryrs-curator`.
2. **Library-only scope**: Foundation 01 exposes model-assisted drafting and semantic grouping as library APIs only. CLI flags/subcommands, provider credentials, dashboard review UI, and acceptance lifecycle work are explicitly deferred.
3. **Bounded evidence input**: define an `EvidencePack` plus explicit request-budget configuration that accepts only existing hotspot, graph, proposal, and document evidence; enforces bounds before any model call; and disables tool use.
4. **Strict drafting path**: add a drafting API that accepts an existing `ProposalDocument`, preserves its target/content shape, and returns a reviewable draft whose claims are backed only by cited input evidence.
5. **Strict grouping path**: add a grouping API that returns reviewable `semantic_graph_grouping` `ProposalDocument` suggestions with exact `sourceNodeIds`, `targetGroupNodeId`, `targetGroupLabel`, and evidence citations backed by the input evidence pack.
6. **Fail-loud validation and tests**: require structured model responses, exact citation/node validation, fail-the-run rejection semantics for invalid output, fake-`ModelClient` tests, and proof that deterministic default-feature surfaces remain model-free.

## Capabilities

### New Capabilities

- `curator-llm-assist`: optional, bounded model-assisted drafting and semantic grouping over deterministic evidence, implemented outside core deterministic contracts.

### Modified Capabilities

- None. Existing deterministic proposal, graph, hotspot, and route capabilities remain authoritative and unchanged.

## Impact

- **New crate**: `crates/scryrs-curator-llm/` becomes the only model-aware curator layer in this slice.
- **Dependency boundary**: default deterministic builds continue to compile and run without adding a model dependency to `scryrs-curator`, `scryrs-types`, or `scryrs-cli` default features.
- **No invocation-surface change**: this slice adds no `scryrs propose --llm`, no `scryrs propose-llm`, and no provider/configuration UX.
- **No source-of-truth mutation**: model-assisted output remains proposal input only and does not mutate `.scryrs/graph.json`, `.scryrs/routes.json`, hotspot artifacts, docs, ADRs, skills, or memory truth.
- **Follow-up required for acceptance**: any future conversion of accepted semantic groupings into recorded evidence for deterministic graph builds is explicitly out of scope for Foundation 01.
