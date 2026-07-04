# proposal-review-cli Specification

## Purpose
TBD - created by archiving change task-6548f8e5-91fe-433e-84fe-68bfa926b90d. Update Purpose after archive.
## Requirements
### Requirement: Proposal review commands are registered and discoverable

The CLI SHALL register a grouped `proposals` root command with `list`, `accept`, and `reject` subcommands. The final review command surface SHALL be:

- `scryrs proposals list <PATH> [--state pending|accepted|rejected|all]`
- `scryrs proposals accept <PATH> <ID> --reviewer <NAME> --rationale <TEXT> --decided-at <RFC3339>`
- `scryrs proposals reject <PATH> <ID> --reviewer <NAME> --rationale <TEXT> --decided-at <RFC3339>`

The command group SHALL appear in human-readable help, machine-readable `--help-json`, snapshots, and CLI documentation.

#### Scenario: Root help exposes grouped review commands

- **WHEN** a caller invokes `scryrs --help`
- **THEN** the output includes a `proposals` command entry
- **AND** the entry describes proposal review operations

#### Scenario: Proposals help exposes subcommands and required review metadata

- **WHEN** a caller invokes `scryrs proposals --help`
- **THEN** the output lists `list`, `accept`, and `reject`
- **AND** the `accept` and `reject` usage text requires `--reviewer`, `--rationale`, and `--decided-at`

#### Scenario: Help-json represents the grouped review surface

- **WHEN** a caller invokes `scryrs --help-json`
- **THEN** the JSON surface document contains a `proposals` command entry
- **AND** that entry exposes nested `list`, `accept`, and `reject` subcommands with their arguments and flags
- **AND** `surfaceVersion` is `0.9.0`

### Requirement: Proposal state listing is deterministic and machine-readable

`scryrs proposals list <PATH>` SHALL inspect `.scryrs/proposals/`, `.scryrs/accepted/`, and `.scryrs/rejected/` and emit a deterministic JSON array sorted by `proposalId` ascending. Each listed row SHALL include `proposalId`, `title`, `targetType`, `createdAt`, and `state`, where `state` is `pending`, `accepted`, or `rejected`.

#### Scenario: List returns pending and reviewed states

- **GIVEN** valid proposal inbox artifacts under `.scryrs/proposals/`
- **AND** some proposal IDs also have matching reviewed artifacts under `.scryrs/accepted/` or `.scryrs/rejected/`
- **WHEN** a caller invokes `scryrs proposals list <PATH>`
- **THEN** the command exits with code `0`
- **AND** stdout is a JSON array sorted by `proposalId`
- **AND** each row reports the correct proposal `state`

#### Scenario: State filter narrows the output

- **GIVEN** a repository containing pending, accepted, and rejected proposals
- **WHEN** a caller invokes `scryrs proposals list <PATH> --state accepted`
- **THEN** the JSON output contains only rows whose `state` is `accepted`

#### Scenario: Invalid state filter fails loudly

- **WHEN** a caller invokes `scryrs proposals list <PATH> --state archived`
- **THEN** the command exits with code `2`
- **AND** stderr contains a usage error for the invalid filter

#### Scenario: Conflicting terminal state fails loudly

- **GIVEN** `.scryrs/accepted/{id}.json` and `.scryrs/rejected/{id}.json` both exist for the same proposal ID
- **WHEN** a caller invokes `scryrs proposals list <PATH>`
- **THEN** the command exits with code `2`
- **AND** stderr reports a conflicting terminal state
- **AND** stdout does not emit a partial success result

### Requirement: Proposal review commands require valid source proposals

Before `accept` or `reject` writes any decision artifact, the source proposal SHALL exist at `.scryrs/proposals/{id}.json`, deserialize as `ProposalDocument`, and pass `ProposalDocument::validate()`.

#### Scenario: Unknown proposal ID fails loudly

