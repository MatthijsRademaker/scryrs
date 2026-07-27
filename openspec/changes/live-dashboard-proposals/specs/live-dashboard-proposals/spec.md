## ADDED Requirements

### Requirement: Clients can publish validated proposals per repository

The live server SHALL expose an authenticated publication endpoint for validated proposal documents. Publication SHALL be idempotent for identical content, reject incompatible schema or target-type payloads, and prevent silent replacement of a different proposal revision.

#### Scenario: Valid proposal is published

- **GIVEN** an authenticated publisher for repository `repo-a`
- **WHEN** it publishes a valid proposal with ID `proposal-1`
- **THEN** the server stores the proposal under `repo-a`
- **AND** inventory can return it as pending

#### Scenario: Identical publication is idempotent

- **GIVEN** proposal `proposal-1` already has identical content on the server
- **WHEN** the client publishes it again
- **THEN** the server returns success
- **AND** no duplicate proposal is created

### Requirement: Live server exposes proposal inventory and detail

The live server SHALL expose repository-scoped read APIs for proposal list and detail. List results SHALL be deterministic and include proposal ID, title, target type, creation time, and state. Detail SHALL include proposal content, evidence, schema version, and review decision metadata when present.

#### Scenario: Live inventory returns repository proposals

- **WHEN** a client requests proposals for `repo-a`
- **THEN** only proposals belonging to `repo-a` are returned
- **AND** rows are sorted by proposal ID

#### Scenario: Live detail returns review metadata

- **GIVEN** proposal `proposal-1` has an accepted review decision
- **WHEN** detail is requested
- **THEN** the response includes accepted outcome, reviewer, rationale, decision timestamp, and accepted content metadata

### Requirement: Live server supports explicit authenticated proposal review

The live server SHALL expose authenticated accept and reject endpoints with the same validation, idempotency, and conflict semantics as local review. Reviewer, rationale, and RFC3339 decision timestamp SHALL be explicit. Opposite terminal outcomes and changed same-outcome retries SHALL fail without overwriting existing decisions.

#### Scenario: First accept succeeds

- **GIVEN** a valid pending proposal and authorized reviewer
- **WHEN** the reviewer accepts it with required metadata
- **THEN** the server stores an accepted decision
- **AND** subsequent inventory reports state `accepted`

#### Scenario: Conflicting review is rejected

- **GIVEN** proposal `proposal-1` is already accepted
- **WHEN** a reviewer attempts to reject it
- **THEN** the server returns conflict
- **AND** the accepted decision remains unchanged

#### Scenario: Identical retry is idempotent

- **GIVEN** an accepted decision already exists
- **WHEN** the same accept request is repeated byte-identically
- **THEN** the server returns success without creating a second decision

### Requirement: Dashboard proxies live proposal reads and review writes

In live mode, the dashboard SHALL proxy proposal list/detail and authenticated accept/reject requests to the configured live server. It SHALL not read or write local proposal artifacts as fallback.

#### Scenario: Live proposal list is proxied

- **WHEN** the browser opens Proposals in live mode
- **THEN** the dashboard requests repository-scoped live inventory
- **AND** renders deterministic proposal rows

#### Scenario: Live review result is surfaced

- **WHEN** a live accept or reject request returns success or conflict
- **THEN** the dashboard displays the result explicitly
- **AND** refreshes proposal state after success

### Requirement: Live Proposals navigation is available with review security

When live mode is active, the dashboard SHALL show Proposals navigation only when live proposal APIs are configured. Review controls SHALL require authenticated server authorization and explicit reviewer metadata. The UI SHALL never imply that live review writes are available when authorization is absent.

#### Scenario: Live proposal navigation renders inventory

- **GIVEN** live proposal reads are available
- **WHEN** navigation renders
- **THEN** a Proposals entry is visible

#### Scenario: Unauthorized review is not hidden

- **WHEN** a reviewer lacks authorization for a live review write
- **THEN** the dashboard displays the server's explicit authorization error
- **AND** it does not claim the review succeeded
