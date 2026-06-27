## ADDED Requirements

### Requirement: Proposal generation command exists and is discoverable

The system SHALL register `scryrs propose <PATH>` as a CLI command. The command SHALL accept a single positional PATH argument pointing to the repository root. The command SHALL appear in human-readable `--help` output, machine-readable `--help-json` output, and the repository README.

#### Scenario: Command is registered in dispatch

- **GIVEN** the CLI is built with the `curator` feature enabled (default)
- **WHEN** a caller invokes `scryrs propose <PATH>`
- **THEN** the command is recognized by the dispatch system
- **AND** the command does not produce an "unknown command" error

#### Scenario: Command appears in --help output

- **GIVEN** a build with the `curator` feature
- **WHEN** a caller invokes `scryrs --help`
- **THEN** the output includes a `propose` command entry with description and usage

#### Scenario: Command appears in --help-json output

- **GIVEN** a build with the `curator` feature
- **WHEN** a caller invokes `scryrs --help-json`
- **THEN** the JSON output includes a `propose` entry in alphabetical sorted position among commands
- **AND** the entry includes a PATH argument with `required: true`

#### Scenario: Command is documented in README

- **GIVEN** the repository README
- **WHEN** a reader inspects the supported commands and current limitations sections
- **THEN** `propose` is listed as a supported command
- **AND** `propose` is no longer listed as unsupported

### Requirement: Required inputs are loaded and validated

The command SHALL load `.scryrs/hotspots.json` and `.scryrs/graph.json` from the repository root. Missing or malformed required artifacts SHALL cause the command to exit with code 2, emit a command-specific error message on stderr, and write no proposal artifacts.

#### Scenario: Missing hotspots artifact fails

- **GIVEN** a repository root where `.scryrs/hotspots.json` does not exist
- **WHEN** a caller runs `scryrs propose <PATH>`
- **THEN** the command exits with code 2
- **AND** stderr contains `scryrs propose: hotspots artifact not found at ...`
- **AND** no `.scryrs/proposals/` files are written

#### Scenario: Malformed hotspots artifact fails

- **GIVEN** a repository root where `.scryrs/hotspots.json` contains invalid JSON
- **WHEN** a caller runs `scryrs propose <PATH>`
- **THEN** the command exits with code 2
- **AND** stderr contains `scryrs propose: malformed hotspots file: ...`
- **AND** no `.scryrs/proposals/` files are written

#### Scenario: Missing graph artifact fails

- **GIVEN** a repository root with valid `.scryrs/hotspots.json` but no `.scryrs/graph.json`
- **WHEN** a caller runs `scryrs propose <PATH>`
- **THEN** the command exits with code 2
- **AND** stderr contains `scryrs propose: graph artifact not found at ...`
- **AND** no `.scryrs/proposals/` files are written

#### Scenario: Malformed graph artifact fails

- **GIVEN** a repository root where `.scryrs/graph.json` contains invalid JSON
- **WHEN** a caller runs `scryrs propose <PATH>`
- **THEN** the command exits with code 2
- **AND** stderr contains `scryrs propose: malformed graph file: ...`
- **AND** no `.scryrs/proposals/` files are written

#### Scenario: Missing PATH argument fails

- **GIVEN** the `scryrs propose` command is invoked without a PATH argument
- **WHEN** the command dispatcher processes the invocation
- **THEN** the command exits with code 2
- **AND** stderr follows the contract three-line format: `scryrs propose: missing required PATH argument`, `Usage: scryrs propose <PATH>`, `See \`scryrs --help\``

### Requirement: Determistic proposal generation from evidence

The command SHALL generate zero or more `ProposalDocument` files under `.scryrs/proposals/`. Each file SHALL validate as a `ProposalDocument` with non-empty rationale, non-empty proposed content, and non-empty evidence using the existing `EvidenceLink` vocabulary. Given identical hotspot and graph inputs, repeated runs SHALL produce the same proposal set with stable content-addressed IDs.

#### Scenario: Every generated file validates as ProposalDocument

- **GIVEN** valid hotspot and graph inputs
- **WHEN** the command generates proposals
- **THEN** every `.scryrs/proposals/{id}.json` file deserializes as a `ProposalDocument`
- **AND** each has non-empty `rationale`
- **AND** each has non-empty `proposedContent`
- **AND** each has non-empty `evidence`
- **AND** each passes `ProposalDocument::validate()`

