## ADDED Requirements

### Requirement: Proposal review commands are discoverable and always available

The system SHALL register a `scryrs proposals` namespace with `list`, `accept`, and `reject` subcommands. These review commands SHALL remain available independently of the `curator` feature, and the chosen syntax SHALL be documented in human help, machine-readable help, CLI docs, proposals docs, and README command references.

#### Scenario: Review commands are recognized without curator

- **GIVEN** a build where `scryrs propose` may be feature-gated by `curator`
- **WHEN** a caller invokes `scryrs proposals list <PATH>`
- **THEN** the dispatcher recognizes the `proposals` namespace
- **AND** the command does not fail as an unknown command merely because `curator` is disabled

#### Scenario: Help surfaces document the review namespace

- **WHEN** a caller invokes `scryrs --help`
- **THEN** the output documents `scryrs proposals list <PATH> [--state pending|accepted|rejected|all]`
- **AND** the output documents `scryrs proposals accept <PATH> <ID> --reviewer <NAME> --rationale <TEXT> --decided-at <RFC3339>`
- **AND** the output documents `scryrs proposals reject <PATH> <ID> --reviewer <NAME> --rationale <TEXT> --decided-at <RFC3339>`

#### Scenario: Machine-readable help reflects the additive surface

- **WHEN** a caller invokes `scryrs --help-json`
- **THEN** the machine-readable surface includes the `proposals` review namespace with its `list`, `accept`, and `reject` argument contracts
- **AND** the top-level `surfaceVersion` is `0.9.0`

#### Scenario: Repository docs distinguish generation from review

- **GIVEN** the repository README, CLI docs, and proposals docs
- **WHEN** a reader inspects proposal-related commands
- **THEN** `scryrs propose` is described as proposal generation
- **AND** `scryrs proposals ...` is described as proposal review
- **AND** the review commands' argument requirements are documented consistently across those docs

### Requirement: Proposal state listing is deterministic and explicit

`scryrs proposals list <PATH>` SHALL emit a deterministic JSON envelope with `command`, `schemaVersion`, and `proposals` fields. `command` SHALL equal `proposals list`, `schemaVersion` SHALL equal `0.9.0`, and `proposals` SHALL be sorted by ascending proposal ID. The command SHALL accept `--state pending|accepted|rejected|all` and default that filter to `all`.

Each proposal-backed row SHALL include `id`, `title`, `targetType`, `createdAt`, and `state`. Accepted and rejected rows SHALL also include `reviewer` and `decidedAt`. Orphan decision rows SHALL surface as distinct `orphan_accepted` or `orphan_rejected` states with the decision metadata needed to identify the orphaned evidence.

#### Scenario: Default listing returns all states in deterministic order

- **GIVEN** valid proposal inbox artifacts with a mix of pending, accepted, and rejected review state
- **WHEN** a caller runs `scryrs proposals list <PATH>` without `--state`
- **THEN** stdout is a deterministic JSON envelope
- **AND** `command` is `proposals list`
- **AND** `schemaVersion` is `0.9.0`
- **AND** the `proposals` array is sorted by ascending proposal ID
- **AND** rows show `pending`, `accepted`, and `rejected` states distinctly

#### Scenario: Exact-state filters return only proposal-backed rows in that state

- **GIVEN** valid proposal inbox artifacts with multiple review states present
- **WHEN** a caller runs `scryrs proposals list <PATH> --state accepted`
- **THEN** the JSON envelope includes only proposal-backed rows whose state is `accepted`
- **AND** pending, rejected, and orphan rows are omitted

#### Scenario: Orphan decisions are surfaced when listing all states

- **GIVEN** `.scryrs/accepted/{id}.json` or `.scryrs/rejected/{id}.json` exists without a matching `.scryrs/proposals/{id}.json`
- **WHEN** a caller runs `scryrs proposals list <PATH> --state all`
- **THEN** the orphan appears in the JSON output with state `orphan_accepted` or `orphan_rejected`
- **AND** the orphan is surfaced instead of being silently skipped

#### Scenario: Conflicting or invalid review evidence fails loudly

- **GIVEN** a proposal ID with both accepted and rejected decision artifacts, or a malformed proposal/decision artifact
- **WHEN** a caller runs `scryrs proposals list <PATH>`
- **THEN** the command exits with code `2`
- **AND** stderr reports a command-specific error instead of silently classifying the invalid state

### Requirement: Accept and reject write durable review decisions from validated proposals

`scryrs proposals accept <PATH> <ID> --reviewer <NAME> --rationale <TEXT> --decided-at <RFC3339>` and `scryrs proposals reject <PATH> <ID> --reviewer <NAME> --rationale <TEXT> --decided-at <RFC3339>` SHALL load `.scryrs/proposals/{ID}.json`, deserialize it as `ProposalDocument`, validate it, enforce filename/id consistency, enforce computed content-address consistency, and then write a valid `ProposalReviewDecision` artifact.

For `accept`, the command SHALL write `.scryrs/accepted/{ID}.json` with `outcome = accepted`, copy proposal `targetType` into `targetType`, copy proposal `proposedContent` into `acceptedContent`, and copy proposal `evidence` into `sourceEvidence`.

For `reject`, the command SHALL write `.scryrs/rejected/{ID}.json` with `outcome = rejected`, copy proposal `evidence` into `sourceEvidence`, and omit `targetType` plus `acceptedContent`.