- **GIVEN** no `.scryrs/proposals/{id}.json` file exists for the requested ID
- **WHEN** a caller invokes `scryrs proposals accept <PATH> <ID> --reviewer alice --rationale approved --decided-at 2026-06-28T12:00:00Z`
- **THEN** the command exits with code `2`
- **AND** stderr reports an unknown proposal ID
- **AND** no reviewed artifact is created

#### Scenario: Malformed proposal document fails loudly

- **GIVEN** `.scryrs/proposals/{id}.json` exists but contains malformed JSON, a wrong schema version, or semantically invalid proposal content
- **WHEN** a caller invokes `scryrs proposals reject <PATH> <ID> --reviewer alice --rationale off-scope --decided-at 2026-06-28T12:00:00Z`
- **THEN** the command exits with code `2`
- **AND** stderr reports the invalid proposal document
- **AND** no reviewed artifact is created

#### Scenario: Missing required review metadata fails loudly

- **WHEN** a caller omits `--reviewer`, `--rationale`, or `--decided-at` from `accept` or `reject`
- **THEN** the command exits with code `2`
- **AND** stderr reports the missing required argument and usage text
- **AND** no reviewed artifact is created

### Requirement: Accept writes deterministic accepted review decisions

`scryrs proposals accept <PATH> <ID>` SHALL write `.scryrs/accepted/{proposalId}.json` as a valid `ProposalReviewDecision` with outcome `accepted`. The written decision SHALL copy `targetType` from the proposal, copy `proposedContent` into `acceptedContent`, and copy proposal evidence into `sourceEvidence`. The source proposal file SHALL remain unchanged.

#### Scenario: Accept writes a valid accepted decision

- **GIVEN** a valid `.scryrs/proposals/{id}.json`
- **WHEN** a caller invokes `scryrs proposals accept <PATH> <ID> --reviewer alice --rationale approved --decided-at 2026-06-28T12:00:00Z`
- **THEN** the command exits with code `0`
- **AND** `.scryrs/accepted/{id}.json` is created
- **AND** the file deserializes as `ProposalReviewDecision`
- **AND** `outcome` is `accepted`
- **AND** `targetType` matches the proposal `targetType`
- **AND** `acceptedContent` equals the proposal `proposedContent`
- **AND** `sourceEvidence` is non-empty and matches the proposal evidence
- **AND** `.scryrs/proposals/{id}.json` is not mutated

#### Scenario: Accept preserves source-of-truth boundaries

- **GIVEN** a repository with existing `.devagent/docs/`, `.scryrs/graph.json`, and `.scryrs/routes.json`
- **WHEN** a caller accepts a proposal
- **THEN** no files under those protected paths are created, modified, or deleted

### Requirement: Reject writes deterministic rejected review decisions

`scryrs proposals reject <PATH> <ID>` SHALL write `.scryrs/rejected/{proposalId}.json` as a valid `ProposalReviewDecision` with outcome `rejected`. The written decision SHALL omit `targetType` and `acceptedContent`, copy proposal evidence into `sourceEvidence`, and leave the source proposal file unchanged.

#### Scenario: Reject writes a valid rejected decision

- **GIVEN** a valid `.scryrs/proposals/{id}.json`
- **WHEN** a caller invokes `scryrs proposals reject <PATH> <ID> --reviewer alice --rationale off-scope --decided-at 2026-06-28T12:00:00Z`
- **THEN** the command exits with code `0`
- **AND** `.scryrs/rejected/{id}.json` is created
- **AND** the file deserializes as `ProposalReviewDecision`
- **AND** `outcome` is `rejected`
- **AND** `targetType` is omitted
- **AND** `acceptedContent` is omitted
- **AND** `sourceEvidence` is non-empty and matches the proposal evidence
- **AND** `.scryrs/proposals/{id}.json` is not mutated

#### Scenario: Reject does not delete the proposal inbox artifact

- **GIVEN** a valid `.scryrs/proposals/{id}.json`
- **WHEN** a caller rejects that proposal
- **THEN** `.scryrs/proposals/{id}.json` still exists after the command completes

