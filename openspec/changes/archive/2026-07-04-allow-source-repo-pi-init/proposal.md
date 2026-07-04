## Why

`scryrs init` currently refuses to run anywhere inside the scryrs source checkout, including this repository root. That keeps the source repo clean, but it also blocks the simplest dogfooding path for Pi users working on scryrs itself. The repository already carries live `.pi/` project configuration, so forbidding a project-local Pi install in root is stricter than necessary and makes trace-hook testing clumsy.

## What Changes

- Allow `scryrs init --agent pi` to run from the scryrs source repository root and subdirectories beneath it.
- Keep the source-repo self-install refusal for `scryrs init --agent claude-code` unchanged.
- Preserve `hooks/pi/index.ts` as canonical source and treat `.pi/extensions/pi-trace/index.ts` as installed, non-canonical runtime copy only.
- Add maintainer/agent guidance in `AGENTS.md` clarifying canonical-vs-installed path ownership:
  - `hooks/pi/index.ts` is source of truth
  - `.pi/extensions/pi-trace/index.ts` is installed artifact for local dogfooding
  - LLMs/agents must not treat installed copy as leading source and must not edit it directly
- Add ignore/documentation coverage so local root-repo Pi installs do not create noisy working-tree churn or source-of-truth ambiguity.

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- `init-installer`: change self-install behavior so Pi may be installed into the scryrs source repo for local dogfooding, while preserving canonical-source boundaries and keeping Claude Code blocked.

## Impact

- `crates/scryrs-cli/src/init.rs` self-install logic and related tests
- `openspec/specs/init-installer/spec.md`
- `AGENTS.md` maintainer instructions for canonical hook source vs installed Pi copy
- likely `.gitignore` and Pi install/readme guidance to keep local installed artifacts non-canonical and low-noise
