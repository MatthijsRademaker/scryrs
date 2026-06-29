# rspress-publishing-adapter Specification

## Purpose

Defines the Rspress publishing adapter as a downstream library that composes over the accepted-only Markdown publisher (`scryrs-adapter-markdown`), writes Rspress-formatted pages into a dedicated subtree of `.devagent/docs/docs/`, deterministically updates `_nav.json`, and supports build-level verification that published knowledge appears in regenerated `llms.txt` surfaces.

## ADDED Requirements

### Requirement: Rspress adapter publishes accepted knowledge only

The Rspress adapter SHALL compose over `publish_accepted_markdown()` from `scryrs-adapter-markdown`, which loads only `.scryrs/accepted/*.json`. It SHALL NOT read `.scryrs/proposals/` or `.scryrs/rejected/` directly, and SHALL NOT publish pending or rejected proposal artifacts.

#### Scenario: Pending proposals do not produce Rspress pages

- **GIVEN** `.scryrs/proposals/*.json` contains pending proposals and `.scryrs/accepted/` is empty
- **WHEN** the Rspress adapter runs against that repository and a docs root
- **THEN** no pages are written to `accepted-knowledge/`
- **AND** `_nav.json` contains no "Accepted Knowledge" section

#### Scenario: Mixed pending and accepted repositories publish only accepted

- **GIVEN** both `.scryrs/proposals/` and `.scryrs/accepted/` contain artifacts for the same target type
- **WHEN** the Rspress adapter runs
- **THEN** only pages corresponding to accepted decisions are published
- **AND** pending proposal IDs do not appear in `_nav.json`

### Requirement: Pages are written to a dedicated generated subtree

The Rspress adapter SHALL write pages to `.devagent/docs/docs/accepted-knowledge/<target-type-slug>/<proposal-id>.md`. It SHALL clear the `accepted-knowledge/` subtree at the start of each publish run before regeneration.

#### Scenario: Pages use deterministic paths under accepted-knowledge

- **GIVEN** accepted `docs_note` and `adr` decisions with Markdown content
- **WHEN** the Rspress adapter publishes to a docs root
- **THEN** pages are written at `accepted-knowledge/docs_note/<proposal-id>.md` and `accepted-knowledge/adr/<proposal-id>.md`
- **AND** paths are derived only from `targetType` slug and `proposalId`

#### Scenario: Stale pages are removed on regeneration

- **GIVEN** a previous publish run wrote a page for proposal `abc-123` under `accepted-knowledge/docs_note/abc-123.md`
- **AND** the corresponding accepted decision has been removed from `.scryrs/accepted/`
- **WHEN** the adapter runs again
- **THEN** `accepted-knowledge/docs_note/abc-123.md` no longer exists
- **AND** other accepted-knowledge pages remain

#### Scenario: Repeated publish runs are byte-stable

- **GIVEN** the same set of accepted Markdown decisions and the same docs root
- **WHEN** the adapter publishes twice
- **THEN** the second run produces byte-identical page files and `_nav.json` compared to the first run

### Requirement: Rspress frontmatter is adapter-owned and does not leak into core contracts

The Rspress adapter SHALL add Rspress-specific frontmatter (`title`, `sidebar_label`) to published pages within the adapter only. It SHALL NOT add new fields, variants, or assumptions to `ProposalReviewDecision`, `ProposalDocument`, `ProposedContent`, `ProposalTargetType`, or any `scryrs-types` contract.

#### Scenario: Pages carry Rspress frontmatter

- **GIVEN** an accepted Markdown decision with `proposalId` and `targetType`
- **WHEN** the adapter writes a page
- **THEN** the page begins with YAML frontmatter containing `title` and `sidebar_label` derived from `targetType` and `proposalId`
- **AND** the accepted Markdown body follows below the frontmatter

#### Scenario: scryrs-types contracts are unchanged

- **WHEN** the Rspress adapter is implemented
- **THEN** `scryrs-types/src/lib.rs` has no new Rspress-specific fields, variants, or imports
- **AND** `ProposalReviewDecision` carries no frontmatter or route fields

### Requirement: Navigation (_nav.json) is updated deterministically

The adapter SHALL read `.devagent/docs/docs/_nav.json`, strip any existing section where `text == "Accepted Knowledge"`, append a fresh section with per-target-type sub-sections, and write back. The section SHALL only include target types that currently have published artifacts.

#### Scenario: Accepted Knowledge section is appended on first publish

