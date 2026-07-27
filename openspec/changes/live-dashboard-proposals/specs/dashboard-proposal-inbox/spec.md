## MODIFIED Requirements

### Requirement: Dashboard read-only proposal list endpoint reuses shared inventory semantics

The dashboard backend SHALL expose `GET /api/proposals` in local and live modes. Local mode SHALL use shared curator inventory over workspace artifacts. Live mode SHALL proxy repository-scoped server inventory and return the same proposal list row contract and deterministic proposal ID ordering.

#### Scenario: Local list preserves existing behavior

- **GIVEN** the dashboard is in local mode
- **WHEN** a caller requests `GET /api/proposals`
- **THEN** the response uses local curator inventory semantics

#### Scenario: Live list proxies server inventory

- **GIVEN** the dashboard is in live mode for repository `repo-a`
- **WHEN** a caller requests `GET /api/proposals`
- **THEN** the dashboard requests live inventory for `repo-a`
- **AND** it does not read local proposal directories

### Requirement: Dashboard proposal detail endpoint returns full document with review metadata

The dashboard backend SHALL expose proposal detail in local and live modes. Live detail SHALL proxy the server's validated proposal and review document while preserving the existing DTO shape.

#### Scenario: Live detail returns proposal content

- **GIVEN** a published proposal exists for the live repository
- **WHEN** `GET /api/proposals/:proposalId` is called
- **THEN** the response includes proposal content, evidence, schema, and review metadata when present

### Requirement: Dashboard proposal review write endpoints reuse CLI review semantics

The dashboard backend SHALL expose accept/reject review endpoints in local mode using existing filesystem semantics and in live mode by proxying authenticated server review APIs. Both modes SHALL require explicit reviewer, rationale, and RFC3339 decision timestamp, preserve idempotency and conflict behavior, and never silently overwrite terminal decisions.

#### Scenario: Live accept proxies explicit review metadata

- **GIVEN** a pending live proposal and authorized reviewer
- **WHEN** the dashboard receives an accept request with valid metadata
- **THEN** it proxies the request to the live server
- **AND** returns the server's review result

#### Scenario: Live review conflict is preserved

- **GIVEN** the live server reports a conflicting terminal decision
- **WHEN** the dashboard receives the response
- **THEN** it returns conflict to the browser
- **AND** does not claim the review succeeded
