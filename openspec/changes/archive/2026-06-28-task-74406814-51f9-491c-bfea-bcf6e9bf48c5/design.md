## Context

scryrs already separates deterministic evidence pipelines from the optional model boundary:

- `scryrs-curator` deterministically generates reviewable `ProposalDocument` candidates from hotspot and graph evidence.
- `scryrs-llm` already defines a provider-neutral `ModelClient`, bounded `ModelRequest`, and request validation.
- Existing proposal and graph contracts already require review-only proposal artifacts, explicit evidence citations, exact semantic-grouping source node IDs, and non-mutation of graph/route source-of-truth artifacts.

The refinement evidence is consistent on the architectural direction: model assistance must remain opt-in, bounded, evidence-backed, and outside ingest, policy, scoring, graph build, route generation, and base proposal validation.

## Goals / Non-Goals

**Goals**

- Add an optional model-assisted curator layer for drafting/summarization over existing evidence only.
- Add an optional model-assisted semantic grouping suggestion path that emits reviewable `semantic_graph_grouping` proposals.
- Keep deterministic curator behavior and all existing deterministic contracts usable with default features and no model dependency.
- Make input/output bounds explicit and testable.
- Reject uncited, malformed, or over-broad model output loudly before any review artifact is accepted.

**Non-Goals**

- No model calls in ingest, policy, hotspot scoring, graph materialization, route generation, or `ProposalDocument::validate()`.
- No silent mutation of `.scryrs/graph.json`, `.scryrs/routes.json`, `.scryrs/hotspots.json`, docs, ADRs, skills, or memory files.
- No CLI flag/subcommand, provider credential/configuration UX, or hosted-provider integration in this slice.
- No auto-acceptance, auto-publishing, dashboard review UI, or graph-build consumption of proposed groupings.
- No replacement of deterministic `scryrs-curator::generate_proposals()` heuristics with model scoring or clustering.

## Decisions

### Decision 1: Use a separate `scryrs-curator-llm` crate

Foundation 01 will use a dedicated `scryrs-curator-llm` crate. Model-aware code does not live inside `scryrs-curator`, even behind `#[cfg(feature = "llm")]`.

This keeps the deterministic curator crate provably model-free, preserves the existing architecture boundary between deterministic logic and model transport, and matches the accepted direction from refinement.

### Decision 2: Scope Foundation 01 to library APIs only

This slice provides library APIs only:

- `draft_proposal(...)`
- `suggest_grouping(...)`

No CLI wiring is included. `scryrs propose --llm`, `scryrs propose-llm`, provider credential loading, and end-user invocation UX are follow-up work.

### Decision 3: Define `EvidencePack` as the bounded input boundary

The model-assist crate will define an `EvidencePack` plus explicit budget configuration.

`EvidencePack` only admits existing deterministic inputs:

- hotspot entries
- graph nodes
- proposal documents
- document evidence already representable through existing evidence links

`EvidencePack` must:

- assign stable input-local IDs to evidence entries used for citation lookup
- expose exact graph node IDs for grouping validation
- enforce `max_input_chars` and caller-visible per-type caps before constructing any `ModelRequest`
- fail loudly on oversize input instead of truncating

### Decision 4: Use strict structured model responses with input-local citation IDs

The model response format must be machine-validated rather than free-form prose.

Drafting responses must resolve to one structured draft object containing the updated proposal fields plus cited evidence IDs.

Grouping responses must resolve to a structured list of grouping suggestions containing:

- `title`
- `rationale`
- `sourceNodeIds`
- `targetGroupNodeId`
- `targetGroupLabel`
- cited evidence IDs

The parser reconstructs proposal evidence from the cited `EvidencePack` entries. Any unknown citation ID or any unknown source node ID is an error.

### Decision 5: Preserve proposal contract shape during drafting

`draft_proposal(...)` accepts an existing `ProposalDocument` and returns a reviewable `ProposalDocument` with the same `targetType` and target/content shape. Model assistance can improve draft prose, but it cannot switch the proposal to a different target type, invent a new structured payload shape, or replace deterministic source data.

### Decision 6: Fail the entire model-assisted run on invalid output

Malformed JSON, uncited claims, missing evidence, hallucinated source node IDs, or target/content mismatches fail the entire drafting or grouping run.

Foundation 01 does not return partial success sets, does not emit diagnostic proposal artifacts, and does not silently skip bad candidates.

### Decision 7: Accepted semantic groupings remain future reviewed evidence

This slice stops at reviewable proposals. It does not define how an accepted grouping becomes `recorded_evidence`, and it does not let graph build or route generation consume model proposals directly.

## Conflict Resolution

The reviewer identified four gaps that needed resolution before implementation. This specification resolves them as follows:

1. **Concrete bounded input type**: resolved by requiring `EvidencePack` plus explicit budget configuration and pre-request bound enforcement.
2. **Crate placement**: resolved in favor of a new `scryrs-curator-llm` crate, not feature-gated code inside `scryrs-curator`.
3. **Structured response and citation protocol**: resolved by requiring strict structured responses that cite `EvidencePack` evidence IDs and exact source node IDs.
4. **Invalid-output behavior**: resolved as fail-the-entire-run with no partial results and no artifact writes in this library-only slice.

## Risks

| Risk | Mitigation |
| --- | --- |
| Input can grow beyond useful prompt size if evidence is packed ad hoc. | `EvidencePack` enforces bounds before request construction and fails loudly instead of truncating. |
| Structural `ProposalDocument` validation alone does not prove evidence citations came from the input. | Add model-assist validation that cross-references every cited evidence ID and source node against the input `EvidencePack`. |
| Model-aware code could leak into deterministic crates through convenience feature gates. | Keep all model-assist code in `scryrs-curator-llm` and leave deterministic crates unchanged. |
| Readers may assume grouping suggestions automatically change the graph. | Keep proposals review-only and explicitly defer accepted-grouping lifecycle and graph-build consumption. |

## Traceability

- **Task input**: bounded drafting/summarization/grouping over deterministic evidence; no model role in ingest, policy, scoring, or base graph truth.
- **Dossier `2026-06-28T07:42:13.560Z`**: goals, non-goals, affected areas, acceptance criteria, and follow-up gaps.
- **Decision `1-swarm-architect-recommendation`**: separate `scryrs-curator-llm` crate, `EvidencePack`, pre-request bounds, strict citation/node validation, library-only scope.
- **Decision `1-swarm-lead-dev-recommendation`**: `draft_proposal(...)`, `suggest_grouping(...)`, fake-client tests, fail-loud invalid-output handling, no default-feature dependency leak.
- **Decision `1-swarm-reviewer-recommendation`**: reviewer gaps were resolved here without expanding scope beyond the accepted foundation slice.
