## Context

The current dashboard proposal surface is local-only and read-only: `GET /api/proposals` and `GET /api/proposals/:proposalId` exist, but there are no write routes or frontend review controls. Meanwhile, the CLI already implements proposal acceptance and rejection with explicit reviewer metadata, RFC3339 `decidedAt` validation, optional reviewed Markdown for Markdown-backed targets, deterministic serialization, byte-identical idempotency, and conflict handling.

This task adds dashboard review actions without changing the review-first artifact model. The dashboard must write only accepted/rejected decision artifacts, never mutate `.scryrs/proposals/{proposalId}.json`, and must reuse the existing CLI/domain behavior rather than reimplementing or weakening it. The canonical `dashboard-proposal-inbox` OpenSpec capability currently forbids writes, so the change also has to revise that contract explicitly.

## Goals / Non-Goals

### Goals

- Add explicit local-dashboard accept and reject actions for pending proposals.
- Require explicit non-empty `reviewer`, `rationale`, and RFC3339 `decidedAt` metadata for both actions.
- Support optional reviewed Markdown content on accept for Markdown-backed proposal targets.
- Reuse one shared review-decision write implementation across CLI and dashboard so validation, override rules, conflicts, and idempotency remain identical.
- Surface clear loading, success, terminal-state, and failure feedback in the dashboard UI.
- Add backend, frontend, and CLI-parity test coverage for success, validation failure, conflict, idempotency, edited Markdown, and proposal-inbox immutability.
- Replace the existing read-only dashboard proposal OpenSpec constraint with the new bounded write behavior.

### Non-Goals

- Do not publish accepted decisions to Markdown or Rspress as part of dashboard review.
- Do not mutate `.devagent/docs/`, `.scryrs/graph.json`, `.scryrs/routes.json`, or proposal inbox artifacts during dashboard review.
- Do not add proposal review support in live dashboard mode.
- Do not add authentication, reviewer discovery, or silent metadata defaults.
- Do not add batch review, bulk actions, or a rich Markdown editor beyond the minimum edited-content input.

## Decisions

### Decision 1: Extract the write path into `scryrs-curator::proposals`

The CLI's private review write/build path moves into a shared `scryrs-curator::proposals` module dedicated to review writes. The shared API accepts a review request struct plus repository root and returns domain errors that the CLI and dashboard map to their own transport-specific error surfaces.

This is required to satisfy the task's "reuse CLI/domain validation, not fork rules" constraint and to keep one authoritative implementation for proposal existence checks, metadata validation, content override rules, artifact validation, deterministic serialization, idempotency, and conflicts.

### Decision 2: Use dedicated local-only POST endpoints

The dashboard write contract uses two dedicated endpoints:

- `POST /api/proposals/:proposalId/accept`
- `POST /api/proposals/:proposalId/reject`

Both endpoints require explicit `reviewer`, `rationale`, and `decidedAt`. The accept endpoint may also carry optional reviewed Markdown content. The backend returns `400` for missing/invalid metadata or unsupported edited-content input, `409` for opposite-outcome conflicts and same-outcome different-byte overwrites, `502` for filesystem/serialization/malformed-artifact failures, and `404` in live mode.

### Decision 3: Keep the dashboard review UI narrow and explicit

`ProposalDetailView` becomes the only review-action entry point. Pending proposals in local mode show an explicit metadata form and accept/reject buttons. The UI does not silently populate reviewer, rationale, or decidedAt.

For Markdown-backed targets, the accept flow exposes an optional reviewed-content textarea initialized from the proposal's current `proposedContent`. Structured targets do not expose edited-content input.

### Decision 4: Treat byte-identical reruns as normal success

The shared review writer preserves the CLI's byte-identical idempotency rule. First-time writes and byte-identical same-outcome reruns both succeed, and dashboard callers receive a normal success response (`200`) followed by a refreshed terminal proposal state.

### Decision 5: Replace the read-only spec constraint with bounded write semantics

The `dashboard-proposal-inbox` capability is revised so proposal review is no longer described as strictly read-only. Instead, the spec now permits explicit accept/reject writes while still prohibiting mutation of proposal inbox artifacts and other protected outputs.

## Conflict Resolution

### Endpoint shape

The dossier raised a choice between a single review endpoint and two outcome-specific endpoints. The accepted architect and lead-dev recommendations both converge on dedicated `accept` and `reject` POST routes, so this specification adopts the two-endpoint contract.

### HTTP error mapping

The dossier noted that the current dashboard error helpers only cover `404` and `502`. Accepted room decisions explicitly call for distinct `400` and `409` handling so the frontend can distinguish validation failures from conflicts and backend failures. This specification adopts that mapping.

### Edited-content UX

Room evidence raised a choice between requiring opt-in editing and pre-populating the reviewed Markdown field. The accepted architect recommendation specifically proposed an optional textarea initialized from the proposal's current content for Markdown-backed targets, which keeps the UI narrow and satisfies the edited-Markdown acceptance criterion without expanding scope into a richer editor.

## Risks

- **Byte-preservation risk:** extracting the review writer must keep serialized JSON byte-identical to the current CLI output or idempotency parity will break.
- **Error-mapping risk:** the backend must separate validation/conflict failures from filesystem failures or the dashboard cannot present clear failure states.
- **Frontend test coverage risk:** proposal mutations currently have no existing store/view test coverage, so the change must add targeted frontend tests rather than relying only on backend coverage.
- **Spec-drift risk:** the old read-only dashboard proposal requirement must be explicitly replaced in the OpenSpec delta so the implementation does not contradict canonical behavior.

## Traceability

- Task: `b67c22eb-4aa1-49fc-bfe4-90e8da6b1cb6`
- Dossier: `2026-07-06T05:44:58.260Z`
- Accepted decisions: `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, `1-swarm-reviewer-recommendation`
- Round outputs: `round:1:agent:swarm-architect`, `round:1:agent:swarm-lead-dev`, `round:1:agent:swarm-reviewer`
- Canonical capability to revise: `openspec/specs/dashboard-proposal-inbox/spec.md`
- Supporting behavior baseline: `openspec/specs/proposal-review-cli/spec.md`