#### Scenario: Repeated runs produce identical file identities

- **GIVEN** identical `.scryrs/hotspots.json` and `.scryrs/graph.json` content
- **WHEN** the command is run twice on the same repository
- **THEN** the set of `.scryrs/proposals/{id}.json` filenames is identical across runs
- **AND** each proposal's `id` field is stable across runs

#### Scenario: Content-addressed IDs are derived from targetType and proposedContent

- **GIVEN** a generated proposal
- **WHEN** its `id` is inspected
- **THEN** the `id` is a 64-character hex string
- **AND** the `id` equals `ProposalDocument::compute_id(target_type, proposed_content)`

#### Scenario: created_at is derived from hotspot artifact

- **GIVEN** valid hotspot and graph inputs
- **WHEN** the command generates proposals
- **THEN** each proposal's `createdAt` field equals the hotspot report's `generatedAt` value
- **AND** `createdAt` is a valid RFC 3339 timestamp

### Requirement: V1 deterministic rules map evidence to target types

The curator engine SHALL apply concrete, testable heuristics to generate proposals for each supported target type. No `debugging_playbook` proposals SHALL be generated. Rules SHALL be deterministic: the same inputs always produce the same proposals.

#### Scenario: docs_note — every hotspot entry

- **GIVEN** a hotspot report with N entries
- **WHEN** the curator engine processes them
- **THEN** exactly N `docs_note` proposals are generated (one per entry)
- **AND** each proposal cites the entry's subject and score
- **AND** each proposal's evidence links use `sourceKind = hotspot_subject`

#### Scenario: skill — hotspot entries with Failure outcomes

- **GIVEN** hotspot entries where some have `outcome.Failure > 0` and some have only `Success` outcomes
- **WHEN** the curator engine processes them
- **THEN** `skill` proposals are generated only for entries with at least one `Failure` outcome event
- **AND** entries without any `Failure` outcomes do not generate skill proposals

#### Scenario: memory_patch — high-failure-ratio entries

- **GIVEN** a hotspot entry with `score >= 4` and failure-ratio ≥ 0.5 (Failure events ≥ 50% of total outcome events)
- **WHEN** the curator engine processes it
- **THEN** a `memory_patch` proposal is generated with structured JSON `proposedContent`
- **AND** entries with `score < 4` or failure-ratio < 0.5 do not generate memory_patch proposals

#### Scenario: adr — cross-kind hotspot clusters

- **GIVEN** hotspot entries for the same subject string appearing across ≥2 distinct `subjectKind` values with aggregate score ≥ 10
- **WHEN** the curator engine processes the hotspot report
- **THEN** an `adr` proposal is generated for each such cluster
- **AND** the proposal cites evidence from all contributing hotspot entries
- **AND** clusters with only one subject kind or aggregate score < 10 do not generate ADR proposals

#### Scenario: semantic_graph_grouping — cross-kind graph node families

- **GIVEN** graph nodes sharing the same subject stem (e.g., `auth`) across ≥2 distinct subject kinds with at least one shared hotspot-backed evidence link
- **WHEN** the curator engine processes graph nodes and hotspot evidence
- **THEN** a `semantic_graph_grouping` proposal is generated with `targetType = semantic_graph_grouping`
- **AND** `proposedContent.sourceNodeIds` lists the exact source graph node IDs
- **AND** `proposedContent.targetGroupNodeId` is set to `domain_term:<subject>`
- **AND** `proposedContent.targetGroupLabel` is set to the subject stem

#### Scenario: debugging_playbook is never generated

- **GIVEN** any combination of hotspot and graph inputs
- **WHEN** the curator engine generates proposals
- **THEN** no proposal with `targetType = debugging_playbook` is produced

### Requirement: Semantic graph grouping proposals preserve source identities and remain non-authoritative

`semantic_graph_grouping` proposals SHALL include a non-empty `sourceNodeIds` list containing exact source graph node IDs in `{subjectKind}:{subject}` format. The command SHALL NOT rewrite `.scryrs/graph.json` or `.scryrs/routes.json` as a result of generating grouping proposals.

#### Scenario: Grouping proposal cites exact source node IDs

