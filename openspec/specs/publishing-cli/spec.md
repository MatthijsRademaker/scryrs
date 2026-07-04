# publishing-cli Specification

## Purpose
TBD - created by archiving change task-dccdf3bd-5dc5-4480-8f9f-caedf8ccd911. Update Purpose after archive.
## Requirements
### Requirement: `scryrs publish` is a first-class shipped CLI command

The shipped CLI SHALL expose a grouped `publish` root command with exactly two subcommands:

- `scryrs publish markdown <PATH> --output <DIR>`
- `scryrs publish rspress <PATH> --docs-root <DIR>`

Both subcommands SHALL be available in the default shipped binary. The publish command group SHALL appear in human help, machine-readable `--help-json`, CLI docs, and doctor command-surface reporting.

#### Scenario: Root help exposes publish commands

- **WHEN** a caller invokes `scryrs --help`
- **THEN** the output includes a `publish` command entry
- **AND** the entry describes accepted-knowledge publishing operations
- **AND** the help text documents both `markdown` and `rspress` publish modes with their required arguments

#### Scenario: Help-json exposes the grouped publish surface

- **WHEN** a caller invokes `scryrs --help-json`
- **THEN** the JSON surface document contains a `publish` command entry
- **AND** that entry exposes nested `markdown` and `rspress` subcommands with their required arguments and flags
- **AND** `surfaceVersion` is `0.14.0`

#### Scenario: Default shipped binary includes rspress publish mode

- **GIVEN** the default shipped `scryrs` binary
- **WHEN** a caller invokes `scryrs publish rspress <PATH> --docs-root <DIR>` with valid inputs
- **THEN** the command is available without requiring a special all-features build

#### Scenario: Missing or unknown publish subcommands fail as usage errors

- **WHEN** a caller invokes `scryrs publish` without a subcommand
- **OR** invokes `scryrs publish html <PATH>`
- **THEN** the command exits `2`
- **AND** stderr reports a usage error with publish-specific guidance
- **AND** no filesystem writes occur

### Requirement: Markdown publish delegates to the accepted-markdown adapter

`scryrs publish markdown <PATH> --output <DIR>` SHALL delegate to `scryrs-adapter-markdown::publish_accepted_markdown(repository_root, output_root)`. It SHALL publish only accepted Markdown-backed review decisions, write deterministic files under `<DIR>/<target-type>/<proposal-id>.md`, emit a deterministic JSON summary to stdout, and SHALL NOT delete stale files in the generic Markdown output root.

The JSON summary SHALL include `command`, `mode`, `schemaVersion`, `count`, and `paths` fields.

#### Scenario: Accepted Markdown decisions publish to deterministic paths

- **GIVEN** `.scryrs/accepted/*.json` contains valid accepted Markdown-backed decisions
- **WHEN** a caller invokes `scryrs publish markdown <PATH> --output <DIR>`
- **THEN** the command exits `0`
- **AND** Markdown files are written under `<DIR>/<target-type>/<proposal-id>.md`
- **AND** stdout is a deterministic JSON summary for the written paths

#### Scenario: Pending and rejected artifacts do not publish in markdown mode

- **GIVEN** `.scryrs/proposals/*.json` and `.scryrs/rejected/*.json` contain artifacts
- **AND** `.scryrs/accepted/` contains no publishable Markdown decisions
- **WHEN** a caller invokes `scryrs publish markdown <PATH> --output <DIR>`
- **THEN** no Markdown files are written
- **AND** no pending or rejected proposal IDs appear in the JSON summary

#### Scenario: Malformed accepted artifact fails loudly before markdown output

- **GIVEN** `.scryrs/accepted/{proposalId}.json` is malformed or fails `ProposalReviewDecision` validation
- **WHEN** a caller invokes `scryrs publish markdown <PATH> --output <DIR>`
- **THEN** the command exits `2`
- **AND** stderr reports the invalid accepted artifact
- **AND** the run does not emit partial Markdown output for that publish invocation

### Requirement: Rspress publish delegates to the accepted-rspress adapter

`scryrs publish rspress <PATH> --docs-root <DIR>` SHALL delegate to `scryrs-adapter-rspress::publish_accepted_rspress(repository_root, docs_root)`. It SHALL publish accepted Markdown-backed review decisions into `<DIR>/accepted-knowledge/`, update `<DIR>/_nav.json` deterministically, and emit a deterministic JSON summary to stdout.

