# dashboard-proposal-inbox Specification

## Purpose
TBD - created by archiving change task-c02ccc65-168e-421d-8912-f4e1dba0fee8. Update Purpose after archive.
## Requirements
### Requirement: Dashboard read-only proposal list endpoint reuses shared inventory semantics

The dashboard backend SHALL expose `GET /api/proposals` as a local-only endpoint that returns a JSON array of proposal list rows. Each row SHALL include `proposalId`, `title`, `targetType`, `createdAt`, and `state` (one of `"pending"`, `"accepted"`, or `"rejected"`). Rows SHALL be sorted by `proposalId` ascending, matching the deterministic sort order of `scryrs proposals list`. The endpoint SHALL load proposals from `.scryrs/proposals/`, accepted decisions from `.scryrs/accepted/`, and rejected decisions from `.scryrs/rejected/` using shared functions from `scryrs-curator::proposals::inventory`.

#### Scenario: List returns deterministic rows sorted by proposalId

- **GIVEN** `.scryrs/proposals/` contains proposal artifacts `abc123.json` and `def456.json`
- **AND** `.scryrs/accepted/` contains `abc123.json`
- **WHEN** a caller requests `GET /api/proposals`
- **THEN** the response status is `200`
- **AND** the response body is a JSON array with two entries
- **AND** the first entry has `proposalId: "abc123"`, `state: "accepted"`
- **AND** the second entry has `proposalId: "def456"`, `state: "pending"`
- **AND** entries are ordered by `proposalId` ascending

#### Scenario: List returns JSON 404 when proposals directory is missing

- **GIVEN** `.scryrs/proposals/` directory does not exist
- **WHEN** a caller requests `GET /api/proposals`
- **THEN** the response status is `404`
- **AND** the response body is a JSON object with `error` containing a message that indicates the proposals directory is missing

#### Scenario: List returns JSON 502 when a proposal artifact is malformed

- **GIVEN** `.scryrs/proposals/` contains a file `bad.json` with unparseable JSON
- **WHEN** a caller requests `GET /api/proposals`
- **THEN** the response status is `502`
- **AND** the response body is a JSON object with `error` containing the file path and parse failure message
- **AND** no partial result is returned

#### Scenario: List returns JSON 502 when conflicting accepted and rejected artifacts exist

- **GIVEN** `.scryrs/proposals/` contains `abc123.json`
- **AND** `.scryrs/accepted/abc123.json` exists
- **AND** `.scryrs/rejected/abc123.json` exists
- **WHEN** a caller requests `GET /api/proposals`
- **THEN** the response status is `502`
- **AND** the response body is a JSON object with `error` containing a message about conflicting terminal state for proposal ID `abc123`

#### Scenario: List returns JSON 404 in live mode

- **GIVEN** the dashboard is running in live mode (`mode: "live"`)
- **WHEN** a caller requests `GET /api/proposals`
- **THEN** the response status is `404`
- **AND** the response body is a JSON object with `error` containing a message that proposals are unavailable in live mode

### Requirement: Dashboard read-only proposal detail endpoint returns full document with review metadata

The dashboard backend SHALL expose `GET /api/proposals/:proposalId` as a local-only endpoint that returns the full `ProposalDocument` (schemaVersion, id, targetType, title, rationale, proposedContent, evidence, createdAt) plus optional review decision metadata when accepted or rejected artifacts exist. The review decision metadata SHALL include reviewer, outcome, decidedAt, and rationale. When the proposal has been accepted, the response SHALL also include acceptedContent and targetType from the review decision.

#### Scenario: Detail returns full proposal document for pending proposal

- **GIVEN** `.scryrs/proposals/abc123.json` contains a valid proposal
- **AND** no accepted or rejected artifact exists for `abc123`
- **WHEN** a caller requests `GET /api/proposals/abc123`
- **THEN** the response status is `200`
- **AND** the response body includes `id`, `targetType`, `title`, `rationale`, `proposedContent`, `evidence`, `createdAt`
- **AND** `reviewDecision` is `null`

#### Scenario: Detail includes review decision metadata for accepted proposal

- **GIVEN** `.scryrs/proposals/abc123.json` contains a valid proposal
- **AND** `.scryrs/accepted/abc123.json` contains a valid accepted decision
- **WHEN** a caller requests `GET /api/proposals/abc123`
- **THEN** the response status is `200`
- **AND** `reviewDecision` is not null
- **AND** `reviewDecision.outcome` is `"accepted"`
- **AND** `reviewDecision.reviewer`, `reviewDecision.decidedAt`, and `reviewDecision.rationale` are present

