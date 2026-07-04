## 1. Refresh scaffold baseline

- [x] 1.1 Capture current drift between `scryrs` checked-in `.pi/` tree and current Swarm defaults (`agents`, `prompts`, `skills`, `rules`, readme, manifest-sensitive files).
- [x] 1.2 Run the repository-supported scaffold refresh path (`swarm init` or equivalent) in a branch and record which files are added, removed, or rewritten.
- [x] 1.3 Classify resulting changes into adopt/preserve/ignore buckets before finalizing edits.

## 2. Reconcile checked-in `.pi` runtime files

- [x] 2.1 Restore missing current-default prompt/readme/skill files required by scaffold sync (`swarm-plan.md`, `swarm-execute-plan.md`, `swarm-execute-task.md`, `.pi/README.md`, `read-project-docs`).
- [x] 2.2 Update `.pi/agents/*.md` to replace stale default skill references (`project-docs` → `read-project-docs`) and remove default `ccc` wiring where it is no longer part of the active Swarm contract.
- [x] 2.3 Update `.pi/prompts/*` and `.pi/rules/*` that changed upstream so outcome-reporting, archive, and verification guidance match current Swarm defaults.

## 3. Preserve scryrs-specific behavior

- [x] 3.1 Re-apply or preserve intentional `scryrs`-specific agent customizations such as model override frontmatter fields after scaffold reconciliation.
- [x] 3.2 Preserve repository-specific guidance and skills, including `shadcn-vue` usage and Pi trace-hook ownership rules in `AGENTS.md`.
- [x] 3.3 Ensure sync work does not introduce foreign stack-specific verification metadata or Go-oriented script contracts into the Rust `scryrs` repository.

## 4. Validate and document

- [x] 4.1 Update maintainer-facing docs (`AGENTS.md`, `.devagent/README.md`, or nearby references) where scaffold behavior or expected `.pi` contents changed.
- [x] 4.2 Verify final checked-in `.pi` tree and manifest-sensitive paths are internally consistent after the sync.
- [x] 4.3 Run repository verification (`scripts/precommit-run`) and any targeted checks needed to prove scaffold sync did not break `scryrs` conventions.
