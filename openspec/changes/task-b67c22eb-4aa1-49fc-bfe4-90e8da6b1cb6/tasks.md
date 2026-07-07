## 1. Shared review writer extraction

- [ ] 1.1 Extract the CLI review-decision build/write path into a shared `scryrs-curator::proposals` review-write API with request/error types, preserving the current deterministic JSON serialization bytes.
- [ ] 1.2 Update `scryrs proposals accept|reject` to call the shared writer and preserve existing CLI exit-code behavior and validation semantics.
- [ ] 1.3 Add or retain CLI-parity tests proving accepted/rejected serialization, content-override rules, idempotency, and conflict behavior remain unchanged after extraction.

## 2. Dashboard backend review API

- [ ] 2.1 Add local-only `POST /api/proposals/:proposalId/accept` and `POST /api/proposals/:proposalId/reject` handlers that call the shared review writer.
- [ ] 2.2 Extend dashboard API error handling so validation failures map to `400`, conflicts map to `409`, filesystem/serialization/malformed-artifact failures map to `502`, and live-mode proposal review remains unavailable.
- [ ] 2.3 Add backend API tests covering accepted write, rejected write, missing metadata, invalid `decidedAt`, edited Markdown accept, structured-target edit rejection, opposite-outcome conflict, byte-identical idempotency, same-outcome different-byte rejection, and proposal-inbox immutability.

## 3. Dashboard frontend review actions

- [ ] 3.1 Extend the proposal API client with review POST helpers and the Pinia store with accept/reject action state plus detail/list refresh on success.
- [ ] 3.2 Extend `ProposalDetailView` so pending local proposals show required reviewer/rationale/decidedAt inputs, accept/reject buttons, and a Markdown-only optional reviewed-content textarea initialized from the proposal content.
- [ ] 3.3 Ensure terminal proposals and live-mode views expose no active review controls and that validation/conflict/backend failures are surfaced clearly.
- [ ] 3.4 Add frontend store/view tests for successful accept/reject, required metadata handling, failed review actions, terminal-state rendering, and live-mode unavailability.

## 4. Spec and documentation alignment

- [ ] 4.1 Revise the `dashboard-proposal-inbox` OpenSpec capability to replace the read-only write prohibition with explicit local-only review-action requirements and preserved artifact-boundary rules.
- [ ] 4.2 Update project documentation describing proposal review and dashboard proposal endpoints so it reflects explicit dashboard accept/reject actions and the unchanged publish boundary.
- [ ] 4.3 Run strict OpenSpec validation after updating the change artifacts and revised capability delta.
