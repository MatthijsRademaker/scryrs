## MODIFIED Requirements

### Requirement: Proposal review commands are registered and discoverable

The CLI SHALL register a grouped `proposals` root command with `list`, `accept`, and `reject` subcommands. The final review command surface SHALL be:

- `scryrs proposals list <PATH> [--state pending|accepted|rejected|all]`
- `scryrs proposals accept <PATH> <ID> --reviewer <NAME> --rationale <TEXT> --decided-at <RFC3339> [--content-file <PATH> | --content-stdin]`
- `scryrs proposals reject <PATH> <ID> --reviewer <NAME> --rationale <TEXT> --decided-at <RFC3339>`

`--content-file` and `--content-stdin` SHALL be optional, mutually exclusive, and available only on the `accept` subcommand. Supplying either flag on `reject` SHALL fail with exit code `2` and a dedicated error.
The command group SHALL appear in human-readable help, machine-readable `--help-json`, snapshots, and CLI documentation.

#### Scenario: Root help exposes grouped review commands

- **WHEN** a caller invokes `scryrs --help`
- **THEN** the output includes a `proposals` command entry
- **AND** the entry describes proposal review operations

#### Scenario: Proposals help exposes subcommands and required review metadata

- **WHEN** a caller invokes `scryrs proposals --help`
- **THEN** the output lists `list`, `accept`, and `reject`
- **AND** the `accept` usage text includes optional `[--content-file <PATH> | --content-stdin]`
- **AND** the `accept` and `reject` usage text requires `--reviewer`, `--rationale`, and `--decided-at`

#### Scenario: Help-json represents the grouped review surface

- **WHEN** a caller invokes `scryrs --help-json`
- **THEN** the JSON surface document contains a `proposals` command entry
- **AND** that entry exposes nested `list`, `accept`, and `reject` subcommands with their arguments and flags
- **AND** the `accept` subcommand includes optional, mutually exclusive `--content-file` and `--content-stdin` flags
- **AND** `surfaceVersion` is `0.17.0`

### Requirement: Accept writes deterministic accepted review decisions

`scryrs proposals accept <PATH> <ID>` SHALL write `.scryrs/accepted/{proposalId}.json` as a valid `ProposalReviewDecision` with outcome `accepted`. The written decision SHALL copy `targetType` from the proposal, copy proposal evidence into `sourceEvidence`, and leave the source proposal file unchanged.

For Markdown-backed target types (`docs_note`, `adr`, `skill`, `debugging_playbook`), the accepted decision SHALL use the reviewed Markdown content when `--content-file` or `--content-stdin` is supplied. When neither override flag is supplied, the decision SHALL copy `proposedContent` into `acceptedContent` unchanged (current behavior).

For structured target types (`memory_patch`, `semantic_graph_grouping`), supplying `--content-file` or `--content-stdin` SHALL fail with exit code `2` and a clear unsupported-target error. No accepted artifact SHALL be written.

#### Scenario: Accept writes a valid accepted decision with content copied from proposal

- **GIVEN** a valid `.scryrs/proposals/{id}.json`
- **WHEN** a caller invokes `scryrs proposals accept <PATH> <ID> --reviewer alice --rationale approved --decided-at 2026-06-28T12:00:00Z` without `--content-file` or `--content-stdin`
- **THEN** the command exits with code `0`
- **AND** `.scryrs/accepted/{id}.json` is created
- **AND** the file deserializes as `ProposalReviewDecision`
- **AND** `outcome` is `accepted`
- **AND** `targetType` matches the proposal `targetType`
- **AND** `acceptedContent` equals the proposal `proposedContent`
- **AND** `sourceEvidence` is non-empty and matches the proposal evidence
- **AND** `.scryrs/proposals/{id}.json` is not mutated

#### Scenario: Accept writes a valid accepted decision with reviewed content from file

- **GIVEN** a valid `.scryrs/proposals/{id}.json` with a Markdown target type (`docs_note`, `adr`, `skill`, or `debugging_playbook`)
- **AND** a file at `reviewed.md` containing non-empty Markdown
- **WHEN** a caller invokes `scryrs proposals accept <PATH> <ID> --content-file reviewed.md --reviewer alice --rationale "edited" --decided-at 2026-06-28T12:00:00Z`
- **THEN** the command exits with code `0`
- **AND** `.scryrs/accepted/{id}.json` is created
- **AND** the file deserializes as `ProposalReviewDecision`
- **AND** `outcome` is `accepted`
- **AND** `targetType` matches the proposal `targetType`
- **AND** `acceptedContent` equals the content of `reviewed.md`
- **AND** `sourceEvidence` is non-empty and matches the proposal evidence
- **AND** `.scryrs/proposals/{id}.json` is not mutated

#### Scenario: Accept writes a valid accepted decision with reviewed content from stdin

