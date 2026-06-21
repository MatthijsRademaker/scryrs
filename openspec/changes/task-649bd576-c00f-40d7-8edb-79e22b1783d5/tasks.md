## 1. Reconcile hotspot contract semantics in the implementation

- [ ] 1.1 Update `crates/scryrs-cli/src/lib.rs` so `runMetadata.firstEventId` and `runMetadata.lastEventId` are the minimum and maximum SQLite ids among subject-bearing events.
- [ ] 1.2 Update `crates/scryrs-core/src/scoring.rs` so the final deterministic tie-break uses the SQLite id of the chronologically first contributing event in evidence order, not the minimum row id in the subject group.
- [ ] 1.3 Make `.scryrs/hotspots.json` write failures fail the command with exit code 1 and stderr reporting for both populated and empty-report success paths.

## 2. Remove stale placeholder hotspot surfaces

- [ ] 2.1 Update hotspot command help text in `crates/scryrs-cli/src/lib.rs` so it describes a real versioned hotspot report rather than a placeholder.
- [ ] 2.2 Update hotspot help snapshots / `--help-json` expectations so they match the real `HotspotsReport` contract and no longer encode placeholder wording or placeholder-only fields.
- [ ] 2.3 Update `README.md` hotspot sections/examples so they describe SQLite-backed recorded-evidence analysis and real report output.

## 3. Add repeatable CLI coverage for contract edge cases

- [ ] 3.1 Add a CLI test for lifecycle-only stores that exits 0, emits the standard envelope, and returns `entries: []` with zero analyzed subject-bearing events.
- [ ] 3.2 Add deterministic ordering coverage for non-monotonic timestamp/id fixtures, including both corrected final tie-break behavior and evidence row-id ordering.
- [ ] 3.3 Add or update success-path tests to assert `.scryrs/hotspots.json` matches stdout byte-for-byte.
- [ ] 3.4 Add artifact write failure tests that assert non-zero failure behavior and stderr reporting.
- [ ] 3.5 Add coverage that stale hotspot placeholder wording is absent from help/snapshots and related CLI expectations.

## 4. Validate the finished change

- [ ] 4.1 Run the repository's Docker-backed Rust test/check workflow covering `scryrs-cli`, `scryrs-core`, and `scryrs-types`.
- [ ] 4.2 Verify `scryrs hotspots <PATH>` remains standalone over `<PATH>/.scryrs/scryrs.db` with no graph, proposal, adapter, runtime, or LLM dependency introduced.
- [ ] 4.3 Verify missing/unsupported/corrupt store behavior still matches the canonical hotspot-report exit-code contract while the new tests pass repeatably.