#### Scenario: Detail includes review decision metadata for rejected proposal

- **GIVEN** `.scryrs/proposals/abc123.json` contains a valid proposal
- **AND** `.scryrs/rejected/abc123.json` contains a valid rejected decision
- **WHEN** a caller requests `GET /api/proposals/abc123`
- **THEN** the response status is `200`
- **AND** `reviewDecision.outcome` is `"rejected"`

#### Scenario: Detail returns JSON 404 when proposal ID is not found

- **GIVEN** `.scryrs/proposals/` contains `abc123.json` but not `nonexistent.json`
- **WHEN** a caller requests `GET /api/proposals/nonexistent`
- **THEN** the response status is `404`
- **AND** the response body is a JSON object with `error` indicating the proposal was not found

#### Scenario: Detail returns JSON 404 in live mode

- **GIVEN** the dashboard is running in live mode
- **WHEN** a caller requests `GET /api/proposals/any-id`
- **THEN** the response status is `404`
- **AND** the response body is a JSON object with `error` containing a message that proposal detail is unavailable in live mode

### Requirement: Proposal inventory logic is shared between CLI and dashboard

The proposal inventory loading and validation functions (`load_proposals`, `load_review_decisions`, `collect_list_rows`, `validate_proposal_document`, `validate_review_decision_artifact`) SHALL be extracted from `scryrs-cli/src/proposals.rs` into a public module in `scryrs-curator` (`scryrs-curator/src/proposals/inventory.rs`). The shared module SHALL use a dedicated `InventoryError` type instead of CLI-specific `CommandError`. Both `scryrs-cli` and `scryrs-dashboard` SHALL depend on `scryrs-curator` for proposal inventory loading.

#### Scenario: CLI imports inventory functions from scryrs-curator

- **GIVEN** the inventory module exists in `scryrs-curator`
- **WHEN** `scryrs-cli` builds
- **THEN** `scryrs-cli/src/proposals.rs` imports `collect_list_rows`, `load_proposals`, and `load_review_decisions` from `scryrs-curator::proposals::inventory`
- **AND** existing CLI proposal tests produce identical results

#### Scenario: Dashboard imports inventory functions from scryrs-curator

- **GIVEN** the inventory module exists in `scryrs-curator`
- **WHEN** `scryrs-dashboard` builds
- **THEN** `scryrs-dashboard/Cargo.toml` lists `scryrs-curator` as a production dependency
- **AND** the `/api/proposals` handler calls `scryrs-curator::proposals::inventory::collect_list_rows`

#### Scenario: InventoryError maps to consumer-specific error types

- **GIVEN** `collect_list_rows` encounters a malformed proposal file
- **WHEN** the CLI consumer calls it
- **THEN** the `InventoryError` is mapped to `CommandError::input` with exit code `2`
- **AND** when the dashboard consumer calls it
- **THEN** the `InventoryError` is mapped to `ApiError::bad_gateway` with status `502`

### Requirement: Frontend Proposals list view shows states and handles errors

The dashboard frontend SHALL provide a Proposals list view at `/proposals` that displays proposal rows with proposalId (truncated), title, targetType, createdAt, and a state badge (Pending / Accepted / Rejected). The view SHALL use a destructive Alert to surface API errors, an EmptyState when no proposals exist, and a routeUnavailableMessage in live mode.

#### Scenario: Proposals list renders rows with state badges

- **GIVEN** `GET /api/proposals` returns two entries with states `"pending"` and `"accepted"`
- **WHEN** the user navigates to `/proposals` in local mode
- **THEN** two rows are displayed
- **AND** each row shows a truncated proposalId, title, targetType, createdAt, and a state badge
- **AND** the pending entry shows a "Pending" badge
- **AND** the accepted entry shows an "Accepted" badge

#### Scenario: Proposals list shows destructive alert on API error

- **GIVEN** `GET /api/proposals` returns `502` with an error message about a malformed artifact
- **WHEN** the user navigates to `/proposals` in local mode
- **THEN** a destructive Alert is displayed with the error message
- **AND** no partial row rendering occurs

#### Scenario: Proposals list shows unavailable message in live mode

- **GIVEN** the dashboard is in live mode
- **WHEN** the user navigates to `/proposals`
- **THEN** an unavailable message is displayed via `routeUnavailableMessage("proposals", "live")`
- **AND** no API call is made

#### Scenario: Proposals list shows empty state when no proposals exist

