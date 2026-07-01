## 1. Add the publish command surface

- [ ] 1.1 Register `publish` as a root CLI command with nested `markdown` and `rspress` subcommands in `crates/scryrs-cli/src/dispatch.rs`, including attempted-command recognition and usage-error handling.
- [ ] 1.2 Add a dedicated `crates/scryrs-cli/src/publish.rs` executor that parses publish arguments and delegates to the adapter APIs.
- [ ] 1.3 Enable `rspress` in the default `crates/scryrs-cli` feature set so `scryrs publish rspress` ships in the default binary.

## 2. Implement the publish mode contracts

- [ ] 2.1 Implement `scryrs publish markdown <PATH> --output <DIR>` by delegating to `publish_accepted_markdown`, emitting deterministic JSON stdout, and preserving accepted-only/no-stale-delete behavior.
- [ ] 2.2 Implement `scryrs publish rspress <PATH> --docs-root <DIR>` by delegating to `publish_accepted_rspress`, emitting deterministic JSON stdout, and preserving no-partial-write behavior for malformed accepted artifacts or malformed `_nav.json`.
- [ ] 2.3 Enforce the publish exit-code policy: exit `0` on success, exit `2` for usage and publish-input validation failures, exit `1` for runtime/filesystem failures.
- [ ] 2.4 Preserve the explicit review boundary so proposal review commands do not auto-publish.

## 3. Align discovery, doctor, and documentation surfaces

- [ ] 3.1 Update `crates/scryrs-cli/src/help_text.rs`, `crates/scryrs-cli/src/help_json.rs`, and related snapshots so both publish modes appear in human help and `--help-json`, with `surfaceVersion` bumped to `0.14.0`.
- [ ] 3.2 Update `crates/scryrs-cli/src/doctor.rs` so doctor command-surface reporting includes `publish` consistently with the shipped feature set.
- [ ] 3.3 Update `.devagent/docs/docs/cli-v0-contract.md`, `.devagent/docs/docs/proposals.md`, and `.devagent/docs/docs/production-suite.md` so operators can discover the publish commands, their exit-code policy, and the explicit accepted-only/manual-publish boundary.

## 4. Add tests and production verification

- [ ] 4.1 Add CLI tests for publish success paths, missing/unknown subcommands, missing required flags, accepted-only filtering, malformed accepted-artifact failures, and malformed Rspress nav no-partial-write behavior.
- [ ] 4.2 Update `scripts/verify-docs-publish` to exercise the real `scryrs publish markdown` and `scryrs publish rspress` commands against deterministic fixtures before the existing docs build and llms-output assertions.
- [ ] 4.3 Update smoke/dispatch/help-json snapshots and any related verification coverage so both publish modes are exercised through the shipped CLI surface.