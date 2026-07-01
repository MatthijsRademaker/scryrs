## 1. Runtime relevance scoring

- [ ] Update `crates/scryrs-runtime/src/lib.rs` so `explain_hints` computes best match tier, saturating total evidence score, evidence count, and the authoritative sort tuple `(tier DESC, score DESC, count DESC, manifest_index ASC, route_id ASC)`.
- [ ] Populate `RouteHintItem.relevance` for explain matches only using the documented packed `u32` formula and named packing constants.
- [ ] Keep `hints_from_manifest` unchanged so plain manifest projection still omits `relevance`.

## 2. Contracts and documentation

- [ ] Update `crates/scryrs-types/src/lib.rs` wording so `RouteHintItem.relevance` is described as omitted for plain projection and populated for explain matches.
- [ ] Update `crates/scryrs-cli/src/route_explain.rs`, `crates/scryrs-cli/src/help_text.rs`, and `crates/scryrs-cli/src/help_json.rs` to document the ranking chain, packed relevance formula, and the distinction between `rank` and explain `relevance`.
- [ ] Update `.devagent/docs/docs/route-manifests.md` and `.devagent/docs/docs/cli-v0-contract.md` so explain results are no longer described as ordinal-only or always `None` for relevance.

## 3. Verification

- [ ] Add runtime tests for tier priority, evidence-score ordering, evidence-count ordering, stable tie-breaks, saturation/boundary behavior, and unchanged plain-projection relevance omission.
- [ ] Add CLI tests and snapshot updates for populated explain relevance JSON, zero-match success, repeated-run byte identity, and updated help/help-json wording.
- [ ] Update the OpenSpec change deltas for `route-explain` and `route-hint` to match the implementation contract.