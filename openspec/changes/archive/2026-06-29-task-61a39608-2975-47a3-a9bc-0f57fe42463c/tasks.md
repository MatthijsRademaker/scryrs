## 1. Rspress adapter crate implementation

- [x] 1.1 Add `scryrs-adapter-markdown` as a dependency in `crates/scryrs-adapter-rspress/Cargo.toml`.
- [x] 1.2 Implement `publish_accepted_rspress(repository_root, docs_root)` entrypoint that calls `publish_accepted_markdown(repo_root, temp_dir)`, reads published files, transforms them into Rspress pages, and writes them to `.devagent/docs/docs/accepted-knowledge/`.
- [x] 1.3 Add declarative subtree cleanup: remove `.devagent/docs/docs/accepted-knowledge/` at the start of each publish run before regenerating.
- [x] 1.4 Add Rspress frontmatter generation: each page gets `title` (derived from target type + proposal ID) and `sidebar_label` (target-type-slug:truncated-proposal-id) in YAML frontmatter, confined to the adapter.
- [x] 1.5 Define `PublishEntry` struct with fields: `path`, `target_type`, `proposal_id`, `nav_text`, `nav_link` — returned from `publish_accepted_rspress()` for machine-readable verification.
- [x] 1.6 Remove the stub `RspressRoute` struct if unused after implementation, or repurpose it only if it serves the publish path.

## 2. Navigation (_nav.json) integration

- [x] 2.1 Implement `update_nav_json(docs_root, published_entries)` that reads `.devagent/docs/docs/_nav.json`, strips any existing "Accepted Knowledge" section, and appends a fresh section.
- [x] 2.2 Within the Accepted Knowledge section, create per-target-type sub-sections for each target type that has published artifacts, with items sorted by `proposalId`.
- [x] 2.3 Ensure strip-and-rebuild is idempotent: reruns with identical input produce byte-identical `_nav.json`.
- [x] 2.4 Handle edge cases: empty published set (remove Accepted Knowledge section entirely), missing `_nav.json` (create from scratch with only the Accepted Knowledge section), malformed `_nav.json` (fail loudly).

## 3. Adapter tests

- [x] 3.1 Add tests proving the adapter only publishes from accepted artifacts (pending proposals produce no Rspress pages).
- [x] 3.2 Add tests for deterministic reruns: same accepted input produces byte-identical docs-source output and `_nav.json`.
- [x] 3.3 Add tests proving accepted-knowledge/ subtree is cleared and regenerated (stale pages from removed decisions are gone).
- [x] 3.4 Add tests verifying non-Markdown accepted decisions (memory_patch, semantic_graph_grouping) produce no Rspress pages or nav entries.
- [x] 3.5 Add tests for nav merge: hand-authored sections preserved, Accepted Knowledge section replaces on rerun, empty published set removes section.
- [x] 3.6 Add tests for malformed `_nav.json` failing loudly.
- [x] 3.7 Add tests verifying `PublishEntry` return values match written files and nav entries.

## 4. Build verification script

- [x] 4.1 Create `scripts/verify-docs-publish` shell script that creates test fixtures, runs the adapter, builds docs, and asserts llms content.
- [x] 4.2 Script must fail loudly with clear messages if bun/node is missing (not silently pass).
- [x] 4.3 Script must assert that `doc_build/llms.txt` and `doc_build/llms-full.txt` contain published proposal IDs as substrings.
- [x] 4.4 Script must assert that links in llms files resolve to valid paths within the docs build output.
- [x] 4.5 Script must clean up after itself (remove generated fixtures and build artifacts).

## 5. CI and scripts/check integration

- [x] 5.1 Add `scripts/verify-docs-publish` invocation to `scripts/check` as a named step after existing checks.
- [x] 5.2 Add a docs-build verification step to `.github/workflows/ci.yml` that runs `scripts/verify-docs-publish` after the Rust tests pass.

## 6. Documentation

- [x] 6.1 Update `.devagent/docs/docs/production-suite.md` to note that the Rspress publishing adapter is now implemented, closing the P5 publishing milestone.
- [x] 6.2 Document the `accepted-knowledge/` subtree ownership in project docs so contributors know it is adapter-owned and should not receive hand-authored pages.

## 7. Scope guardrails

- [x] 7.1 Do not add a standalone CLI binary to `scryrs-adapter-rspress` in this slice.
- [x] 7.2 Do not add new fields or variants to `ProposalReviewDecision`, `ProposalDocument`, `ProposedContent`, `ProposalTargetType`, or any `scryrs-types` contract.
- [x] 7.3 Do not modify `scryrs-adapter-markdown` behavior or API — consume it as-is.
- [x] 7.4 Do not publish from `.scryrs/proposals/` or `.scryrs/rejected/`.
