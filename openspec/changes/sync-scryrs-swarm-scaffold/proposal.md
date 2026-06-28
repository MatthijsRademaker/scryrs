## Why

The checked-in Swarm/Pi scaffold in `scryrs` has drifted behind current Swarm defaults. Agent definitions still reference retired skills (`ccc`, `project-docs`), some current default prompt files are missing, and repository guidance no longer matches the scaffold contract that newer Swarm tooling expects. This should be corrected now, before more project-specific customization accumulates on top of stale defaults.

## What Changes

- Sync the checked-in `.pi/` scaffold in `scryrs` with current Swarm defaults where those defaults are still part of the active runtime contract.
- Replace stale skill wiring (`project-docs` → `read-project-docs`, remove default `ccc` usage) across Swarm agent definitions and related references.
- Restore current default prompt/skill/readme files that are missing from the repository's materialized `.pi/` tree.
- Update Swarm-facing guidance and prompt text so verification, archive, and outcome-reporting instructions match current Swarm behavior.
- Preserve intentional `scryrs`-specific customizations such as model overrides, Rust verification scripts, `shadcn-vue`, and Pi trace-hook ownership rules.
- Explicitly avoid copying stack-specific `dev-swarm` verification metadata or Go-specific tooling contracts into the Rust `scryrs` repository.

## Capabilities

### New Capabilities
- `swarm-scaffold-sync`: Defines how an already-initialized `scryrs` repository keeps its checked-in Swarm/Pi scaffold synchronized with current Swarm defaults without clobbering project-specific customizations.

### Modified Capabilities
- `init-installer`: Clarify repository-local Pi dogfooding expectations so checked-in scaffold guidance and installed Pi runtime-copy guidance do not drift.

## Impact

- `.pi/agents/*.md`
- `.pi/prompts/*.md`
- `.pi/skills/read-project-docs/` and stale skill references
- `.pi/rules/*.md` and `.pi/README.md`
- `.pi/.swarm-pi-manifest.json` and any scaffold metadata updated by `swarm init`
- `AGENTS.md`, `.devagent/README.md`, and related docs where current Swarm guidance is referenced
- Verification flow validation via existing Rust/Docker-backed `scripts/precommit-run`
