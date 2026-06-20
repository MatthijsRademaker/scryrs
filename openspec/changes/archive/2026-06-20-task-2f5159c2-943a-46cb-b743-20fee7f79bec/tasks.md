## 1. Sync the Phase 1 status surfaces

- [x] 1.1 Update `.devagent/docs/docs/roadmap.mdx` so the current-state and Phase 1 sections describe the implemented `record`, persistence, manifest, hooks, installer, and verification boundary instead of calling them forthcoming.
- [x] 1.2 Update `.devagent/docs/docs/trace-hook-contract.md` so it no longer describes Pi/Claude hook coverage or the checked-in manifest as planned/forthcoming, while keeping the accepted Claude and Pi lifecycle limitations explicit.
- [x] 1.3 Update `README.md` help/help-json/output examples to match the live CLI surface: three commands, `init --agent`, `surfaceVersion` `0.3.0`, and current exit-code semantics.
- [x] 1.4 Update `scryrs.json` `record.outputContract` text so it matches the actual CLI summary and rejection-diagnostic fields.
- [x] 1.5 Keep the Phase 1 closure matrix for this change consistent with the final docs and code state.

## 2. Harden `record` ingestion without expanding scope

- [x] 2.1 Add post-deserialization validation in `crates/scryrs-core/src/ingestion.rs` for `schema_version == SCHEMA_VERSION`.
- [x] 2.2 Add post-deserialization validation that `event_type` matches the concrete `payload.type` discriminator.
- [x] 2.3 Preserve the current ingestion-only boundary: rejected lines emit deterministic diagnostics, later lines continue, accepted lines still persist, and no hotspot/analysis behavior is added.
- [x] 2.4 Audit and update CLI/core tests and any verification fixtures so all accepted fixture events satisfy the new invariants, and add explicit rejection coverage for version and type mismatches.

## 3. Close the remaining installer and Pi hook trust gaps

- [x] 3.1 Fix the `.claude/settings.json` collision message in `crates/scryrs-cli/src/init.rs` so it does not claim the hook source was installed before the write path runs.
- [x] 3.2 Add or update installer tests to confirm the collision branch still exits 2, does not mutate files, and emits the corrected guidance.
- [x] 3.3 Update `hooks/pi/index.ts` so resolved non-zero `pi.exec('scryrs', ['record', '--stdin'], ...)` results are logged as fail-open trace failures in addition to thrown errors.
- [x] 3.4 Extend Pi hook verification/tests to cover both missing-binary and resolved non-zero exit-code failures while keeping handler return values and tool-result non-interference unchanged.

## 4. Verify closure with the existing Docker-backed workflow

- [x] 4.1 Run Docker-backed `scripts/check`.
- [x] 4.2 Run Docker-backed `scripts/test`.
- [x] 4.3 Run Docker-backed `scripts/verify-trace-capture`.
- [x] 4.4 If any validation command fails, fix the implementation or record the failure as an explicit blocker before declaring Phase 1 closed.
