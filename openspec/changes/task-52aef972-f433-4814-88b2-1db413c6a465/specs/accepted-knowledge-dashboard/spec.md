## ADDED Requirements

### Requirement: Dashboard accepted knowledge list endpoint returns validated accepted decisions

The dashboard backend SHALL expose `GET /api/accepted` as a local-only endpoint that returns a JSON array of accepted knowledge list rows. Each row SHALL include `proposalId`, `title`, `targetType`, `reviewer`, `decidedAt`, `evidenceSummary`, and a `publishStatus` block with per-surface entries. Rows SHALL be sorted by `proposalId` ascending. The endpoint SHALL load proposals and accepted review decisions using shared functions from `scryrs_curator::proposals::inventory` and SHALL reject malformed, conflicting, or orphaned accepted artifacts with HTTP 502.

#### Scenario: List returns accepted decisions sorted by proposalId

- **GIVEN** `.scryrs/proposals/` contains `abc123.json` and `def456.json`
- **AND** `.scryrs/accepted/` contains `abc123.json` with a valid accepted decision
- **WHEN** a caller requests `GET /api/accepted`
- **THEN** the response status is `200`
- **AND** the response body is a JSON array with one entry
- **AND** the entry has `proposalId: "abc123"`, `title` from the matching proposal, and `targetType` from the accepted decision
- **AND** `reviewer`, `decidedAt`, and `evidenceSummary` are present
- **AND** a `publishStatus` block is included

#### Scenario: List excludes pending and rejected proposals

- **GIVEN** `.scryrs/proposals/` contains `abc123.json` (accepted) and `def456.json` (rejected) and `ghi789.json` (pending)
- **AND** `.scryrs/accepted/` contains `abc123.json`
- **AND** `.scryrs/rejected/` contains `def456.json`
- **WHEN** a caller requests `GET /api/accepted`
- **THEN** the response contains only the accepted entry for `abc123`
- **AND** no entries for `def456` or `ghi789` appear

#### Scenario: List returns JSON 404 when accepted directory is missing

- **GIVEN** `.scryrs/accepted/` directory does not exist
- **WHEN** a caller requests `GET /api/accepted`
- **THEN** the response status is `404`
- **AND** the response body is a JSON object with `error` indicating the accepted directory is missing

#### Scenario: List returns JSON 502 when an accepted artifact is malformed

- **GIVEN** `.scryrs/accepted/` contains a file `bad.json` with unparseable JSON
- **WHEN** a caller requests `GET /api/accepted`
- **THEN** the response status is `502`
- **AND** the response body is a JSON object with `error` containing the file path and parse failure message
- **AND** no partial result is returned

#### Scenario: List returns JSON 502 when an accepted artifact has no matching proposal

- **GIVEN** `.scryrs/accepted/orphan.json` exists
- **AND** `.scryrs/proposals/orphan.json` does not exist
- **WHEN** a caller requests `GET /api/accepted`
- **THEN** the response status is `502`
- **AND** the response body is a JSON object with `error` containing a message about an orphan review

#### Scenario: List returns JSON 502 when conflicting accepted and rejected artifacts exist

- **GIVEN** `.scryrs/proposals/conflict.json` exists
- **AND** `.scryrs/accepted/conflict.json` exists
- **AND** `.scryrs/rejected/conflict.json` exists
- **WHEN** a caller requests `GET /api/accepted`
- **THEN** the response status is `502`
- **AND** the response body is a JSON object with `error` containing a message about conflicting terminal state

#### Scenario: List returns JSON 404 in live mode

- **GIVEN** the dashboard is running in live mode (`mode: "live"`)
- **WHEN** a caller requests `GET /api/accepted`
- **THEN** the response status is `404`
- **AND** the response body is a JSON object with `error` indicating accepted knowledge is unavailable in live mode

### Requirement: Dashboard accepted knowledge detail endpoint returns accepted content as primary truth

The dashboard backend SHALL expose `GET /api/accepted/:proposalId` as a local-only endpoint that returns the full accepted knowledge detail for a given proposal ID. The response SHALL expose `acceptedContent` as the primary content field, with an optional `originalProposedContent` field for comparison when the proposal inbox file still exists. The response SHALL include `reviewer`, `rationale`, `decidedAt`, `evidence`, and a per-surface `publishStatus` block.

#### Scenario: Detail returns accepted content as primary field

