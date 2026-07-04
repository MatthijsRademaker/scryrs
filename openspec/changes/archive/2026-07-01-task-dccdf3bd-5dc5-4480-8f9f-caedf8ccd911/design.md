## Context

The repository already contains two accepted-knowledge publishing adapters:

- `scryrs-adapter-markdown::publish_accepted_markdown(repository_root, output_root)`
- `scryrs-adapter-rspress::publish_accepted_rspress(repository_root, docs_root)`

Those adapters already enforce the core publishing rules that matter here: they read only `.scryrs/accepted/*.json`, ignore pending and rejected proposal artifacts, produce deterministic output, and fail loudly on malformed accepted artifacts. The Rspress adapter additionally validates `_nav.json` before clearing or rewriting `accepted-knowledge/`, which is the no-partial-write boundary required by the task.

The missing product surface is the shipped CLI. `scryrs publish ...` is currently absent from dispatch, help text, help-json, doctor command-surface reporting, docs, and production verification. `crates/scryrs-cli/Cargo.toml` also leaves `rspress` out of the default feature set, which conflicts with the requirement that `scryrs publish rspress` be a first-class operator command.

## Goals / Non-Goals

### Goals

- Expose `scryrs publish markdown <PATH> --output <DIR>` as the operator-facing generic Markdown publishing command.
- Expose `scryrs publish rspress <PATH> --docs-root <DIR>` as the operator-facing Rspress docs publishing command.
- Keep publishing explicit and accepted-only: pending/rejected artifacts never publish, and proposal review does not auto-publish.
- Reuse the existing adapter APIs without duplicating accepted-artifact parsing, Markdown rendering, Rspress frontmatter generation, or nav rebuild logic.
- Align help text, `--help-json`, doctor reporting, docs, tests, and production verification with the new command surface.
- Preserve loud failure semantics and the existing Rspress no-partial-write guarantee.

### Non-Goals

- Changing `ProposalDocument`, `ProposalReviewDecision`, `ProposedContent`, `ProposalTargetType`, or evidence schemas.
- Adding automatic publish behavior to `scryrs propose`, `scryrs proposals accept`, or `scryrs proposals reject`.
- Adding dashboard publishing UI or LLM-assisted publishing.
- Reworking adapter output formats, Rspress frontmatter conventions, nav structure, or Markdown evidence rendering beyond CLI integration.
- Adding stale-file cleanup for generic Markdown publishing.

## Decisions

### Decision 1: Ship `publish` as a first-class grouped CLI command

The CLI will expose a grouped `publish` root command with nested `markdown` and `rspress` subcommands. The command is integrated into the existing root-command handling and discoverability surfaces, and `rspress` is enabled in the default `scryrs-cli` feature set so `scryrs publish rspress` exists in the shipped binary rather than only behind `--all-features`.

### Decision 2: Keep the CLI layer thin and adapter-backed

`scryrs publish markdown` delegates to `publish_accepted_markdown(repository_root, output_root)`. `scryrs publish rspress` delegates to `publish_accepted_rspress(repository_root, docs_root)`. The CLI must not duplicate accepted-artifact loading, decision validation, Markdown rendering, Rspress frontmatter generation, or `_nav.json` rebuild logic.

This preserves the existing adapter boundaries:

- Markdown remains the reusable generic renderer for accepted Markdown-backed decisions.
- Rspress remains the adapter-owned transformation from accepted Markdown output into `accepted-knowledge/` pages plus `_nav.json` updates.

### Decision 3: Publish output and exit behavior follow the existing CLI contract

Both publish modes emit deterministic JSON summaries to stdout and keep diagnostics on stderr. The documented exit policy is:

- `0`: success
- `2`: usage errors and publish-input validation failures (missing/invalid subcommands, missing required flags, malformed accepted artifacts, malformed Rspress `_nav.json`)
- `1`: runtime or filesystem failures after argument validation

This keeps publish aligned with the established CLI contract while preserving the task’s requirement that malformed inputs fail loudly and do not report fake success.

