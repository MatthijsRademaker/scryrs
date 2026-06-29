## 1. Add the `scryrs doctor` CLI command

- [x] 1.1 Extend the CLI root command surface, help output, and machine-readable help surface to include `scryrs doctor`.
- [x] 1.2 Implement doctor findings for binary version, shipped command surface / feature availability, resolved `local` vs `live` mode, local store status, Claude Code hook status, Pi hook status, live server reachability when live mode is configured, and docs links.
- [x] 1.3 Support human-readable output by default and `--json` output for automation using the same diagnostic categories.
- [x] 1.4 Enforce the `ok` / `warn` / `error` severity model with exit `0` for OK+WARN-only results and exit `2` when any structural error is present.
- [x] 1.5 Add coverage for at least: no config/local empty store, local initialized store, live config with unreachable server, missing remote identity, Claude Code hook present, Pi hook present, and unsupported or corrupt config reporting.
- [x] 1.6 Update the CLI reference documentation for the new public command contract.

## 2. Add the authoritative production verification gate

- [x] 2.1 Create `scripts/verify-production-suite` as the single headless production-readiness entrypoint with clear per-lane headers and non-zero failure behavior.
- [x] 2.2 Add `scripts/precommit-run --production` as the explicit heavy-lane wrapper without changing the default PR-gate posture.
- [x] 2.3 Add a deterministic core-artifact-loop verification lane that runs `record -> hotspots -> graph -> route -> propose -> proposals accept` through the real `scryrs` binary and asserts the expected deterministic artifacts are produced.
- [x] 2.4 Compose the production suite from the existing lanes plus the new core-artifact-loop lane: `scripts/check`, `scripts/test --full`, `scripts/security`, `scripts/verify-install`, `scripts/verify-trace-capture`, `scripts/verify-live-hotspots`, and `scripts/verify-docs-publish`.
- [x] 2.5 Add a runnable privacy assertion lane that verifies telemetry/privacy defaults programmatically and wire it into the production suite.
- [x] 2.6 Keep live dashboard verification out of the automated production gate and preserve it as a documented manual smoke path.

## 3. Publish the production-hardening operator docs

- [x] 3.1 Update `.devagent/docs/docs/production-suite.md` to link directly to `scryrs doctor` and `scripts/verify-production-suite`.
- [x] 3.2 Update `scripts/verification/README.md` to document the production-suite lanes, prerequisites, expected runtime/posture, failure interpretation, live dashboard manual boundary, and Linux-vs-macOS verification posture.
- [x] 3.3 Document the exact macOS manual verification commands and the explicit limitation that current Linux Docker automation does not prove Darwin behavior.
- [x] 3.4 Update any related user-facing docs needed to keep install, release, and diagnostic guidance consistent with the new operator path.
