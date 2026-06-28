## 1. Workspace and dependency boundaries

- [ ] 1.1 Create `crates/scryrs-curator-llm/` as a new workspace member for Foundation 01 model-assist logic.
- [ ] 1.2 Add the crate dependencies needed to consume `scryrs-curator`, `scryrs-types`, and `scryrs-llm` without adding model awareness to `scryrs-curator`, `scryrs-types`, or `scryrs-cli` default features.
- [ ] 1.3 Verify the deterministic curator crate remains model-free: no `scryrs-llm` dependency and no `#[cfg(feature = "llm")]` model paths inside deterministic proposal generation.

## 2. Bounded evidence input and request construction

- [ ] 2.1 Implement `EvidencePack` and its budget configuration for existing hotspot, graph, proposal, and document evidence only.
- [ ] 2.2 Assign stable input-local evidence IDs and preserve exact graph node IDs so later citation and grouping validation can be mechanized.
- [ ] 2.3 Enforce `max_input_chars`, positive `max_output_tokens`, finite `timeout_ms`, caller-visible per-type caps, and `allow_tools = false` before any model call.
- [ ] 2.4 Fail loudly on oversize evidence or invalid request construction; do not silently truncate or expand input.

## 3. Model-assisted drafting and grouping APIs

- [ ] 3.1 Implement `draft_proposal(...)` as a library API that accepts an existing `ProposalDocument` plus `EvidencePack` and returns a reviewable draft with the same `targetType` and target/content shape.
- [ ] 3.2 Implement `suggest_grouping(...)` as a library API that returns reviewable `semantic_graph_grouping` `ProposalDocument` suggestions with exact `sourceNodeIds`, `targetGroupNodeId`, `targetGroupLabel`, title, rationale, and evidence citations.
- [ ] 3.3 Define strict structured response parsing for both APIs using cited `EvidencePack` evidence IDs rather than trusting free-form prose.
- [ ] 3.4 Reject malformed responses, uncited claims, unknown evidence IDs, unknown source node IDs, missing evidence, and target/content mismatches by failing the entire run.

## 4. Verification

- [ ] 4.1 Add fake-`ModelClient` tests that prove bounded request construction from existing evidence.
- [ ] 4.2 Add tests for successful evidence-backed drafting that preserve proposal target/content shape.
- [ ] 4.3 Add tests for successful evidence-backed semantic grouping suggestions that validate exact `sourceNodeIds` and proposal evidence.
- [ ] 4.4 Add tests that reject uncited output, hallucinated source node IDs, missing evidence, malformed structured responses, and partial-success behavior.
- [ ] 4.5 Prove default deterministic builds and tests remain model-free and that Foundation 01 adds no CLI surface or source-of-truth mutation path.
