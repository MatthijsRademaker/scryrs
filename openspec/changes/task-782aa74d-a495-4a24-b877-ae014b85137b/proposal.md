## Why

The repository already has independent pieces of the production suite: local trace capture and hotspot scoring, live server ingest and queries, graph and route generation, proposal review, accepted-evidence publishing, Docker-backed verification scripts, install scripts, and project docs. What it does not have is one installed-user diagnostic path and one authoritative production-readiness gate that compose those pieces into a repeatable release workflow.

This change closes that gap for Production Hardening 01. It gives maintainers and installed users a native `scryrs doctor` command that reports the shipped binary surface, resolved local-vs-live mode, store and hook status, live-server reachability, and docs links. It also adds a single production verification entrypoint that proves the deterministic core artifact loop, the live server loop, packaging/install posture, docs publishing, and privacy/security boundaries without duplicating existing verification logic.

## What Changes

1. **Add `scryrs doctor` as a native CLI command**
   - Extend the CLI root command surface to include `doctor`.
   - Support human-readable output by default and `--json` for automation.
   - Report binary version, available command surface, resolved mode (`local` or `live`), local store status, Claude Code and Pi hook status where applicable, live server reachability when live mode is configured, and links to the relevant docs.
   - Use a typed findings model with `ok`, `warn`, and `error` severities and a documented exit-code contract.

2. **Add one authoritative production verification gate**
   - Create `scripts/verify-production-suite` as the single headless release-verification entrypoint.
   - Compose the existing Docker-backed lanes instead of reimplementing them: `scripts/check`, `scripts/test --full`, `scripts/security`, `scripts/verify-install`, `scripts/verify-trace-capture`, `scripts/verify-live-hotspots`, and `scripts/verify-docs-publish`.
   - Add the missing deterministic core-artifact-loop verification (`record -> hotspots -> graph -> route -> propose -> proposals accept`) and include it in the production suite.
   - Add a runnable privacy assertion lane that verifies telemetry/privacy defaults through compiled tests rather than source-text inspection.
   - Expose the heavy lane through `scripts/precommit-run --production` without making it the default PR gate.

3. **Document the operator and release workflow clearly**
   - Update the CLI reference for `scryrs doctor`, including output categories, severity policy, and test coverage expectations.
   - Update the Production Suite Plan and verification docs to link directly to `scryrs doctor` and `scripts/verify-production-suite`, explain lane prerequisites/runtime/failure interpretation, and state the current posture.
   - Keep live dashboard verification as an explicit documented manual smoke boundary for this change.
   - Keep Linux install verification automated through the existing installer verification path and document the exact macOS manual verification path plus its current CI limitation.

## Impact

- Installed users get a first-class diagnostic command instead of piecing together state from separate commands and files.
- Release maintainers get one authoritative production gate with a single exit status and explicit lane boundaries.
- The missing end-to-end proof for the deterministic core artifact loop becomes part of release hardening.
- Privacy/security checks become explicit release work rather than an implicit assumption.
- The change stays within current product boundaries: no hosted deployment work, no auth/TLS scope, no dashboard automation expansion, and no LLM-owned truth paths.