- **GIVEN** `.scryrs/proposals/abc123.json` contains a valid proposal with `proposedContent: "original text"`
- **AND** `.scryrs/accepted/abc123.json` contains an accepted decision with `acceptedContent: "reviewed text"` (overridden via `--content-file`)
- **WHEN** a caller requests `GET /api/accepted/abc123`
- **THEN** the response status is `200`
- **AND** `acceptedContent` is `"reviewed text"`
- **AND** `originalProposedContent` is `"original text"`
- **AND** `reviewer`, `rationale`, and `decidedAt` are present

#### Scenario: Detail returns accepted content equal to proposed content when no override

- **GIVEN** `.scryrs/proposals/abc123.json` contains `proposedContent: "some content"`
- **AND** `.scryrs/accepted/abc123.json` is accepted without content override
- **WHEN** a caller requests `GET /api/accepted/abc123`
- **THEN** the response status is `200`
- **AND** `acceptedContent` equals `"some content"`
- **AND** `originalProposedContent` is either absent or equals the same value

#### Scenario: Detail returns JSON 404 when proposal ID is not among accepted decisions

- **GIVEN** `.scryrs/accepted/` contains `abc123.json` but not `nonexistent.json`
- **WHEN** a caller requests `GET /api/accepted/nonexistent`
- **THEN** the response status is `404`
- **AND** the response body is a JSON object with `error` indicating the proposal is not an accepted decision

#### Scenario: Detail returns JSON 404 in live mode

- **GIVEN** the dashboard is running in live mode
- **WHEN** a caller requests `GET /api/accepted/any-id`
- **THEN** the response status is `404`
- **AND** the response body is a JSON object with `error` indicating accepted knowledge is unavailable in live mode

### Requirement: Publish status distinguishes published, unpublished, and non-publishable states

The accepted knowledge DTOs SHALL include a `publishStatus` block containing per-surface entries. Each surface entry SHALL include the surface name (`rspress` or `markdown`) and a `status` value. Rspress status SHALL be computed by checking for the expected published file at `.devagent/docs/docs/accepted-knowledge/<target-type>/<proposal-id>.md` on the filesystem at query time. Generic Markdown status SHALL always be `unknown` because the operator-chosen `--output <DIR>` is not persisted. Target types with no publish surface (`memory_patch`, `semantic_graph_grouping`) SHALL report `not_publishable`.

#### Scenario: Rspress publish status is published when file exists

- **GIVEN** `.scryrs/accepted/abc123.json` has `targetType: "docs_note"`
- **AND** `.devagent/docs/docs/accepted-knowledge/docs_note/abc123.md` exists
- **WHEN** a caller requests accepted list or detail for proposal `abc123`
- **THEN** the `publishStatus.rspress.status` is `"published"`
- **AND** `publishStatus.rspress.path` includes the relative path to the published file

#### Scenario: Rspress publish status is not_published when file does not exist

- **GIVEN** `.scryrs/accepted/abc123.json` has `targetType: "docs_note"`
- **AND** `.devagent/docs/docs/accepted-knowledge/docs_note/abc123.md` does not exist
- **WHEN** a caller requests accepted list or detail for proposal `abc123`
- **THEN** the `publishStatus.rspress.status` is `"not_published"`

#### Scenario: Non-publishable target types report not_publishable

- **GIVEN** `.scryrs/accepted/abc123.json` has `targetType: "memory_patch"`
- **WHEN** a caller requests accepted list or detail for proposal `abc123`
- **THEN** the `publishStatus` block indicates `not_publishable` across all surfaces
- **AND** no filesystem probe is performed

#### Scenario: Generic Markdown publish status is always unknown

- **GIVEN** any accepted decision
- **WHEN** the publish status for surface `markdown` is computed
- **THEN** `publishStatus.markdown.status` is `"unknown"`
- **AND** `publishStatus.markdown.reason` includes text indicating the output root is not persisted

### Requirement: Frontend accepted knowledge views are local-only with live-mode gating

The dashboard frontend SHALL provide Accepted Knowledge views at `/accepted` (list) and `/accepted/:proposalId` (detail). Both routes SHALL be registered in the Vue Router, gated to local mode only via `dashboard-mode.ts` navigation, and return an unavailable message via `routeUnavailableMessage()` when accessed in live mode. A dedicated Pinia store SHALL manage accepted knowledge state, fetching from `/api/accepted` endpoints.

#### Scenario: Accepted list renders rows with publish-status badges

- **GIVEN** `GET /api/accepted` returns accepted items with publishStatus blocks
- **WHEN** the user navigates to `/accepted` in local mode
- **THEN** rows display title, targetType, reviewer, and decidedAt
- **AND** each row shows publish-status badges for each surface
- **AND** `published` status renders distinctly from `not_published` and `not_publishable`

