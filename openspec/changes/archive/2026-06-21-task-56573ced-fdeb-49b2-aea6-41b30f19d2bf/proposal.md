## Why

Phase 2 hotspot materialization is fully implemented in code and tests — `scryrs hotspots <PATH>` opens SQLite via `TraceQuery`, runs deterministic `score_hotspots` with a documented weight table and six-key tie-break, emits a versioned `HotspotsReport` (schemaVersion 1.0.0) to stdout and `.scryrs/hotspots.json`, and handles empty/missing/corrupt stores with correct exit codes. The E2E test in `hotspot_e2e.rs` proves the full public pipeline. The README already reflects the real three-command CLI.

However, the repo still contains multiple stale source-of-truth artifacts that describe `scryrs hotspots <PATH>` as a placeholder or say Phase 2 is still deferred. This is a source-of-truth reconciliation problem, not a code gap. Leaving these artifacts unresolved makes the project look unfinished and misleads future contributors.

## What Changes

### Published documentation updates

- **roadmap.mdx**: Remove '(placeholder)' label on `scryrs hotspots <PATH>`, remove 'Phase 2 hotspot materialization and later-suite features are deferred' wording from Current Starting Point, mark Phase 2 deliverables as delivered.
- **cli-v0-contract.md**: Replace the entire 'scryrs hotspots \<PATH\> (v0 placeholder)' section with the real hotspot contract matching the live `--help` and `--help-json` output — `HotspotsReport` schema (schemaVersion 1.0.0, runMetadata, entries, artifact file), exit codes 0/1/2, and the `.scryrs/hotspots.json` artifact.
- **architecture.mdx**: Update Current Limitations to reflect that hotspots is production-level with real SQLite analysis, scoring, and artifact output. Retain scaffold caveat for genuinely placeholder crates (graph, curator, adapters).
- **trace-hook-contract.md**: Update 'Canonicalization for hotspot grouping is deferred to Phase 2' to state that command canonicalization remains a known limitation not scheduled for any current roadmap phase.

### OpenSpec contract reconciliation

- **phase-1-closure/spec.md**: Add a reconciliation header superseding the 'Phase 2 behavior remains out of scope' requirement and its placeholder-envelope scenario, referencing hotspot-report/spec.md and hotspot-verification/spec.md as the canonical Phase 2 contract.
- **cli-foundation-closure/spec.md**: Add a reconciliation header superseding the 'Single placeholder command operates correctly' requirement and its placeholder-JSON / no-backend-wiring scenarios.
- **cli-golden-tests/spec.md**: Add a reconciliation header superseding the 'hotspots placeholder output is verified by inline snapshot' requirement.

### Phase 2 closure evidence matrix

Publish a reconciliation spec (`specs/phase-2-closure/spec.md`) that:
- Maps each roadmap Phase 2 deliverable (real hotspots, deterministic aggregation, stable JSON contract, `.scryrs/hotspots.json`) to concrete code paths and test evidence.
- Documents accepted limitations (no command-subject canonicalization, no graph/proposal/runtime integration).
- Supersedes conflicting requirements in the three stale specs.

### Non-changes

- No production code modifications — the live CLI, scoring engine, and E2E tests are correct.
- No Phase 3+ scope (graph, proposal, adapter, runtime, dashboard, MCP, LLM).
- No changes to README.md (already accurate).
- No changes to hotspot-report/spec.md or hotspot-verification/spec.md (already canonical).

## Impact

- **Affected code**: None (documentation and OpenSpec artifact changes only).
- **Affected specs**: phase-1-closure/spec.md, cli-foundation-closure/spec.md, cli-golden-tests/spec.md (reconciliation headers added).
- **Affected docs**: roadmap.mdx, cli-v0-contract.md, architecture.mdx, trace-hook-contract.md (stale wording corrected).
- **New artifacts**: specs/phase-2-closure/spec.md (reconciliation and evidence matrix spec).
- **Risk profile**: Low — no behavioral changes, no regressions. The change is purely source-of-truth reconciliation.
- **Verification**: Existing `scripts/test` and `cargo test --workspace` must continue to pass. No new test surface needed.