- **GIVEN** `_nav.json` has three hand-authored sections and no "Accepted Knowledge" section
- **WHEN** the adapter publishes accepted docs_note and adr artifacts
- **THEN** `_nav.json` contains four sections: the original three plus "Accepted Knowledge" at the end
- **AND** the "Accepted Knowledge" section contains sub-sections for docs_note and adr with items sorted by `proposalId`

#### Scenario: Accepted Knowledge section is replaced on rerun

- **GIVEN** a previous publish created an "Accepted Knowledge" section with entries A, B, C
- **AND** accepted artifacts now contain only B and D
- **WHEN** the adapter publishes again
- **THEN** the "Accepted Knowledge" section contains only B and D
- **AND** entries A and C are removed
- **AND** hand-authored sections are unchanged

#### Scenario: Empty published set removes the section

- **GIVEN** `_nav.json` contains an existing "Accepted Knowledge" section
- **AND** `.scryrs/accepted/` contains no publishable Markdown decisions
- **WHEN** the adapter publishes
- **THEN** the "Accepted Knowledge" section is removed from `_nav.json`
- **AND** hand-authored sections are preserved

#### Scenario: Missing _nav.json is created from scratch

- **GIVEN** `.devagent/docs/docs/_nav.json` does not exist
- **WHEN** the adapter publishes accepted artifacts
- **THEN** a new `_nav.json` is created containing only the "Accepted Knowledge" section

#### Scenario: Malformed _nav.json fails loudly

- **GIVEN** `_nav.json` contains invalid JSON
- **WHEN** the adapter runs
- **THEN** the run fails with a clear error message
- **AND** no pages are written

### Requirement: Non-Markdown accepted decisions are not published

The Rspress adapter SHALL NOT produce pages or nav entries for accepted `memory_patch` or `semantic_graph_grouping` decisions. It SHALL rely on the Markdown adapter's existing filter rather than re-deriving the non-Markdown skip logic.

#### Scenario: Non-Markdown decisions produce no Rspress output

- **GIVEN** `.scryrs/accepted/` contains one `docs_note` decision and one `memory_patch` decision
- **WHEN** the adapter publishes
- **THEN** only the `docs_note` page is written
- **AND** `_nav.json` contains no entry for the `memory_patch` decision
- **AND** the run does not fail because of the non-Markdown decision

### Requirement: Build verification proves published knowledge in llms surfaces

A verification script SHALL run the Rspress adapter against test fixtures, execute `bun run build` in `.devagent/docs/`, and assert that generated `doc_build/llms.txt` and `doc_build/llms-full.txt` contain the published proposal IDs and valid links.

#### Scenario: llms.txt includes published proposal IDs

- **GIVEN** the adapter published a `docs_note` with proposal ID `abc-123`
- **AND** `bun run build` completed successfully
- **WHEN** the verification script checks `doc_build/llms.txt` and `doc_build/llms-full.txt`
- **THEN** both files contain the substring `abc-123`
- **AND** the files contain a valid link to `/project-docs/accepted-knowledge/docs_note/abc-123.html`

#### Scenario: Verification fails loudly when Bun is missing

- **GIVEN** Bun is not installed or not on PATH
- **WHEN** the verification script runs
- **THEN** it exits with a non-zero status and a clear error message about the missing Bun/Node runtime

### Requirement: Adapter is library-only with machine-readable output

The Rspress adapter SHALL expose a library API that returns structured `PublishEntry` data (path, proposal ID, target type, nav text, nav link) for each published artifact. It SHALL NOT add a standalone CLI binary in this slice.

#### Scenario: publish_accepted_rspress returns structured entries

- **GIVEN** accepted artifacts for `docs_note` and `adr`
- **WHEN** `publish_accepted_rspress(repo_root, docs_root)` is called
- **THEN** the returned `Vec<PublishEntry>` contains one entry per published artifact
- **AND** each entry includes `path`, `proposal_id`, `target_type`, `nav_text`, and `nav_link`

### Requirement: Adapter depends on scryrs-adapter-markdown as upstream boundary

The Rspress adapter SHALL add `scryrs-adapter-markdown` as a Cargo dependency and call `publish_accepted_markdown()` to obtain the accepted-only Markdown output. It SHALL NOT re-implement accepted-artifact parsing, `ProposalReviewDecision` validation, `proposalId` sorting, non-Markdown skip, or Markdown rendering.

#### Scenario: Markdown adapter is the only path to accepted artifacts

- **GIVEN** the Rspress adapter needs to publish accepted knowledge
- **WHEN** it loads publishable artifacts
- **THEN** it calls `publish_accepted_markdown(repo_root, temp_dir)` and iterates the resulting files
- **AND** it never reads `.scryrs/accepted/*.json` directly
- **AND** it never reads `.scryrs/proposals/*.json`