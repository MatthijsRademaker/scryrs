## 1. Rename supported Pi install path

- [x] 1.1 Update Pi installer constants and next-step text in `crates/scryrs-cli/src/init.rs` to write `.pi/extensions/scryrs/index.ts` only.
- [x] 1.2 Update Pi hook detection in `crates/scryrs-cli/src/doctor.rs` to check `.pi/extensions/scryrs/index.ts` only, with no legacy `pi-trace` handling.
- [x] 1.3 Update repository ignore and maintainer guidance (`.gitignore`, `AGENTS.md`, `.pi/README.md`) to describe `hooks/pi/index.ts` as canonical source and `.pi/extensions/scryrs/index.ts` as installed runtime copy.

## 2. Align tests and verification with hard-cut path

- [x] 2.1 Update `crates/scryrs-cli/src/init_tests.rs` to assert Pi installation, collisions, and source-repo dogfooding against `.pi/extensions/scryrs/index.ts`.
- [x] 2.2 Update `scripts/verification/installed-hook-e2e.mjs` to install from and load `.pi/extensions/scryrs/index.ts` only.
- [x] 2.3 Update any remaining repository-owned Pi install-path assertions or fixtures to use `scryrs` path only.

## 3. Align user docs and verify end to end

- [x] 3.1 Update `hooks/pi/README.md` to document `.pi/extensions/scryrs/` and `~/.pi/agent/extensions/scryrs/` as supported install locations.
- [x] 3.2 Update any remaining live docs or help text that present `.pi/extensions/pi-trace/` as supported behavior.
- [x] 3.3 Run targeted verification for init and installed-hook flows, then confirm no repository-owned supported-path references to `pi-trace` remain.
