## 1. Gate Bash capture in reference hooks

- [x] 1.1 Update `hooks/pi/index.ts` so `bash` emits `CommandExecuted` only when `SCRYRS_DEBUG` is set to a non-empty value.
- [x] 1.2 Update `hooks/claude-code/scryrs-hook.mjs` so Bash trace capture is skipped by default and re-enabled only when `SCRYRS_DEBUG` is set to a non-empty value.
- [x] 1.3 Keep non-Bash tool mappings and hook fail-open/non-interference behavior unchanged while adding Bash gating.

## 2. Align docs and manifest with observer-first boundary

- [x] 2.1 Update `hooks/pi/README.md` to describe default tracked tools without Bash and debug-gated Bash capture via `SCRYRS_DEBUG`.
- [x] 2.2 Update `hooks/claude-code/README.md` to describe default tool coverage without Bash and debug-gated Bash capture via `SCRYRS_DEBUG`.
- [x] 2.3 Update `.devagent/docs/docs/trace-hook-contract.md` to document observer-first default scope and debug-gated Bash observed-command semantics.
- [x] 2.4 Update `scryrs.json` so harness metadata reflects default observer-first tools plus explicit debug-only Bash/`CommandExecuted` support.

## 3. Refresh verification coverage

- [x] 3.1 Update Claude Code verification fixtures to assert that Bash is not persisted when `SCRYRS_DEBUG` is unset.
- [x] 3.2 Update Pi verification fixtures to assert that Bash is not persisted when `SCRYRS_DEBUG` is unset.
- [x] 3.3 Add debug-mode verification in both harness fixtures proving Bash still persists as `CommandExecuted` when `SCRYRS_DEBUG` is set.

## 4. Add roadmap note for future rewrite work

- [x] 4.1 Update `.devagent/docs/docs/roadmap.mdx` to reinforce observer-first hotspot detection as current product boundary.
- [x] 4.2 Add thin roadmap entry describing RTK-style rewrite/optimizer behavior as later-phase optional work, not current implementation.

## 5. Verify change end-to-end

- [x] 5.1 Run targeted verification for changed hook paths and docs consistency checks.
- [x] 5.2 Run `scripts/verify-trace-capture` (or targeted harness variants as needed) and confirm default mode plus debug mode expectations both pass.
- [x] 5.3 Review final diff to ensure no Rust schema, scoring, or CLI behavior changed outside intended observer-first boundary.