- **GIVEN** graph nodes `file:auth`, `search:auth`, and `symbol:auth`
- **WHEN** a semantic grouping proposal is generated
- **THEN** `proposedContent.sourceNodeIds` contains `["file:auth", "search:auth", "symbol:auth"]`
- **AND** the grouping does not modify `.scryrs/graph.json`

#### Scenario: Grouping proposal without source nodes is invalid

- **GIVEN** the curator engine attempts to create a grouping proposal
- **WHEN** the proposal is validated
- **THEN** proposals with empty `sourceNodeIds` are rejected by `ProposalDocument::validate()`

### Requirement: Source-of-truth artifacts are never mutated

The command SHALL write proposal artifacts only under `.scryrs/proposals/`. The command SHALL NOT create or modify `.scryrs/graph.json`, `.scryrs/routes.json`, `.devagent/docs/`, or any source-of-truth ADR, skill, or memory files.

#### Scenario: Graph artifact is unmodified

- **GIVEN** a repository with existing `.scryrs/graph.json`
- **WHEN** the proposal generation command runs
- **THEN** `.scryrs/graph.json` content is unmodified (same mtime or identical bytes)

#### Scenario: Route artifact is unmodified

- **GIVEN** a repository with existing `.scryrs/routes.json`
- **WHEN** the proposal generation command runs
- **THEN** `.scryrs/routes.json` content is unmodified

#### Scenario: Docs are unmodified

- **GIVEN** a repository with existing `.devagent/docs/` content
- **WHEN** the proposal generation command runs
- **THEN** no files under `.devagent/docs/` are created, modified, or deleted

#### Scenario: Proposal inbox is the only write target

- **GIVEN** a successful proposal generation run
- **WHEN** the filesystem is inspected
- **THEN** the only new or modified files are under `.scryrs/proposals/`

### Requirement: Upsert-only semantics with no automatic stale cleanup

The command SHALL write or overwrite `.scryrs/proposals/{id}.json` files for each currently-generated candidate. The command SHALL NOT remove proposal files that are no longer generated from current inputs. When evidence or rationale changes but `proposedContent` stays the same (same content-addressed ID), the existing file SHALL be overwritten with updated fields.

#### Scenario: Repeated runs with same inputs overwrite same files

- **GIVEN** a successful first run that writes proposal files
- **WHEN** the command runs again with identical inputs
- **THEN** the same `.scryrs/proposals/{id}.json` files are overwritten
- **AND** no additional files are created beyond those from the first run

#### Scenario: Stale proposals are not removed

- **GIVEN** a previous run wrote a proposal file that is no longer generated under current inputs
- **WHEN** the command runs again
- **THEN** the stale proposal file remains on disk
- **AND** the command does not emit errors or warnings about stale files

#### Scenario: Changed evidence overwrites same file when content is unchanged

- **GIVEN** a proposal file exists from a previous run
- **AND** new hotspot inputs change the evidence or rationale but produce the same `proposedContent`
- **WHEN** the command runs
- **THEN** the existing file is overwritten with updated evidence and rationale
- **AND** the filename (derived from the content-addressed ID) is unchanged

### Requirement: Feature gating and error handling follow CLI contract patterns

The command SHALL be gated behind `#[cfg(feature = "curator")]` matching the existing pattern in `graph.rs` and `route.rs`. Exit codes SHALL follow the established contract: 0 for success, 1 for I/O failures, 2 for usage/input errors.

#### Scenario: Command is feature-gated

- **GIVEN** a build without the `curator` feature
- **WHEN** a caller invokes `scryrs propose .`
- **THEN** the command produces an "unknown command" error

#### Scenario: Exit code 0 on success

- **GIVEN** valid hotspot and graph inputs
- **WHEN** the command runs successfully and writes proposal files
- **THEN** the exit code is 0

#### Scenario: Exit code 1 on I/O failure

- **GIVEN** valid inputs but the `.scryrs/proposals/` directory cannot be written to
- **WHEN** the command runs
- **THEN** the exit code is 1

#### Scenario: Exit code 2 on usage or input errors

- **GIVEN** missing or malformed required artifacts
- **WHEN** the command runs
- **THEN** the exit code is 2

#### Scenario: Error messages follow contract format

- **GIVEN** any error condition
- **WHEN** the command emits an error on stderr
- **THEN** the first line begins with `scryrs propose:`
- **AND** the error output follows the three-line contract format (error, usage, `See \`scryrs --help\``) when applicable