#### Scenario: Accepted detail renders accepted content as primary with reviewer metadata

- **GIVEN** `GET /api/accepted/:proposalId` returns a detail with overridden acceptedContent
- **WHEN** the user navigates to `/accepted/:proposalId` in local mode
- **THEN** acceptedContent is displayed as the primary content area
- **AND** when originalProposedContent differs, a comparison or diff indicator is shown
- **AND** reviewer, rationale, and decidedAt metadata are displayed
- **AND** evidence links are listed
- **AND** publish status is rendered per surface

#### Scenario: Accepted views show destructive alerts on API errors

- **GIVEN** `GET /api/accepted` returns `502` with an error message about a malformed artifact
- **WHEN** the user navigates to `/accepted` in local mode
- **THEN** a destructive Alert is displayed with the error message
- **AND** no partial row rendering occurs

#### Scenario: Accepted nav item appears only in local mode

- **WHEN** `navigationForMode("local")` is called
- **THEN** the returned array includes an Accepted nav item with route `/accepted`
- **WHEN** `navigationForMode("live")` is called
- **THEN** the returned array does not include an Accepted nav item

#### Scenario: Accepted views show unavailable message in live mode

- **GIVEN** the dashboard is in live mode
- **WHEN** the user navigates to `/accepted` or `/accepted/:proposalId`
- **THEN** an unavailable message is displayed via `routeUnavailableMessage()`
- **AND** no API call is made

### Requirement: Accepted knowledge validation reuses shared inventory and fails loud

The accepted knowledge endpoints SHALL reuse `scryrs_curator::proposals::inventory::load_review_decisions()` and associated validation functions. Malformed accepted artifacts SHALL return HTTP 502. Accepted artifacts without matching proposal inbox files (orphans) SHALL return HTTP 502 via `InventoryError::OrphanReview`. The `map_inventory_error` function in `server.rs` SHALL handle all inventory error variants, mapping `MissingDirectory` to 404 and all other variants to 502.

#### Scenario: Orphan accepted artifact fails the list endpoint with 502

- **GIVEN** `.scryrs/accepted/orphan.json` exists with a valid review decision for proposal ID `orphan`
- **AND** `.scryrs/proposals/orphan.json` does not exist
- **WHEN** a caller requests `GET /api/accepted`
- **THEN** the response status is `502`
- **AND** the error references the orphan review file path

#### Scenario: Conflicting accepted and rejected artifacts for the same proposal fail with 502

- **GIVEN** `.scryrs/proposals/conflict.json` exists
- **AND** `.scryrs/accepted/conflict.json` exists
- **AND** `.scryrs/rejected/conflict.json` exists
- **WHEN** a caller requests `GET /api/accepted`
- **THEN** the response status is `502`
- **AND** the error message indicates conflicting terminal state for proposal ID `conflict`

#### Scenario: Missing accepted directory returns 404, not 502

- **GIVEN** `.scryrs/accepted/` directory does not exist
- **WHEN** a caller requests `GET /api/accepted`
- **THEN** the response status is `404`
- **AND** the response body is a JSON object indicating the accepted directory is missing

### Requirement: Documentation explains the accepted-versus-published boundary

Operator documentation in `.devagent/docs/docs/proposals.md` and `.devagent/docs/docs/cli-v0-contract.md` SHALL document the Accepted Knowledge dashboard surface, the new `/accepted` and `/accepted/:proposalId` routes, the accepted-versus-published boundary, the Rspress detection heuristic, and the generic Markdown `unknown` publish-status limitation.

#### Scenario: Proposals documentation describes the accepted knowledge dashboard surface

- **WHEN** `.devagent/docs/docs/proposals.md` is read
- **THEN** it describes that the Accepted Knowledge dashboard view shows only accepted decisions from `.scryrs/accepted/*.json`
- **AND** it explains that acceptance is durable review state while publishing is a separate operator action
- **AND** it documents the Rspress publish-status detection heuristic
- **AND** it documents that generic Markdown publish status is `unknown` because output roots are operator-chosen

#### Scenario: CLI contract documentation lists the new API endpoints

- **WHEN** `.devagent/docs/docs/cli-v0-contract.md` is read
- **THEN** the REST API table includes `GET /api/accepted` and `GET /api/accepted/:proposalId`
- **AND** both endpoints are annotated as local-mode only
