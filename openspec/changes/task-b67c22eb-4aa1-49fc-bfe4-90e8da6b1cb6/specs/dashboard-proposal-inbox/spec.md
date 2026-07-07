## ADDED Requirements

### Requirement: Dashboard proposal review write endpoints reuse CLI review semantics

The dashboard backend SHALL expose local-only `POST /api/proposals/:proposalId/accept` and `POST /api/proposals/:proposalId/reject` endpoints that write `ProposalReviewDecision` artifacts through the same shared review-decision build/write path used by `scryrs proposals accept|reject`.

Both endpoints SHALL require explicit non-empty `reviewer`, `rationale`, and `decidedAt` fields. `decidedAt` SHALL be validated as RFC3339 and SHALL NOT be derived implicitly by the backend. The accept endpoint MAY accept optional reviewed Markdown content only for Markdown-backed target types (`docs_note`, `adr`, `skill`, `debugging_playbook`). For structured target types (`memory_patch`, `semantic_graph_grouping`), edited content input SHALL be rejected and no review artifact SHALL be written.

Successful first-time writes and byte-identical same-outcome reruns SHALL return `200`. Missing/empty metadata, invalid RFC3339 timestamps, and unsupported edited-content input SHALL return `400`. Opposite-outcome attempts and same-outcome reruns that would change serialized artifact bytes SHALL return `409`. Filesystem, serialization, or malformed-artifact failures SHALL return `502`. In live mode, both review write endpoints SHALL return `404`.

#### Scenario: Accept writes an accepted review decision and leaves the proposal inbox unchanged

- **GIVEN** a valid pending proposal exists at `.scryrs/proposals/abc123.json`
- **WHEN** a caller posts reviewer, rationale, decidedAt, and optional reviewed Markdown content to `POST /api/proposals/abc123/accept`
- **THEN** the response status is `200`
- **AND** `.scryrs/accepted/abc123.json` is created as a valid `ProposalReviewDecision` with `outcome: "accepted"`
- **AND** the decision copies `sourceEvidence` from the proposal
- **AND** the decision preserves the proposal `targetType`
- **AND** `acceptedContent` equals the proposal `proposedContent` unless reviewed Markdown content was explicitly supplied
- **AND** `.scryrs/proposals/abc123.json` remains unchanged

#### Scenario: Reject writes a rejected review decision and leaves the proposal inbox unchanged

- **GIVEN** a valid pending proposal exists at `.scryrs/proposals/abc123.json`
- **WHEN** a caller posts reviewer, rationale, and decidedAt to `POST /api/proposals/abc123/reject`
- **THEN** the response status is `200`
- **AND** `.scryrs/rejected/abc123.json` is created as a valid `ProposalReviewDecision` with `outcome: "rejected"`
- **AND** the decision copies `sourceEvidence` from the proposal
- **AND** the rejected decision omits `targetType` and `acceptedContent`
- **AND** `.scryrs/proposals/abc123.json` remains unchanged

#### Scenario: Missing metadata or invalid decidedAt fails before any write

- **GIVEN** a valid pending proposal exists at `.scryrs/proposals/abc123.json`
- **WHEN** a caller omits `reviewer`, `rationale`, or `decidedAt`, or supplies a non-RFC3339 `decidedAt`, on either review endpoint
- **THEN** the response status is `400`
- **AND** the response body contains a JSON `error` message describing the validation failure
- **AND** no accepted or rejected artifact is written

#### Scenario: Structured targets reject edited content overrides

- **GIVEN** `.scryrs/proposals/abc123.json` is a valid proposal with `targetType: "memory_patch"` or `targetType: "semantic_graph_grouping"`
- **WHEN** a caller supplies reviewed Markdown content to `POST /api/proposals/abc123/accept`
- **THEN** the response status is `400`
- **AND** the response body contains a JSON `error` message indicating edited content is unsupported for the target type
- **AND** no accepted artifact is written

#### Scenario: Conflicting review outcomes fail without overwrite

- **GIVEN** `.scryrs/accepted/abc123.json` already exists
- **WHEN** a caller posts to `POST /api/proposals/abc123/reject`
- **THEN** the response status is `409`
- **AND** the response body contains a JSON `error` message describing the conflicting terminal decision
- **AND** no rejected artifact is written

#### Scenario: Same-outcome rerun with different serialized bytes fails without overwrite

- **GIVEN** `.scryrs/rejected/abc123.json` already exists
- **AND** rerunning reject with different review metadata would change the serialized artifact bytes
- **WHEN** a caller posts the new reject request to `POST /api/proposals/abc123/reject`
- **THEN** the response status is `409`
- **AND** the existing rejected artifact is not overwritten

#### Scenario: Byte-identical same-outcome rerun succeeds idempotently

- **GIVEN** `.scryrs/accepted/abc123.json` already exists
- **AND** rerunning accept with the same metadata and reviewed content would produce byte-identical JSON
- **WHEN** a caller posts the same accept request to `POST /api/proposals/abc123/accept`
- **THEN** the response status is `200`
- **AND** `.scryrs/accepted/abc123.json` remains byte-identical

