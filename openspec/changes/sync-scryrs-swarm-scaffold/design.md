## Context

`scryrs` is already Swarm-initialized and carries a checked-in `.pi/` tree plus Swarm-facing documentation in `AGENTS.md` and `.devagent/README.md`. Recent Swarm scaffold changes landed upstream after the last `scryrs` sync: default agent skill wiring moved from `project-docs` to `read-project-docs`, default `ccc` usage was removed from agent skill lists, and current default prompt/readme files now include files missing from `scryrs` (`swarm-plan.md`, `swarm-execute-plan.md`, `swarm-execute-task.md`, `.pi/README.md`, `read-project-docs`).

At same time, `scryrs` has intentional repository-specific customizations that must not be overwritten: model override frontmatter, `shadcn-vue` skill guidance, Rust/Docker verification scripts, and Pi trace-hook source ownership rules. This is therefore not a blind copy problem. It is a selective scaffold reconciliation problem.

## Goals / Non-Goals

**Goals:**
- Bring checked-in `scryrs` Swarm/Pi scaffold back into alignment with active Swarm defaults.
- Restore missing default files and rename stale default skill references.
- Preserve `scryrs`-specific runtime, UI, verification, and hook-ownership customizations.
- Leave the repository in a state where future `swarm init` / scaffold reviews are easier to reason about.

**Non-Goals:**
- Do not introduce Go-specific Swarm verification metadata or command inventories into the Rust `scryrs` repository.
- Do not rewrite `scryrs` product behavior, hook logic, or runtime architecture.
- Do not remove repository-specific customizations solely because they differ from generic defaults.
- Do not treat installed `.pi/extensions/pi-trace/index.ts` as canonical source.

## Decisions

1. **Use current Swarm defaults as comparison baseline, not as blind replacement.**
   Run `swarm init` or compare against current scaffold defaults to surface drift, then reconcile file-by-file.

   Alternative rejected: manually copying files from `dev-swarm`. That risks dragging along repository-specific implementation details, stale generated files, or non-default behavior.

2. **Classify files into three buckets before editing: adopt, preserve, or ignore.**
   - **Adopt:** missing default prompts/skills/readmes and stale default skill names.
   - **Preserve:** model routing overrides, `shadcn-vue`, trace-hook ownership rules, Rust verification scripts.
   - **Ignore:** foreign stack-specific verification metadata such as `.devagent/verification.json` semantics from unrelated repositories.

   Alternative rejected: one-pass overwrite. That would likely clobber active `scryrs` customizations or create wrong-stack contracts.

3. **Treat agent skill wiring as contract-level sync work.**
   Updating `project-docs` → `read-project-docs` and removing default `ccc` references is not cosmetic. It changes which shared skills current Swarm prompts and defaults expect to exist.

   Alternative rejected: leaving stale skill names in place because they still happen to exist locally. That preserves drift and makes future scaffold upgrades harder.

4. **Keep verification sync constrained to repository-native scripts.**
   `scryrs` already has authoritative verification entrypoints (`scripts/check`, `scripts/test`, `scripts/security`, `scripts/precommit-run`). The sync should update Swarm guidance text to reflect current outcome/verification wording, but it should not import Go-oriented verification schemas.

   Alternative rejected: adding foreign verification metadata during this sync. That expands scope and risks misconfiguring worker expectations.

## Risks / Trade-offs

- [Risk] `swarm init` may touch more files than intended. → Mitigation: review diff bucket-by-bucket and keep only scaffold-sync changes.
- [Risk] Removing stale skill names may break local habits or undocumented workflows. → Mitigation: preserve optional local capabilities when intentionally needed, but remove them from default agent contracts.
- [Risk] Missing prompts may no longer be used by current workflows. → Mitigation: restore them only when they are part of current defaults; treat presence as scaffold completeness, not proof of runtime use.
- [Risk] Maintainers may assume upstream `dev-swarm` files are always safe to copy. → Mitigation: document explicit rule that foreign verification metadata and stack-specific scripts are out of scope.

## Migration Plan

1. Capture current drift between `scryrs` checked-in `.pi/` tree and current Swarm defaults.
2. Run `swarm init` or equivalent scaffold refresh in a branch.
3. Reconcile agent defs, prompts, skills, rules, and `.pi` readme to current defaults.
4. Re-apply or preserve intentional `scryrs`-specific customizations where default refresh would overwrite them.
5. Update documentation references in `AGENTS.md` / `.devagent/README.md` if scaffold behavior or expected files changed.
6. Verify with existing repository verification (`scripts/precommit-run`) and targeted inspection of `.pi/` ownership-sensitive paths.

## Open Questions

- Should `scryrs` keep any form of `ccc` or `project-docs` locally as non-default optional skills, or should they be removed entirely once agent definitions stop referencing them?
- Should `.pi/.swarm-pi-manifest.json` be treated as a fully regenerated artifact during sync, or only updated indirectly via `swarm init`?
- Are the missing planning prompts still part of active `scryrs` workflows, or are they present only because current upstream defaults still ship them?
