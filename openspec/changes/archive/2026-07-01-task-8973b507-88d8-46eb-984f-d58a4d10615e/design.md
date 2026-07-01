## Context

`scryrs route explain` already consumes only `.scryrs/routes.json`, performs deterministic case-insensitive matching, and returns one hint per matched route. The missing piece is ranking quality: today the command keeps exact/prefix/substring tiers but breaks ties with manifest order alone and leaves `RouteHintItem.relevance` empty. The task requires deterministic, model-free relevance scoring that uses existing manifest evidence while preserving byte-identical repeatability, zero-match success, and route identity boundaries.

## Goals / Non-Goals

**Goals**

- Populate `RouteHintItem.relevance` for every explain match.
- Order explain matches by match quality first, then evidence strength, then stable tie-breaks.
- Keep the implementation model-free and based only on `.scryrs/routes.json`.
- Preserve one-hint-per-route identity boundaries and zero-match exit-0 behavior.
- Document the ranking formula and tie-break chain across CLI help, help-json, project docs, and OpenSpec.
- Prove deterministic output with runtime and CLI tests.

**Non-Goals**

- No semantic retrieval, fuzzy matching, embeddings, LLM ranking, or hidden heuristics.
- No changes to route-manifest generation or `.scryrs/routes.json` schema.
- No population of `relevance` in plain `hints_from_manifest` output.
- No merging or deduplication of distinct route IDs.
- No changes to explain artifact dependencies, mutation behavior, or zero-match contract.

## Decisions

### Decision 1: Use the full deterministic sort tuple as the authoritative ordering rule

Explain matches sort by `(best_match_tier DESC, total_evidence_score DESC, evidence_count DESC, manifest_index ASC, route_id ASC)`.

- `best_match_tier` remains the existing exact/prefix/substring tier across `label`, `subject`, `id`, `target`, `kind`, and `evidenceLinks[].subject`.
- `total_evidence_score` is the saturating sum of `EvidenceLink.score.unwrap_or(0)` across the route's evidence links.
- `evidence_count` is `RouteEntry.evidenceLinks.len()`.
- `manifest_index` preserves stable manifest ordering for equal tuples.
- `route_id` is the final defensive deterministic tie-break.

The full tuple, not the serialized `relevance` number, is the ordering authority.

### Decision 2: Populate `relevance` with a packed deterministic `u32`

Each explain match serializes `relevance` as:

`tier * 1_000_000_000 + min(total_evidence_score, 999_999) * 1_000 + min(evidence_count, 999)`

Implementation should define named constants for the tier multiplier and component caps so the packing bounds are explicit in code and documentation. The packed value is a display-friendly monotonic derivative of the sort tuple; it is not a replacement for tuple-based sorting.

### Decision 3: Keep plain manifest projection unchanged

`hints_from_manifest` stays deterministic and continues to omit `relevance`. Documentation and type comments must distinguish plain projection output from explain-match output so consumers are not told that relevance is always deferred.

### Decision 4: Update every consumer-facing contract surface in lockstep

The change must update:

- `scryrs route explain` help text
- `--help-json` ranking and hint-field descriptions
- `.devagent/docs/docs/route-manifests.md`
- `.devagent/docs/docs/cli-v0-contract.md`
- OpenSpec deltas for `route-explain` and `route-hint`

Those surfaces must document the full tie-break chain, the packed relevance formula, and the distinction between `rank` (manifest ordinal) and explain `relevance`.

### Decision 5: Extend tests around the new ranking dimensions and bounds

Coverage must explicitly verify:

- exact > prefix > substring priority even when lower tiers have stronger evidence
- evidence-score ordering within a tier
- evidence-count ordering when tier and score tie
- stable manifest/route-id tie-breaks
- populated numeric `relevance` in explain JSON
- unchanged zero-match success and empty hints output
- repeated-run byte identity
- saturation/boundary behavior for evidence-score aggregation and packed relevance computation
- unchanged plain `hints_from_manifest` omission of `relevance`

## Risks

- Evidence score aggregation can overflow `u32` if many high-scored links accumulate; use saturating arithmetic and test the boundary.
- Consumers may misread the packed `relevance` number as the full ordering rule; documentation must state that the full sort tuple is authoritative.
- Help/help-json/docs/specs can drift if one surface keeps the old “deferred/ordinal only” wording; update them together.
- Snapshot coverage will fail until help/help-json output is refreshed to the new contract text.

## Traceability

- Task `8973b507-88d8-46eb-984f-d58a4d10615e` defines the feature goal, scenarios, technical notes, and acceptance criteria.
- Dossier `2026-07-01T12:05:10.957Z` defines the affected areas, assumptions, non-goals, and proposal sketch.
- Accepted decisions `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, and `1-swarm-reviewer-recommendation` resolve the tuple-versus-packed-score rule, constant/documentation requirements, test scope, and overflow handling.