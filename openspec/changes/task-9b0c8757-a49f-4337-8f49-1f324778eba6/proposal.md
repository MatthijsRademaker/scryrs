## Why

The shipped spec (proposal-generation spec:181-203) requires source-of-truth artifacts to remain untouched during proposal generation, but the current integration test `source_of_truth_not_mutated()` at `crates/scryrs-cli/src/propose.rs:389-423` only verifies `.scryrs/graph.json` and `.scryrs/hotspots.json`. It does **not** verify:

- `.scryrs/routes.json` byte-for-byte non-mutation
- `.devagent/docs/` file-level non-mutation
- Whole-repo file-inventory confinement to `.scryrs/proposals/` only

Additionally, `.scryrs/hotspots.json` is an **input** artifact, not a source-of-truth output named by the spec. Including it in the non-mutation assertion creates a false sense of completeness while masking the real gaps.

This change closes the trust gap between the review-first promise ("proposal generation does not silently mutate source-of-truth") and the executable proof that backs it.

## What Changes

- **Strengthen `source_of_truth_not_mutated()`** in `crates/scryrs-cli/src/propose.rs` to seed `.scryrs/routes.json` and `.devagent/docs/` content before running `write_proposals`, then assert byte-for-byte identity on **all** protected paths: `.scryrs/graph.json`, `.scryrs/routes.json`, and `.devagent/docs/`.
- **Add a whole-repo file-inventory diff** that records all file paths and hashes before and after `write_proposals`, and asserts that any new or modified files are confined to `.scryrs/proposals/**`.
- **Remove `.scryrs/hotspots.json` from the non-mutation assertion** — it is an input artifact (hotspots are the seed that drives proposal generation), not a source-of-truth output named by the spec's "Source-of-truth artifacts are never mutated" requirement.
- **Bug-contingent write-path fix**: If the strengthened test reveals a real production-code write outside `.scryrs/proposals/`, fix **only** the offending write instruction. Do not redesign proposal heuristics, target types, or inbox semantics.

## Impact

- **Affected crate**: `scryrs-cli` — test code only in the `#[cfg(test)]` module of `propose.rs`.
- **No production code changes expected**: `write_proposals` already writes only to `.scryrs/proposals/{id}.json` and creates `.scryrs/proposals/` directory. The test is expected to pass without production changes.
- **No breaking changes**: All existing commands, contracts, and artifact paths remain unchanged.
- **No new dependencies**: Uses existing test infrastructure (`tempfile`, `std::fs`, `std::collections::HashMap`).
- **No spec contract changes**: The canonical `proposal-generation` spec already requires non-mutation of graph, routes, and docs. This change only strengthens the executable verification of that existing contract.