- **GIVEN** `GET /api/proposals` returns an empty array
- **WHEN** the user navigates to `/proposals` in local mode
- **THEN** an EmptyState component is displayed indicating no proposals

### Requirement: Frontend Proposal detail view renders content and evidence

The dashboard frontend SHALL provide a Proposal detail view at `/proposals/:proposalId` that displays proposal rationale, proposedContent (variant-aware rendering), evidence links (structured list), and optional review decision metadata. The view SHALL use a destructive Alert for API errors and a routeUnavailableMessage in live mode.

#### Scenario: Detail renders rationale and markdown proposed content

- **GIVEN** `GET /api/proposals/abc123` returns a proposal with `targetType: "docs_note"` and `proposedContent` as a markdown string
- **WHEN** the user navigates to `/proposals/abc123`
- **THEN** the rationale is displayed as plain text
- **AND** the proposed content is displayed in a `<pre>` block with word-wrap

#### Scenario: Detail renders evidence links as structured list

- **GIVEN** `GET /api/proposals/abc123` returns a proposal with two evidence links
- **WHEN** the user navigates to `/proposals/abc123`
- **THEN** each evidence link displays `sourceKind`, `subject`, and `rowIds`
- **AND** the evidence links are presented as a structured list, not raw JSON

#### Scenario: Detail renders review decision metadata when present

- **GIVEN** `GET /api/proposals/abc123` returns a proposal with `reviewDecision.outcome: "accepted"`
- **WHEN** the user navigates to `/proposals/abc123`
- **THEN** a review decision section is displayed
- **AND** it shows `outcome` (e.g. "Accepted"), `reviewer`, `decidedAt`, and `rationale`

#### Scenario: Detail shows destructive alert on API error

- **GIVEN** `GET /api/proposals/abc123` returns `502`
- **WHEN** the user navigates to `/proposals/abc123`
- **THEN** a destructive Alert is displayed with the error message

#### Scenario: Detail shows unavailable message in live mode

- **GIVEN** the dashboard is in live mode
- **WHEN** the user navigates to any `/proposals/:proposalId`
- **THEN** an unavailable message is displayed via `routeUnavailableMessage`
- **AND** no API call is made

### Requirement: Proposals navigation is local-only

The Proposals navigation entry SHALL be registered in `LOCAL_NAV` in `crates/scryrs-dashboard/frontend/src/shared/lib/dashboard-mode.ts`. It SHALL NOT appear in `LIVE_NAV`. The `routeUnavailableMessage` function SHALL return explicit unavailable messages for route names `"proposals"` and `"proposal-detail"` in live mode.

#### Scenario: Proposals appears in local navigation

- **GIVEN** the dashboard is in local mode
- **WHEN** the sidebar navigation is rendered
- **THEN** a "Proposals" navigation entry is visible
- **AND** it links to `/proposals`

#### Scenario: Proposals is absent from live navigation

- **GIVEN** the dashboard is in live mode
- **WHEN** the sidebar navigation is rendered
- **THEN** no "Proposals" navigation entry is visible

#### Scenario: Live-mode navigation to proposals shows unavailable message

- **GIVEN** the dashboard is in live mode
- **WHEN** the user navigates to `/proposals` or `/proposals/:proposalId`
- **THEN** a message is displayed indicating proposals are not available in live mode

### Requirement: Proposal views perform no writes

The dashboard proposal endpoints and views SHALL be strictly read-only. No code path introduced by this change SHALL create, modify, or delete files under `.scryrs/proposals/`, `.scryrs/accepted/`, `.scryrs/rejected/`, `.scryrs/graph.json`, `.scryrs/routes.json`, `.devagent/docs/`, or any other persistent state. No accept, reject, or edit actions SHALL be implemented.

#### Scenario: Listing proposals does not mutate the filesystem

- **GIVEN** `.scryrs/proposals/`, `.scryrs/accepted/`, and `.scryrs/rejected/` contain artifacts
- **WHEN** `GET /api/proposals` or `GET /api/proposals/:proposalId` is called
- **THEN** no new files are created under those directories
- **AND** no existing files are modified or deleted
- **AND** `.scryrs/graph.json` and `.scryrs/routes.json` are unchanged

#### Scenario: No accept/reject actions are exposed

- **WHEN** the dashboard server routes and frontend views are inspected
- **THEN** no `POST`, `PUT`, `PATCH`, or `DELETE` handlers exist for `/api/proposals`
- **AND** the frontend contains no accept, reject, or edit controls for proposals