### Requirement: Repeated review commands are idempotent only for byte-identical results

Repeated `accept` or `reject` operations for the same proposal ID SHALL succeed only when the resulting review-decision artifact would be byte-identical to the existing artifact. Opposite-outcome attempts, conflicting same-ID terminal states, or same-outcome reruns that would change artifact bytes SHALL fail with exit code `2` and SHALL NOT overwrite the existing decision.

#### Scenario: Repeated identical accept is deterministic

- **GIVEN** `.scryrs/accepted/{id}.json` already exists from a prior successful accept
- **AND** rerunning `accept` with the same proposal and the same review metadata would produce byte-identical JSON
- **WHEN** a caller reruns `scryrs proposals accept <PATH> <ID> --reviewer alice --rationale approved --decided-at 2026-06-28T12:00:00Z`
- **THEN** the command exits with code `0`
- **AND** `.scryrs/accepted/{id}.json` remains byte-identical

#### Scenario: Opposite-outcome review fails loudly

- **GIVEN** `.scryrs/accepted/{id}.json` already exists
- **WHEN** a caller invokes `scryrs proposals reject <PATH> <ID> --reviewer alice --rationale off-scope --decided-at 2026-06-28T12:00:00Z`
- **THEN** the command exits with code `2`
- **AND** stderr reports a conflicting terminal decision
- **AND** no rejected artifact is written

#### Scenario: Same-outcome overwrite with different bytes fails loudly

- **GIVEN** `.scryrs/rejected/{id}.json` already exists
- **AND** rerunning `reject` with different review metadata would change the serialized artifact bytes
- **WHEN** a caller reruns `scryrs proposals reject <PATH> <ID> ...`
- **THEN** the command exits with code `2`
- **AND** the existing reviewed artifact is not overwritten

### Requirement: Exit codes and non-mutating review boundaries follow the CLI contract

Proposal review commands SHALL use exit code `0` for success, exit code `2` for usage or input errors, and exit code `1` for serialization or filesystem write failures. Review commands SHALL NOT create, modify, or delete `.devagent/docs/`, `.scryrs/graph.json`, `.scryrs/routes.json`, or files under `.scryrs/proposals/`.

#### Scenario: Filesystem write failure exits 1

- **GIVEN** a valid proposal and valid review metadata
- **AND** the reviewed-artifact directory cannot be written
- **WHEN** a caller invokes `scryrs proposals accept <PATH> <ID> ...`
- **THEN** the command exits with code `1`
- **AND** stderr reports the write failure

#### Scenario: Review commands do not mutate protected paths

- **GIVEN** a repository with existing docs, graph, routes, and proposal inbox files
- **WHEN** a caller runs `list`, `accept`, or `reject`
- **THEN** none of those protected files are created, modified, or deleted as a side effect of review operations

### Requirement: Proposal review commands do not trigger publish side effects

`scryrs proposals list`, `scryrs proposals accept`, and `scryrs proposals reject` SHALL remain review and ledger operations only. They SHALL NOT write generic Markdown publish output, SHALL NOT update Rspress `accepted-knowledge/` pages, and SHALL NOT modify Rspress `_nav.json`. Publication requires a separate explicit `scryrs publish ...` invocation.

#### Scenario: Accept remains ledger-only

- **GIVEN** a valid pending proposal exists under `.scryrs/proposals/`
- **WHEN** a caller invokes `scryrs proposals accept <PATH> <ID> ...`
- **THEN** the command writes only the accepted review decision under `.scryrs/accepted/`
- **AND** no generic Markdown publish output is created
- **AND** no Rspress docs output is created or updated

#### Scenario: Reject remains ledger-only

- **GIVEN** a valid pending proposal exists under `.scryrs/proposals/`
- **WHEN** a caller invokes `scryrs proposals reject <PATH> <ID> ...`
- **THEN** the command writes only the rejected review decision under `.scryrs/rejected/`
- **AND** no generic Markdown publish output is created
- **AND** no Rspress docs output is created or updated