- **GIVEN** a valid `.scryrs/proposals/{id}.json` with a Markdown target type
- **AND** non-empty Markdown content is piped on stdin
- **WHEN** a caller invokes `scryrs proposals accept <PATH> <ID> --content-stdin --reviewer alice --rationale "edited" --decided-at 2026-06-28T12:00:00Z`
- **THEN** the command exits with code `0`
- **AND** `.scryrs/accepted/{id}.json` is created
- **AND** `acceptedContent` equals the piped stdin content
- **AND** `.scryrs/proposals/{id}.json` is not mutated

#### Scenario: Structured target override fails loudly

- **GIVEN** a valid `.scryrs/proposals/{id}.json` with a structured target type (`memory_patch` or `semantic_graph_grouping`)
- **WHEN** a caller invokes `scryrs proposals accept <PATH> <ID> --content-file reviewed.md --reviewer alice --rationale approved --decided-at 2026-06-28T12:00:00Z`
- **THEN** the command exits with code `2`
- **AND** stderr reports an unsupported-target-type error
- **AND** no accepted artifact is written under `.scryrs/accepted/`

#### Scenario: Content override flags are mutually exclusive

- **GIVEN** a valid `.scryrs/proposals/{id}.json` with a Markdown target type
- **WHEN** a caller supplies both `--content-file <PATH>` and `--content-stdin` on `accept`
- **THEN** the command exits with code `2`
- **AND** stderr reports that the flags are mutually exclusive
- **AND** no accepted artifact is written

#### Scenario: Content override flags rejected on reject command

- **GIVEN** a valid `.scryrs/proposals/{id}.json`
- **WHEN** a caller supplies `--content-file <PATH>` or `--content-stdin` on `scryrs proposals reject`
- **THEN** the command exits with code `2`
- **AND** stderr reports that content override flags are not supported on the reject subcommand
- **AND** no rejected artifact is written

#### Scenario: Accept preserves source-of-truth boundaries

- **GIVEN** a repository with existing `.devagent/docs/`, `.scryrs/graph.json`, and `.scryrs/routes.json`
- **WHEN** a caller accepts a proposal, with or without content override flags
- **THEN** no files under those protected paths are created, modified, or deleted

### Requirement: Repeated review commands are idempotent only for byte-identical results

Repeated `accept` or `reject` operations for the same proposal ID SHALL succeed only when the resulting review-decision artifact would be byte-identical to the existing artifact. Opposite-outcome attempts, conflicting same-ID terminal states, or same-outcome reruns that would change artifact bytes — including changed reviewed Markdown content — SHALL fail with exit code `2` and SHALL NOT overwrite the existing decision.

#### Scenario: Repeated identical accept is deterministic

- **GIVEN** `.scryrs/accepted/{id}.json` already exists from a prior successful accept
- **AND** rerunning `accept` with the same proposal and the same review metadata (including identical `--content-file` content or identical stdin) would produce byte-identical JSON
- **WHEN** a caller reruns `scryrs proposals accept <PATH> <ID> --reviewer alice --rationale approved --decided-at 2026-06-28T12:00:00Z`
- **THEN** the command exits with code `0`
- **AND** `.scryrs/accepted/{id}.json` remains byte-identical

#### Scenario: Same-outcome overwrite with different reviewed content fails loudly

- **GIVEN** `.scryrs/accepted/{id}.json` already exists with overridden Markdown content
- **AND** rerunning `accept` with the same metadata but different `--content-file` bytes would produce different serialized JSON
- **WHEN** a caller reruns `scryrs proposals accept <PATH> <ID> --content-file different.md --reviewer alice --rationale approved --decided-at 2026-06-28T12:00:00Z`
- **THEN** the command exits with code `2`
- **AND** the existing accepted decision is not overwritten

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

### Requirement: Listing validates review decisions with per-target-type content rules

`scryrs proposals list` SHALL validate every encountered review-decision artifact against its source proposal. For Markdown-backed target types (`docs_note`, `adr`, `skill`, `debugging_playbook`), the accepted decision SHALL match the proposal on `proposalId`, `targetType`, and `sourceEvidence`, but `acceptedContent` MAY differ from `proposedContent`. For structured target types (`memory_patch`, `semantic_graph_grouping`), `acceptedContent` SHALL equal `proposedContent`.

#### Scenario: List tolerates reviewed Markdown content that differs from proposal content

- **GIVEN** a valid `.scryrs/proposals/{id}.json` with a Markdown target type
- **AND** `.scryrs/accepted/{id}.json` exists with `acceptedContent` that differs from the proposal `proposedContent`
- **AND** the accepted decision matches the proposal on `proposalId`, `targetType`, and `sourceEvidence`
- **WHEN** a caller invokes `scryrs proposals list <PATH>`
- **THEN** the command exits with code `0`
- **AND** the proposal is listed with state `accepted`

#### Scenario: List rejects structured accepted content that differs from proposal content

- **GIVEN** a valid `.scryrs/proposals/{id}.json` with a structured target type (`memory_patch` or `semantic_graph_grouping`)
- **AND** `.scryrs/accepted/{id}.json` exists with `acceptedContent` that differs from the proposal `proposedContent`
- **WHEN** a caller invokes `scryrs proposals list <PATH>`
- **THEN** the command exits with code `2`
- **AND** stderr reports that `acceptedContent` does not match the proposal