The JSON summary SHALL include `command`, `mode`, `schemaVersion`, `count`, and `entries`. Each entry SHALL include `path`, `proposalId`, `targetType`, `navText`, and `navLink`.

#### Scenario: Accepted decisions publish to Rspress pages and nav

- **GIVEN** `.scryrs/accepted/*.json` contains valid accepted Markdown-backed decisions
- **WHEN** a caller invokes `scryrs publish rspress <PATH> --docs-root <DIR>`
- **THEN** the command exits `0`
- **AND** `accepted-knowledge/` pages are written under `<DIR>`
- **AND** `<DIR>/_nav.json` is updated deterministically
- **AND** stdout is a deterministic JSON summary for the published entries

#### Scenario: Pending and rejected artifacts do not publish in rspress mode

- **GIVEN** `.scryrs/proposals/*.json` and `.scryrs/rejected/*.json` contain artifacts
- **AND** `.scryrs/accepted/` contains no publishable Markdown decisions
- **WHEN** a caller invokes `scryrs publish rspress <PATH> --docs-root <DIR>`
- **THEN** no `accepted-knowledge/` pages are written
- **AND** `_nav.json` contains no pending or rejected proposal IDs

#### Scenario: Malformed nav fails before rspress writes

- **GIVEN** `<DIR>/_nav.json` is malformed
- **AND** `<DIR>/accepted-knowledge/` already contains previously published files
- **WHEN** a caller invokes `scryrs publish rspress <PATH> --docs-root <DIR>`
- **THEN** the command exits `2`
- **AND** stderr reports the malformed nav input
- **AND** the pre-existing `accepted-knowledge/` output remains unchanged for that failed run

### Requirement: Publish output uses documented success and failure codes

Both publish modes SHALL use exit `0` for success, exit `2` for usage errors and publish-input validation failures, and exit `1` for runtime or filesystem failures. Diagnostics SHALL be written to stderr, and stdout SHALL remain reserved for the deterministic JSON success summary.

#### Scenario: Missing required flag fails with usage error

- **WHEN** a caller invokes `scryrs publish markdown <PATH>` without `--output`
- **OR** invokes `scryrs publish rspress <PATH>` without `--docs-root`
- **THEN** the command exits `2`
- **AND** stderr reports the missing required flag
- **AND** no filesystem writes occur

#### Scenario: Runtime write failure exits 1

- **GIVEN** valid accepted publish inputs
- **AND** the target output location cannot be written
- **WHEN** a caller invokes either publish mode
- **THEN** the command exits `1`
- **AND** stderr reports the write failure

### Requirement: Publishing remains explicit and separate from proposal review

Publishing SHALL remain a separate operator action over accepted review decisions. `scryrs proposals accept`, `scryrs proposals reject`, and `scryrs proposals list` SHALL NOT trigger Markdown or Rspress publication as a side effect.

#### Scenario: Accepting a proposal does not auto-publish

- **GIVEN** a valid pending proposal exists
- **WHEN** a caller invokes `scryrs proposals accept <PATH> <ID> ...`
- **THEN** `.scryrs/accepted/{ID}.json` is written as reviewed evidence
- **AND** no generic Markdown publish output is created
- **AND** no Rspress `accepted-knowledge/` pages or `_nav.json` updates occur until a separate `scryrs publish ...` command is run

### Requirement: Docs and doctor surfaces stay aligned with the publish contract

The CLI docs and doctor command surface SHALL describe both publish modes, their accepted-only/manual-publish boundary, and their exit-code behavior.

#### Scenario: Doctor reports publish in the shipped command surface

- **WHEN** a caller invokes `scryrs doctor --json`
- **THEN** the reported command surface includes `publish`
- **AND** the surface remains consistent with the shipped feature availability for `markdown` and `rspress`

#### Scenario: Operator docs explain explicit accepted-only publishing

- **WHEN** a reader consults `.devagent/docs/docs/cli-v0-contract.md`, `.devagent/docs/docs/proposals.md`, or `.devagent/docs/docs/production-suite.md`
- **THEN** they can discover `scryrs publish markdown` and `scryrs publish rspress`
- **AND** the docs state that publishing reads accepted review decisions only
- **AND** the docs state that `scryrs proposals accept` does not publish automatically

