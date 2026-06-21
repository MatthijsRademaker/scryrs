## 1. Reconcile stale published documentation

- [ ] 1.1 Update `.devagent/docs/docs/roadmap.mdx` Current Starting Point: remove '(placeholder)' label on `scryrs hotspots <PATH>`; replace 'Phase 2 hotspot materialization and later-suite features are deferred' with accurate status describing the shipped hotspot product; mark Phase 2 section as delivered.
- [ ] 1.2 Rewrite `.devagent/docs/docs/cli-v0-contract.md` hotspots section: replace the entire '(v0 placeholder)' section with the real `HotspotsReport` contract (schemaVersion 1.0.0, runMetadata, entries, artifact file `.scryrs/hotspots.json`, exit codes 0/1/2); update agent-facing hotspot contract and exit-code policy tables; update snapshot testing section to reference real hotspot tests.
- [ ] 1.3 Update `.devagent/docs/docs/architecture.mdx` Current Limitations: remove 'Behavior is scaffold-level: commands print placeholders'; replace with accurate assessment that `scryrs hotspots` is production-level (real SQLite analysis, deterministic scoring, artifact output) while graph, curator, and adapter crates remain scaffold-level.
- [ ] 1.4 Update `.devagent/docs/docs/trace-hook-contract.md` Limitations section: change 'Canonicalization for hotspot grouping is deferred to Phase 2' to 'Command canonicalization remains a known limitation not scheduled for any current roadmap phase.'

## 2. Reconcile stale OpenSpec specs

- [ ] 2.1 Add reconciliation header to `openspec/specs/phase-1-closure/spec.md` superseding Requirement 'Phase 2 behavior remains out of scope' and its scenario 'Closure work does not add Phase 2 behavior', citing hotspot-report/spec.md and hotspot-verification/spec.md as the superseding Phase 2 contract.
- [ ] 2.2 Add reconciliation header to `openspec/specs/cli-foundation-closure/spec.md` superseding Requirement 'Single placeholder command operates correctly' and its scenarios asserting placeholder JSON envelope and 'no backend wiring', citing the live hotspot implementation and hotspot-report/spec.md.
- [ ] 2.3 Add reconciliation header to `openspec/specs/cli-golden-tests/spec.md` superseding Requirement 'hotspots placeholder output is verified by inline snapshot' and its scenario, citing hotspot_e2e.rs and hotspot-verification/spec.md.

## 3. Publish Phase 2 closure reconciliation spec

- [ ] 3.1 Create `openspec/changes/task-56573ced-fdeb-49b2-aea6-41b30f19d2bf/specs/phase-2-closure/spec.md` with: Phase 2 evidence matrix mapping each roadmap deliverable to code/test artifacts; explicit supersedure of conflicting placeholder-era requirements; documentation of accepted limitations (no command canonicalization, no graph/proposal/runtime integration); requirement that stale docs describe real hotspot product boundary.
- [ ] 3.2 Validate the reconciliation spec against the OpenSpec CLI with `openspec validate --strict`.

## 4. Verify change completeness

- [ ] 4.1 Confirm `scripts/test` (or `cargo test --workspace`) passes — no behavioral changes, existing tests serve as verification that the code still matches the documented contract.
- [ ] 4.2 Confirm no production code files have been modified — diff is limited to `.devagent/docs/docs/` and `openspec/specs/`.
- [ ] 4.3 Confirm README.md is not modified (already accurate).
- [ ] 4.4 Confirm no Phase 3+ scope (graph, proposal, adapter, runtime, dashboard, MCP, LLM) has been introduced.