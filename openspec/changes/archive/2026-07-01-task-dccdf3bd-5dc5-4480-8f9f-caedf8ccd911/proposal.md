## Why

Operators already have accepted-knowledge publishing libraries, but the shipped `scryrs` CLI still has no first-class publish surface. Today, getting reviewed knowledge out of `.scryrs/accepted/` requires crate examples or internal scripts, which violates the product requirement for an explicit operator command and leaves help, `--help-json`, doctor output, docs, and production verification out of sync with the actual publishing story.

This change closes that product-surface gap without reopening adapter rendering logic. It adds an explicit `scryrs publish` command for the existing Markdown and Rspress adapters, keeps publishing review-first and manual, documents the CLI contract, and verifies both publish modes through the real binary.

## What Changes

- Add a first-class `scryrs publish` root command with nested `markdown` and `rspress` subcommands:
  - `scryrs publish markdown <PATH> --output <DIR>`
  - `scryrs publish rspress <PATH> --docs-root <DIR>`
- Ship `publish rspress` in the default CLI binary by enabling the `rspress` feature in the default `scryrs-cli` feature set.
- Implement a thin CLI executor that delegates directly to `scryrs-adapter-markdown::publish_accepted_markdown` and `scryrs-adapter-rspress::publish_accepted_rspress` rather than duplicating accepted-artifact loading, Markdown rendering, Rspress frontmatter generation, or nav rebuild logic.
- Emit deterministic JSON summaries on stdout for both publish modes, keep diagnostics on stderr, and document the exit-code policy:
  - exit `0` for success
  - exit `2` for usage errors and publish-input validation failures such as malformed accepted artifacts or malformed Rspress `_nav.json`
  - exit `1` for runtime/filesystem failures
- Keep publishing explicit and review-first: pending and rejected artifacts never publish, and `scryrs proposals accept` / `reject` remain ledger-only commands that do not trigger publication.
- Update help text, `--help-json` (surface version `0.14.0`), doctor command-surface reporting, CLI docs, proposals docs, production-suite docs, and snapshots to reflect the new publish surface.
- Update production verification so the real `scryrs` binary exercises both publish modes, while preserving the existing docs build and llms-output assertions for Rspress publishing.

## Impact

- **Affected code paths**: `crates/scryrs-cli/src/dispatch.rs`, new `crates/scryrs-cli/src/publish.rs`, `crates/scryrs-cli/src/help_text.rs`, `crates/scryrs-cli/src/help_json.rs`, `crates/scryrs-cli/src/doctor.rs`, `crates/scryrs-cli/Cargo.toml`, CLI tests/snapshots, and `scripts/verify-docs-publish`.
- **Affected docs**: `.devagent/docs/docs/cli-v0-contract.md`, `.devagent/docs/docs/proposals.md`, and `.devagent/docs/docs/production-suite.md`.
- **Affected specs**: add `publishing-cli`; update the Markdown adapter boundary to remain the reusable renderer behind the CLI; require proposal review commands to stay non-publishing; require production verification to exercise the shipped publish CLI.
- **Non-goals preserved**: no proposal-schema changes, no automatic publish during proposal acceptance, no dashboard publishing UI, no LLM-assisted publishing, no adapter output-format redesign, and no stale-file deletion for generic Markdown output.
- **Behavioral boundary**: this is a CLI-surface integration and verification/documentation change; accepted-artifact semantics and adapter-owned rendering rules remain the source of truth.