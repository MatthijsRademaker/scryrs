## 1. Relax source-repo init policy for Pi

- [x] 1.1 Update `crates/scryrs-cli/src/init.rs` so source-checkout detection returns enough information to distinguish source-root path resolution from blanket refusal.
- [x] 1.2 Allow `scryrs init --agent pi` inside scryrs source checkout while keeping `scryrs init --agent claude-code` refusal unchanged.
- [x] 1.3 Resolve source-repo Pi installs to repository root so subdirectory invocations do not create nested `.pi/extensions/pi-trace/` trees.

## 2. Preserve canonical-source boundaries

- [x] 2.1 Add repository guidance in `AGENTS.md` stating `hooks/pi/index.ts` is canonical source and `.pi/extensions/pi-trace/index.ts` is installed runtime copy only.
- [x] 2.2 Add explicit guidance that LLMs/agents must never edit `.pi/extensions/pi-trace/index.ts` directly and must treat it as non-leading, non-canonical artifact.
- [x] 2.3 Update ignore/documentation files so local root-repo Pi installs do not create noisy working-tree artifacts.

## 3. Verify changed init behavior

- [x] 3.1 Add/adjust CLI tests for allowed source-root Pi install, continued source-root Claude Code refusal, and source-subdirectory root resolution.
- [x] 3.2 Add/adjust tests for unchanged collision behavior when `.pi/extensions/pi-trace/index.ts` already exists.
- [x] 3.3 Update OpenSpec docs/spec references affected by source-repo Pi install semantics.
