## Why

The dashboard currently exposes only proposal-inbox visibility: operators can browse proposals and see review metadata, but they cannot answer the basic operational question — which decisions actually left `.scryrs/proposals/`, what reviewed content was accepted, and whether any accepted item was later materialized to Markdown or Rspress. The scryrs product boundary is explicit: acceptance is durable review state (`.scryrs/accepted/*.json`), while publishing is a separate, partially discoverable operator action. The dashboard must expose both surfaces without conflating them.

This change adds a dedicated, local-only, read-only Accepted Knowledge surface to the dashboard with publish-status visibility that respects the accepted-versus-published product boundary.

## What Changes

- **New backend endpoints**: `GET /api/accepted` (list) and `GET /api/accepted/:proposalId` (detail), reusing shared inventory validation from `scryrs_curator::proposals::inventory` for fail-loud semantics on malformed, conflicting, and orphaned accepted artifacts.
- **New Accepted Item DTOs**: `AcceptedItemListRow` (proposalId, title, targetType, reviewer, decidedAt, evidenceSummary, publishStatus block) and `AcceptedItemDetail` (acceptedContent as primary truth, optional originalProposedContent for comparison, reviewer, rationale, evidence, per-surface publishStatus).
- **Publish-status model**: A per-surface status block with three states — `published` (Rspress file exists at deterministic path), `not_published` (Rspress-publishable but file not found), `not_publishable` (memory_patch, semantic_graph_grouping) — plus a separate `unknown` label for generic Markdown since the operator-chosen `--output <DIR>` is never persisted.
- **Rspress detection via filesystem probe**: At query time, the backend checks for `.devagent/docs/docs/accepted-knowledge/<target-type>/<proposal-id>.md`. No publish metadata artifact is created or persisted.
- **New frontend routes and views**: `/accepted` (AcceptedListView) and `/accepted/:proposalId` (AcceptedDetailView) with a dedicated Pinia store, a new nav item in LOCAL_NAV, and live-mode unavailability gating following the existing proposals/sessions/events pattern.
- **Documentation updates**: `.devagent/docs/docs/proposals.md` and `.devagent/docs/docs/cli-v0-contract.md` updated to document the new endpoints, the accepted-versus-published boundary, Rspress detection heuristics, and generic Markdown discoverability limits.

## Impact

- **Dashboard backend** (`crates/scryrs-dashboard/src/server.rs`): New endpoint handlers, new DTOs, publish-status filesystem probing logic, live-mode gating.
- **Frontend** (`crates/scryrs-dashboard/frontend/src/`): New router entries, nav items, Pinia store, Api client DTOs, and two new views.
- **Shared inventory** (`crates/scryrs-curator`): No changes required — the new endpoints reuse existing `load_review_decisions()` and validation functions as-is.
- **Adapters** (`scryrs-adapter-markdown`, `scryrs-adapter-rspress`): No changes — publish output behavior and contracts are unchanged.
- **Documentation** (`.devagent/docs/docs/`): Updated proposals.md and cli-v0-contract.md.
- **Tests** (`crates/scryrs-dashboard/tests/api.rs`): New API contract tests for accepted list/detail endpoints, orphan/malformed/conflicting artifact error coverage, publish-status scenarios, and live-mode refusal.
- **No changes to**: CLI commands, publish adapters, proposal/review contracts, live-mode server, or `.scryrs/` directory layout.