#### Scenario: Accept writes a valid accepted decision

- **GIVEN** `.scryrs/proposals/<ID>.json` exists, deserializes as a valid `ProposalDocument`, and its filename stem plus computed content address both match `<ID>`
- **WHEN** a caller runs `scryrs proposals accept <PATH> <ID> --reviewer <NAME> --rationale <TEXT> --decided-at <RFC3339>`
- **THEN** scryrs writes `.scryrs/accepted/<ID>.json`
- **AND** the written artifact validates as `ProposalReviewDecision`
- **AND** `outcome` is `accepted`
- **AND** `targetType` matches the proposal `targetType`
- **AND** `acceptedContent` matches the proposal `proposedContent`
- **AND** `sourceEvidence` matches the proposal `evidence`

#### Scenario: Reject writes a valid rejected decision

- **GIVEN** `.scryrs/proposals/<ID>.json` exists, deserializes as a valid `ProposalDocument`, and its filename stem plus computed content address both match `<ID>`
- **WHEN** a caller runs `scryrs proposals reject <PATH> <ID> --reviewer <NAME> --rationale <TEXT> --decided-at <RFC3339>`
- **THEN** scryrs writes `.scryrs/rejected/<ID>.json`
- **AND** the written artifact validates as `ProposalReviewDecision`
- **AND** `outcome` is `rejected`
- **AND** `sourceEvidence` matches the proposal `evidence`
- **AND** the rejected artifact does not require accepted-content fields to be present

#### Scenario: Existing decision artifacts prevent overwrite

- **GIVEN** `.scryrs/accepted/<ID>.json` or `.scryrs/rejected/<ID>.json` already exists for the proposal ID
- **WHEN** a caller runs `scryrs proposals accept ...` or `scryrs proposals reject ...` for that ID
- **THEN** the command exits with code `2`
- **AND** no decision artifact is overwritten

### Requirement: Review commands fail loudly with command-specific usage diagnostics

Usage and input failures for `list`, `accept`, and `reject` SHALL follow the established three-line stderr contract: a command-specific error line, a command-specific `Usage:` line, and `See \`scryrs --help\``. These failures SHALL exit with code `2`.

#### Scenario: Missing required arguments produce command-specific usage

- **WHEN** a caller omits required arguments for `scryrs proposals accept` or `scryrs proposals reject`
- **THEN** stderr contains three lines
- **AND** line 1 begins with `scryrs proposals accept:` or `scryrs proposals reject:`
- **AND** line 2 is the command-specific `Usage:` line for that subcommand
- **AND** line 3 is `See \`scryrs --help\``
- **AND** the process exits with code `2`

#### Scenario: Unknown proposal IDs fail loudly

- **GIVEN** no `.scryrs/proposals/<ID>.json` exists for the requested ID
- **WHEN** a caller runs `scryrs proposals accept ...` or `scryrs proposals reject ...`
- **THEN** the command exits with code `2`
- **AND** stderr reports an unknown proposal ID using the three-line command-specific usage format

#### Scenario: Invalid proposal inputs fail before writing decisions

- **GIVEN** the proposal JSON is malformed, fails `ProposalDocument` validation, has a filename/id mismatch, has a computed content-address mismatch, or `--decided-at` is not a valid RFC 3339 timestamp
- **WHEN** a caller runs `scryrs proposals accept ...` or `scryrs proposals reject ...`
- **THEN** the command exits with code `2`
- **AND** stderr reports the validation failure using the three-line command-specific usage format
- **AND** no accepted or rejected artifact is written

### Requirement: Review commands preserve proposal inbox artifacts and source-of-truth outputs

Proposal review commands SHALL create only reviewed-evidence artifacts under `.scryrs/accepted/` or `.scryrs/rejected/`. They SHALL NOT delete or mutate `.scryrs/proposals/{id}.json`, `.scryrs/graph.json`, `.scryrs/routes.json`, `.devagent/docs/`, or representative memory/docs source-of-truth paths.

#### Scenario: Accept leaves source-of-truth artifacts untouched

- **GIVEN** a repository with existing `.scryrs/proposals/`, `.scryrs/graph.json`, `.scryrs/routes.json`, and `.devagent/docs/` content
- **WHEN** a caller successfully runs `scryrs proposals accept ...`
- **THEN** the only new or modified artifact is `.scryrs/accepted/<ID>.json`
- **AND** the original proposal inbox artifact remains unchanged
- **AND** graph, routes, docs, and memory truth remain unchanged

#### Scenario: Reject records review evidence without deleting the proposal

- **GIVEN** a repository with a valid proposal inbox artifact
- **WHEN** a caller successfully runs `scryrs proposals reject ...`
- **THEN** scryrs writes `.scryrs/rejected/<ID>.json`
- **AND** `.scryrs/proposals/<ID>.json` is not deleted or mutated
- **AND** no source-of-truth docs, graph, routes, or memory artifacts are modified

#### Scenario: Deterministic review writes remain byte-identical across identical clean fixtures

- **GIVEN** two clean repositories with identical proposal inbox fixtures and the same explicit reviewer metadata
- **WHEN** `scryrs proposals accept ...` or `scryrs proposals reject ...` is run once in each repository
- **THEN** the resulting decision JSON bytes are identical across both runs
- **AND** the success stdout summaries are deterministic for the same inputs