#### Scenario: Review write endpoints are unavailable in live mode

- **GIVEN** the dashboard is running in live mode
- **WHEN** a caller posts to `POST /api/proposals/abc123/accept` or `POST /api/proposals/abc123/reject`
- **THEN** the response status is `404`
- **AND** the response body contains a JSON `error` message indicating proposal review is unavailable in live mode

## MODIFIED Requirements

### Requirement: Frontend Proposal detail view renders content and evidence

The dashboard frontend SHALL provide a Proposal detail view at `/proposals/:proposalId` that displays proposal rationale, variant-aware `proposedContent`, evidence links, and optional review decision metadata. For pending proposals in local mode, the detail view SHALL also render explicit accept/reject review controls.

The review controls SHALL require explicit non-empty reviewer, rationale, and decidedAt inputs before submission and SHALL NOT silently populate those values. For Markdown-backed target types (`docs_note`, `adr`, `skill`, `debugging_playbook`), the accept flow SHALL offer an optional reviewed Markdown textarea initialized from the proposal's current `proposedContent`. Structured target types SHALL NOT expose edited-content input.

Successful reviews SHALL refresh the proposal detail and list state so the proposal transitions to its terminal accepted/rejected state with review metadata visible. Validation failures, conflicts, and backend failures SHALL be surfaced clearly in the UI. Accepted/rejected proposals and live-mode proposal views SHALL NOT expose active review controls or make write API calls.

#### Scenario: Pending markdown-backed proposal shows explicit review controls

- **GIVEN** `GET /api/proposals/abc123` returns a pending proposal with `targetType: "docs_note"`
- **WHEN** the user navigates to `/proposals/abc123` in local mode
- **THEN** the detail view displays reviewer, rationale, and decidedAt inputs plus accept/reject buttons
- **AND** the accept flow displays an optional reviewed Markdown textarea initialized from the proposal content

#### Scenario: Pending structured proposal omits edited-content input

- **GIVEN** `GET /api/proposals/abc123` returns a pending proposal with `targetType: "memory_patch"`
- **WHEN** the user navigates to `/proposals/abc123` in local mode
- **THEN** the detail view displays reviewer, rationale, and decidedAt inputs plus accept/reject buttons
- **AND** no reviewed-content textarea is shown

#### Scenario: Successful review refreshes the terminal state

- **GIVEN** a pending proposal detail view is open in local mode
- **WHEN** the user submits a valid accept or reject action and the review endpoint returns `200`
- **THEN** the view refreshes proposal detail and list state
- **AND** the proposal is shown in its terminal state with review decision metadata visible
- **AND** active accept/reject controls are no longer rendered

#### Scenario: Failed review action shows a clear error state

- **GIVEN** a pending proposal detail view is open in local mode
- **WHEN** the user submits a review action and the backend returns `400`, `409`, or `502`
- **THEN** the view displays a clear error message describing the failure
- **AND** the pending review form remains available for correction or retry

#### Scenario: Live mode keeps proposal detail unavailable for review writes

- **GIVEN** the dashboard is in live mode
- **WHEN** the user navigates to `/proposals/abc123`
- **THEN** an unavailable message is displayed via `routeUnavailableMessage`
- **AND** no review-write controls are rendered
- **AND** no review-write API call is made

### Requirement: Proposal views perform no writes

The dashboard proposal views and endpoints SHALL preserve the review-first artifact boundary. Read operations remain read-only. Explicit accept and reject actions MAY write only `.scryrs/accepted/{proposalId}.json` or `.scryrs/rejected/{proposalId}.json` review-decision artifacts.

No dashboard proposal code path SHALL create, modify, or delete `.scryrs/proposals/{proposalId}.json`, `.devagent/docs/`, `.scryrs/graph.json`, `.scryrs/routes.json`, or publish outputs. Dashboard review actions SHALL NOT publish accepted content as Markdown or Rspress output.

#### Scenario: Accept writes only the accepted artifact

- **GIVEN** a valid pending proposal exists at `.scryrs/proposals/abc123.json`
- **WHEN** a caller successfully accepts the proposal through the dashboard
- **THEN** `.scryrs/accepted/abc123.json` may be created or confirmed idempotent
- **AND** `.scryrs/proposals/abc123.json`, `.devagent/docs/`, `.scryrs/graph.json`, and `.scryrs/routes.json` are unchanged

#### Scenario: Reject writes only the rejected artifact

- **GIVEN** a valid pending proposal exists at `.scryrs/proposals/abc123.json`
- **WHEN** a caller successfully rejects the proposal through the dashboard
- **THEN** `.scryrs/rejected/abc123.json` may be created or confirmed idempotent
- **AND** `.scryrs/proposals/abc123.json`, `.devagent/docs/`, `.scryrs/graph.json`, and `.scryrs/routes.json` are unchanged