## Why

The installed Pi extension artifact still uses `pi-trace`, which is stale product naming and leaks an implementation-era label into user-facing install paths, tests, and maintainer docs. Rename it now while the surface is still alpha so the canonical installed path matches `scryrs` and we avoid carrying legacy naming forward.

## What Changes

- **BREAKING** Rename installed Pi extension directory from `.pi/extensions/pi-trace/` to `.pi/extensions/scryrs/`.
- **BREAKING** Rename documented user-global Pi extension directory from `~/.pi/agent/extensions/pi-trace/` to `~/.pi/agent/extensions/scryrs/`.
- Update `scryrs init --agent pi` to write only `.pi/extensions/scryrs/index.ts`.
- Update Pi hook verification, maintainer guidance, ignore rules, and user docs to use `scryrs` path consistently.
- Keep harness identifiers and command routing unchanged: `--agent pi` and `scryrs hook pi` remain as-is.
- Do not add compatibility shims, dual-path detection, or legacy `pi-trace` migration logic in installer or doctor.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `init-installer`: Pi installed-hook target path changes from `.pi/extensions/pi-trace/` to `.pi/extensions/scryrs/`, including source-repo dogfooding path and maintainer guidance about canonical vs installed copies.
- `init-verification`: Installed-hook verification changes to assert the Pi artifact under `.pi/extensions/scryrs/` and to load artifacts from that path only.

## Impact

- Affected code: `crates/scryrs-cli/src/init.rs`, `crates/scryrs-cli/src/doctor.rs`, `crates/scryrs-cli/src/init_tests.rs`, `scripts/verification/installed-hook-e2e.mjs`, `hooks/pi/README.md`, `AGENTS.md`, `.pi/README.md`, `.gitignore`.
- Affected specs/docs: `openspec/specs/init-installer/spec.md`, `openspec/specs/init-verification/spec.md`, plus related repository guidance.
- User impact: existing `pi-trace` installs become stale and unsupported; users must rerun `scryrs init --agent pi` to get `.pi/extensions/scryrs/index.ts`.
- No API or CLI harness-name changes: `pi` and `claude-code` remain the only supported harness identifiers.
