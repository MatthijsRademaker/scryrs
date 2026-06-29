## Why

The accepted-only Markdown publishing foundation (Foundation 01, shipped in `crates/scryrs-adapter-markdown`) can write reviewed knowledge as plain Markdown to any output root. However, no bridge exists to promote that approved knowledge into the live Rspress docs site at `.devagent/docs/` or its generated `llms.txt` surfaces. Until this adapter ships, approved scryrs knowledge stays invisible to human readers browsing the docs site and to future agents consuming `doc_build/llms.txt` — the publishing loop is open.

## What Changes

1. **Implement `scryrs-adapter-rspress`** as a downstream library that composes over `publish_accepted_markdown` from `scryrs-adapter-markdown`. It calls the Markdown adapter into a temp scratch directory, then reads, transforms, and relocates those files into `.devagent/docs/docs/accepted-knowledge/<target-type-slug>/<proposal-id>.md` with Rspress frontmatter (`title`, `sidebar_label`) confined to the adapter.
2. **Add deterministic `_nav.json` integration.** The adapter reads the existing `_nav.json`, strips any prior "Accepted Knowledge" section, and appends a fresh section with sub-sections grouped by target type and items sorted by `proposalId`. Reruns produce byte-stable output.
3. **Add `scripts/verify-docs-publish`** — a shell script that creates test fixtures, runs the adapter, executes `bun run build` in `.devagent/docs/`, and asserts that `doc_build/llms.txt` and `doc_build/llms-full.txt` contain published proposal IDs with valid links.
4. **Wire verification into `scripts/check` and CI.** Both `scripts/check` and `.github/workflows/ci.yml` gain a step that invokes `scripts/verify-docs-publish`.
5. **Library-only, no CLI binary.** The adapter exposes `publish_accepted_rspress(repo_root, docs_root) -> Result<Vec<PublishEntry>, PublishError>` as a library API, consistent with the Markdown adapter.

## Impact

- **Affected specs:** new `rspress-publishing-adapter` spec
- **Affected code:** `crates/scryrs-adapter-rspress/` (lib, Cargo.toml gains dependency on `scryrs-adapter-markdown`), `.devagent/docs/docs/_nav.json` (consumed at runtime), `scripts/check`, `.github/workflows/ci.yml`
- **No changes to:** `scryrs-types`, `scryrs-core`, `scryrs-graph`, `scryrs-curator`, `scryrs-adapter-markdown` (except as a consumed dependency), proposal generation, or review-decision contracts. Rspress-specific frontmatter and route shape do not leak into core/proposal contracts.