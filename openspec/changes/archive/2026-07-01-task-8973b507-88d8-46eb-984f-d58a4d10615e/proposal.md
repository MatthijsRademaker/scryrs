## Why

`scryrs route explain` already filters route-manifest entries deterministically, but it still ranks same-tier matches by manifest order alone and leaves `RouteHintItem.relevance` unset. That makes consumers depend on ID-ordinal ordering even when hotspot/doc evidence strength clearly differs.

This change makes explain results meaningfully ranked without changing the route manifest schema, artifact inputs, zero-match contract, or route identity boundaries.

## What Changes

- Populate `RouteHintItem.relevance` for every matched hint returned by `scryrs route explain`.
- Rank matched hints by the authoritative deterministic tuple `(best_match_tier DESC, total_evidence_score DESC, evidence_count DESC, manifest_index ASC, route_id ASC)`.
- Compute `total_evidence_score` as the saturating sum of `EvidenceLink.score.unwrap_or(0)`; treat absent scores as zero.
- Serialize `relevance` as a documented packed `u32`: `tier * 1_000_000_000 + min(total_evidence_score, 999_999) * 1_000 + min(evidence_count, 999)`.
- Keep the full sort tuple authoritative for ordering; the packed `relevance` value is a display-friendly monotonic derivative, not the sort key.
- Leave `hints_from_manifest` unchanged so plain manifest projection still omits `relevance`.
- Preserve zero-match success, byte-stable output, read-only manifest consumption, and one-hint-per-route identity boundaries.
- Update runtime tests, CLI tests/snapshots, help surfaces, project docs, and OpenSpec wording to reflect the new ranking contract.

## Impact

- Touches runtime ranking logic in `crates/scryrs-runtime` and scoped route-hint wording in `crates/scryrs-types`.
- Updates `scryrs route explain` help text/help-json plus consumer docs to document the formula, tie-break chain, and plain-projection versus explain-match relevance behavior.
- Requires new coverage for evidence-score ordering, evidence-count ordering, packed relevance serialization, stable tie-breaks, zero-match output, repeated-run byte identity, and saturation boundaries.
- Does not add model-based retrieval, merge route IDs, change `.scryrs/routes.json`, or require a schema-version bump.