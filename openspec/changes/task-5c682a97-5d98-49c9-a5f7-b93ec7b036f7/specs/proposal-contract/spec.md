## MODIFIED Requirements

### Requirement: Proposal inbox layout is deterministic and non-authoritative

Proposal artifacts SHALL be stored as individual JSON files under `.scryrs/proposals/`. Each filename stem SHALL equal the proposal document `id`, and each `id` SHALL be a deterministic SHA-256 content address derived from the proposal `targetType` plus the canonical serialized `proposedContent`. Proposal inbox files are review artifacts only and SHALL NOT directly mutate published docs, ADRs, skills, playbooks, memory truth, `.scryrs/graph.json`, or `.scryrs/routes.json`. Accepted and rejected review outcomes SHALL be recorded as separate artifacts under `.scryrs/accepted/` and `.scryrs/rejected/` rather than by mutating proposal inbox files.

#### Scenario: Equivalent proposal content yields the same inbox identity

- **GIVEN** two proposal documents with the same `targetType` and the same canonical `proposedContent`
- **WHEN** their proposal IDs are computed
- **THEN** the resulting `id` values are identical
- **AND** the resulting inbox filename stems are identical

#### Scenario: Proposal artifacts do not change source-of-truth outputs

- **GIVEN** a proposal JSON file exists under `.scryrs/proposals/`
- **WHEN** a reviewer or tool inspects the repository's published docs, memory truth, `.scryrs/graph.json`, or `.scryrs/routes.json`
- **THEN** none of those source-of-truth artifacts have been mutated merely by the proposal file's existence

#### Scenario: Review decisions are recorded outside the proposal inbox

- **GIVEN** a proposal JSON file exists under `.scryrs/proposals/`
- **WHEN** the proposal is later accepted or rejected
- **THEN** the original proposal file remains in `.scryrs/proposals/`
- **AND** the review outcome is stored in a separate `.scryrs/accepted/{proposalId}.json` or `.scryrs/rejected/{proposalId}.json` artifact
- **AND** proposal review state is not represented by mutating the proposal document itself

### Requirement: Scope is limited to contract and inbox definition

The proposal contract SHALL continue to define `ProposalDocument` as a lifecycle-free review inbox artifact. Review decision artifacts are a separate contract and SHALL NOT add status, reviewer, acceptance, or rejection fields to `ProposalDocument`. This change SHALL NOT introduce accept/reject CLI commands, dashboard review UI, automatic proposal or accepted-evidence consumption by graph build or route generation, docs publishing adapters, or memory mutation.

#### Scenario: Proposal documents carry no acceptance lifecycle fields

- **GIVEN** a proposal document produced by this contract
- **WHEN** a consumer inspects the serialized JSON
- **THEN** it does not rely on status, reviewer, acceptance, or rejection fields to be valid
- **AND** review decision metadata lives in separate reviewed-artifact documents

#### Scenario: CLI review commands remain unavailable

- **WHEN** this change is implemented
- **THEN** `scryrs accept` is not registered as a CLI command
- **AND** `scryrs reject` is not registered as a CLI command

#### Scenario: Graph build and route generation do not consume proposal inbox or review decision artifacts yet

- **GIVEN** one or more proposal files exist under `.scryrs/proposals/`
- **AND** one or more review decision files exist under `.scryrs/accepted/` or `.scryrs/rejected/`
- **WHEN** graph build or route generation runs
- **THEN** those commands do not treat proposal inbox files as authoritative graph or route input
- **AND** those commands do not consume reviewed artifacts until accepted-evidence ingestion is introduced by a follow-up change
