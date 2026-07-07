## Why

The dashboard proposal inbox is currently a read-only local view, so reviewers still have to fall back to `scryrs proposals accept|reject` to complete the review-first workflow. This task closes that gap by adding explicit dashboard accept/reject actions that collect reviewer metadata, optionally accept reviewed Markdown for Markdown-backed targets, and write durable review-decision artifacts without mutating the proposal inbox.

The implementation must preserve the existing review contract rather than inventing a dashboard-specific fork: validation, content-override rules, idempotency, and conflict behavior must stay identical to the CLI/domain path. The canonical OpenSpec dashboard proposal inbox spec also currently forbids writes, so this change must explicitly replace that read-only constraint.

## What Changes

### 1. Shared review-decision write path

- Extract the CLI's review-decision build/write path into a shared `scryrs-curator::proposals` API so the dashboard and CLI use the same validation, serialization, conflict, and idempotency logic.
- Preserve byte-identical JSON output so existing CLI idempotency rules continue to work unchanged.
- Keep CLI-specific exit-code handling in the CLI layer by mapping shared domain errors back to the current command behavior.

### 2. Local-only dashboard review write endpoints

- Add `POST /api/proposals/:proposalId/accept` and `POST /api/proposals/:proposalId/reject` as local-only dashboard endpoints.
- Require explicit `reviewer`, `rationale`, and `decidedAt` metadata for both actions; do not derive defaults.
- Allow optional reviewed Markdown content only on accept for Markdown-backed targets.
- Return distinguishable JSON failures for input validation (`400`), conflicts/idempotency mismatches (`409`), and filesystem or serialization failures (`502`).

### 3. Proposal detail review UI

- Extend the dashboard API client, Pinia proposal store, and `ProposalDetailView` with explicit accept/reject actions for pending local proposals.
- Require reviewer, rationale, and decidedAt inputs before submission.
- For Markdown-backed targets, provide an optional reviewed-content textarea initialized from the proposal's current Markdown.
- Refresh the proposal detail/list state after success and surface loading, success, terminal-state, and failure feedback clearly.

### 4. Verification and contract alignment

- Add backend tests for accepted, rejected, missing metadata, invalid timestamp, edited Markdown, structured-target edit rejection, conflicts, idempotency, and proposal-inbox immutability.
- Add frontend store/view tests for successful review, required metadata handling, failures, terminal-state rendering, and live-mode unavailability.
- Update the dashboard proposal inbox OpenSpec capability and related project docs so the canonical contract matches the new bounded write behavior.

## Impact

- Reviewers can complete accept/reject decisions from the dashboard without manual CLI commands.
- The proposal inbox remains immutable; only `.scryrs/accepted/` or `.scryrs/rejected/` artifacts are written.
- CLI and dashboard review behavior stay aligned because both use one shared domain implementation.
- The dashboard proposal inbox contract moves from strictly read-only review to explicit local-only review actions with preserved artifact boundaries.
