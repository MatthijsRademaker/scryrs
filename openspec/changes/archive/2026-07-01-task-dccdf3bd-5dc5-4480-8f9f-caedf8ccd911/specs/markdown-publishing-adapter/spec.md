## MODIFIED Requirements

### Requirement: Markdown publishing remains framework-agnostic behind a reusable library boundary

The Markdown publishing capability SHALL expose `publish_accepted_markdown()` from `crates/scryrs-adapter-markdown` as the reusable rendering boundary for generic accepted-knowledge Markdown. Callers, including the shipped `scryrs` CLI, SHALL invoke this library API rather than re-implementing accepted-artifact loading, `ProposalReviewDecision` validation, `proposalId` sorting, Markdown rendering, or target-path derivation.

The adapter SHALL NOT depend on `scryrs-adapter-rspress`, SHALL NOT assume `.devagent/docs/` layout, Rspress routes, or frontmatter conventions, and SHALL continue to render plain generic Markdown at caller-selected output roots.

#### Scenario: CLI markdown publish reuses the library boundary

- **GIVEN** accepted Markdown-backed review decisions and a writable output directory outside `.devagent/docs/`
- **WHEN** a caller invokes `scryrs publish markdown <PATH> --output <DIR>`
- **THEN** the CLI delegates rendering to `publish_accepted_markdown()`
- **AND** the written files remain plain generic Markdown under the caller-selected output root
- **AND** the CLI does not duplicate adapter rendering logic

#### Scenario: Non-CLI callers keep the same generic Markdown boundary

- **GIVEN** any non-CLI caller that needs accepted Markdown output
- **WHEN** it invokes `publish_accepted_markdown(repo_root, output_root)` directly
- **THEN** the same deterministic accepted-only Markdown output is produced
- **AND** the adapter remains independent from Rspress-specific layout or metadata