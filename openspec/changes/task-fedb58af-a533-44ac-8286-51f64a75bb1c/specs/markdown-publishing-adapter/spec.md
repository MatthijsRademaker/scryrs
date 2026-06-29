## ADDED Requirements

### Requirement: Markdown publishing reads accepted evidence only

The Markdown publishing entrypoint SHALL load publish inputs exclusively from `.scryrs/accepted/*.json` under the caller-supplied repository root. It SHALL validate each artifact as a `ProposalReviewDecision` and SHALL NOT read `.scryrs/proposals/*.json` or `.scryrs/rejected/*.json` as publish inputs.

#### Scenario: Pending proposals do not publish

- **GIVEN** `.scryrs/proposals/*.json` contains pending proposals and `.scryrs/accepted/` contains no accepted decisions
- **WHEN** the Markdown publisher runs against that repository and an output root
- **THEN** no Markdown files are written
- **AND** no pending proposal content is published

#### Scenario: Missing accepted directory is an empty success

- **GIVEN** the repository has no `.scryrs/accepted/` directory
- **WHEN** the Markdown publisher runs
- **THEN** it completes without writing output
- **AND** it does not treat the missing directory as an error

#### Scenario: Malformed accepted decision fails loudly

- **GIVEN** `.scryrs/accepted/{proposalId}.json` exists but is malformed or fails `ProposalReviewDecision` validation
- **WHEN** the Markdown publisher runs
- **THEN** the run fails loudly
- **AND** it does not emit a fake partial-success result for that invalid artifact

### Requirement: Accepted Markdown decisions publish to deterministic generic paths

The publisher SHALL process accepted decisions in `proposalId` ascending order and SHALL write Markdown-backed accepted decisions to `<output-root>/<target-type>/<proposal-id>.md`. It SHALL support `docs_note`, `adr`, `skill`, and `debugging_playbook` accepted content. It SHALL create or overwrite current target files only and SHALL NOT delete stale files in the output root.

#### Scenario: Accepted Markdown target types use stable paths

- **GIVEN** accepted `docs_note` and `adr` decisions with Markdown `acceptedContent`
- **WHEN** the publisher runs with a chosen output root
- **THEN** it writes `<output-root>/docs_note/<proposal-id>.md` and `<output-root>/adr/<proposal-id>.md`
- **AND** the paths are derived only from `targetType` and `proposalId`

#### Scenario: Repeated publish runs are byte-stable

- **GIVEN** the same set of accepted Markdown decisions and the same output root
- **WHEN** the publisher runs twice
- **THEN** the second run overwrites the same target paths only
- **AND** each written file is byte-identical to the first run's output

#### Scenario: Non-Markdown accepted decisions are skipped

- **GIVEN** an accepted `memory_patch` or `semantic_graph_grouping` decision exists under `.scryrs/accepted/`
- **WHEN** the Markdown publisher runs
- **THEN** it does not write a Markdown file for that decision
- **AND** the run does not fail solely because that accepted decision is non-Markdown

### Requirement: Rendered Markdown preserves review provenance and evidence backlinks

Each published file SHALL render generic Markdown only: a deterministic heading derived from `proposalId` and `targetType`, a `Review Metadata` block, the accepted Markdown body, and an `Evidence backlinks` section. The metadata SHALL include `proposalId`, `targetType`, `reviewer`, `decidedAt`, and `rationale`. The evidence section SHALL include every `sourceEvidence` entry and preserve available source-kind, subject, row IDs, doc reference, description, and score fields without Rspress frontmatter or route assumptions.

#### Scenario: Rendered file includes review metadata and accepted body

- **GIVEN** an accepted Markdown decision with reviewer metadata and non-empty `acceptedContent`
- **WHEN** the publisher writes the target file
- **THEN** the file includes a `Review Metadata` block containing `proposalId`, `targetType`, `reviewer`, `decidedAt`, and `rationale`
- **AND** the accepted Markdown body appears after that metadata block

#### Scenario: Evidence backlinks preserve available source fields

- **GIVEN** `sourceEvidence` entries containing source kind, subject, row IDs, and optional doc-reference metadata
- **WHEN** the publisher renders `Evidence backlinks`
- **THEN** every evidence entry appears in the section
- **AND** available row IDs, `docRef`, `description`, and `score` values are preserved in the rendered output

### Requirement: Markdown publishing remains framework-agnostic and library-only

This foundation slice SHALL expose Markdown publishing from `crates/scryrs-adapter-markdown` as a library API that takes repository and output roots. It SHALL not add a new CLI command in this change, SHALL not depend on `scryrs-adapter-rspress`, and SHALL not assume `.devagent/docs/` layout, Rspress routes, or frontmatter conventions.

#### Scenario: Caller chooses an arbitrary output root

- **GIVEN** a writable output directory outside `.devagent/docs/`
- **WHEN** the publisher runs
- **THEN** files are written relative to that output directory
- **AND** the published content remains plain Markdown without Rspress-specific metadata

#### Scenario: Existing CLI surface remains unchanged

- **WHEN** this foundation change is implemented
- **THEN** no new `publish` or `markdown` command is required on the `scryrs` CLI
- **AND** Markdown publishing is provided by the adapter library only

### Requirement: Project docs describe the reviewed-evidence publishing boundary

Project documentation SHALL state that generic Markdown publishing consumes accepted review decisions from `.scryrs/accepted/` rather than raw proposal inbox files, and that Markdown is the generic publishing surface before any Rspress-specific follow-up.

#### Scenario: Proposals docs explain accepted-only publishing

- **WHEN** a reader consults `.devagent/docs/docs/proposals.md`
- **THEN** it describes `.scryrs/accepted/` as the Markdown publishing input
- **AND** it does not describe `.scryrs/proposals/` as automatically publishable adapter input

#### Scenario: Production suite docs place Markdown before Rspress

- **WHEN** a reader consults `.devagent/docs/docs/production-suite.md`
- **THEN** the publishing milestone describes reviewed knowledge leaving `.scryrs/` through the generic Markdown adapter
- **AND** Rspress remains a later, separate publishing surface
