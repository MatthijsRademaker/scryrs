## Why

`scryrs init --agent <NAME>` now defaults to live mode, but first-run users who have no `scryrs.json` or environment overrides only get a fail-fast missing-config error. That preserves automation safety, but it makes the default path feel broken for interactive humans because the CLI already knows exactly which live bootstrap values are missing.

## What Changes

- Add an interactive live-init wizard for `scryrs init` when required live fields are missing and the command is running in an interactive terminal.
- Add `--no-interactive` to preserve explicit fail-fast behavior and guarantee no prompts for automation, scripts, CI, or users who prefer flags/env only.
- Keep non-interactive invocations safe: when stdin/stdout are not terminals, missing live fields still exit 2 with deterministic guidance instead of blocking.
- Provide a richer wizard UX for live bootstrap collection, including labels, defaults from existing resolution layers, validation, confirmation, and clear commit-vs-local wording.
- Preserve existing config precedence and write behavior: flags/env/.scryrs overrides/existing `scryrs.json` remain inputs; successful live init still writes only `remote.ingest_url`, `remote.workspace_id`, and `remote.docker_network` to committed `scryrs.json`.
- Update help/help-json, README/CLI docs, and tests for interactive and non-interactive behavior.

## Capabilities

### New Capabilities

- `init-interactive-wizard`: Interactive live-init wizard behavior, including prompt conditions, non-interactive opt-out, field validation, and confirmation semantics.

### Modified Capabilities

- `init-installer`: `scryrs init` live-mode missing-config handling changes from promptless fail-fast in all cases to TTY-only wizard unless `--no-interactive` is supplied.
- `cli-discovery-ux`: Help and machine-readable CLI discovery need to describe `--no-interactive` and wizard behavior.
- `workspace-live-bootstrap`: Live bootstrap setup becomes wizard-assisted while preserving the same committed manifest and managed `.scryrs/` scaffold contracts.

## Impact

- **CLI surface**: `scryrs init` gains `--no-interactive`; optional `--interactive` is intentionally not required for the default human path.
- **Runtime behavior**: interactive terminals can collect missing live config; non-interactive contexts keep deterministic exit-2 behavior.
- **Implementation scope**: `crates/scryrs-cli/src/init.rs`, dispatch/help/help-json surfaces, init tests, and documentation.
- **Dependencies**: likely add a terminal prompt crate for rich UX, or implement a minimal internal prompt layer if dependency cost is rejected during design.
- **Artifacts**: no changes to hook source, trace schema, record transport, server endpoints, dashboard behavior, or local-mode manifest behavior.