### Decision 4: Publishing remains explicit and review-first

Publishing consumes accepted review decisions only. Pending proposals under `.scryrs/proposals/` and rejected decisions under `.scryrs/rejected/` are never publish inputs. `scryrs proposals accept` and `scryrs proposals reject` remain ledger-only review operations: they create reviewed decision artifacts, but they do not invoke Markdown or Rspress publication as a side effect.

### Decision 5: Discoverability and verification must match the shipped surface

The new publish surface must appear consistently in:

- `scryrs --help`
- `scryrs --help-json` with `surfaceVersion` bumped to `0.14.0`
- `scryrs doctor --json` command-surface reporting
- `.devagent/docs/docs/cli-v0-contract.md`
- `.devagent/docs/docs/proposals.md`
- `.devagent/docs/docs/production-suite.md`
- CLI tests and snapshots
- production verification through the real `scryrs` binary

`scripts/verify-docs-publish` is the required verification lane for this change: it must exercise both `scryrs publish markdown` and `scryrs publish rspress`, then continue to prove that published Rspress output reaches the docs build and llms surfaces.

## Risks

| Risk | Mitigation |
| --- | --- |
| Enabling `rspress` by default increases default CLI dependency weight and build cost. | Accept the cost because the task requires `publish rspress` as a first-class shipped command; keep the change localized to feature composition and publish integration. |
| Existing adapter `PublishError` shapes may not cleanly distinguish validation failures from runtime I/O failures. | Preserve the documented exit-code split without duplicating rendering logic; keep any error classification work minimal and confined to supporting the CLI contract. |
| Moving verification from adapter example invocation to the real CLI path could break the current docs-publish lane if the script is not updated carefully. | Reuse the existing deterministic fixture flow in `scripts/verify-docs-publish`, but route publication through the built `scryrs` binary before the existing docs build and llms assertions. |

## Conflict Resolution

1. **Dispatch architecture**: refinement disagreed between a dedicated pre-clap executor path and a clap-native grouped subcommand. Resolved in favor of the accepted architect decision: model `publish` as a grouped command with nested `markdown` and `rspress` subcommands while still updating the root command handling so `publish` is recognized correctly.
2. **Default feature availability**: refinement questioned whether `rspress` could stay feature-gated. Resolved in favor of default inclusion because the task requires `scryrs publish rspress` to be first-class in the shipped CLI.
3. **Exit-code policy**: refinement raised ambiguity around adapter validation failures. Resolved to the accepted decision: malformed accepted artifacts and malformed `_nav.json` are exit `2`; runtime/filesystem failures are exit `1`.
4. **Missing publish subcommand behavior**: one refinement question suggested matching the bare-help pattern used elsewhere, but the task acceptance criteria require missing publish subcommands to fail with established usage-error handling. Resolved to usage error exit `2` with no filesystem writes.

## Traceability

| Source | Use in this design |
| --- | --- |
| Task `dccdf3bd-5dc5-4480-8f9f-caedf8ccd911` | Defines the operator scenarios, technical notes, and acceptance criteria for the new publish CLI. |
| Dossier `2026-07-01T07:04:55.167Z` | Supplies goals, non-goals, affected areas, assumptions, open questions, and the baseline proposal sketch. |
| Decision `1-swarm-architect-recommendation` | Fixes first-class `rspress` shipping, grouped publish command structure, and exit-code policy. |
| Decision `1-swarm-lead-dev-recommendation` | Fixes adapter-backed executor scope, deterministic JSON stdout, help-json version bump, and CLI-path verification. |
| Decision `1-swarm-reviewer-recommendation` | Confirms affected areas and the need to align dispatch, feature composition, doctor reporting, and verification. |
| Round outputs `round:1:agent:swarm-architect`, `round:1:agent:swarm-lead-dev`, `round:1:agent:swarm-reviewer` | Supply implementation-facing evidence for dispatch integration, default-feature implications, help/help-json updates, and verification-script changes. |