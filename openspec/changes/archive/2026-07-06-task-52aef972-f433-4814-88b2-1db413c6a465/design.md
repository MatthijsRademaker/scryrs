## Context

The scryrs workflow divides knowledge management into three distinct phases: proposal generation (`scryrs propose`), review (`scryrs proposals accept/reject`), and publishing (`scryrs publish markdown/rspress`). The dashboard currently stops at proposal inbox visibility — it lists proposals and optional review metadata at `/proposals` and `/proposals/:proposalId`, but has no accepted-knowledge-first surface and no publish-status model.

This change adds a dedicated Accepted Knowledge surface as a sibling to the existing Proposals surface. The underlying architecture already cleanly separates concerns: accepted decisions live in `.scryrs/accepted/*.json`, validated by `scryrs_curator::proposals::inventory`, and publishing operates through independent adapter crates. The dashboard must expose these layers without collapsing them.

## Goals / Non-Goals

### Goals
- Expose accepted review decisions in the local dashboard as a first-class, read-only list/detail surface.
- Show accepted-item detail with reviewer metadata, reviewed/accepted content (as primary truth), and evidence.
- Surface publish status per publish surface without equating acceptance with publication.
- Reuse existing shared inventory validation so malformed, conflicting, or orphaned accepted artifacts fail loudly.
- Update operator documentation to make the accepted-versus-published boundary explicit.

### Non-Goals
- Do not add publish, accept, reject, or edit actions to the dashboard.
- Do not change CLI publish behavior, adapter output formats, or review-decision contracts.
- Do not make accepted knowledge available in live mode.
- Do not infer published success from proposal state, accepted state, or proposal-detail presence alone.
- Do not hide malformed artifacts behind silent fallbacks or best-effort partial rendering.
- Do not add a persisted publish-metadata artifact or publish ledger.

## Decisions

### Decision 1: Dedicated Accepted Knowledge routes, not a tab/filter overlay

**Chosen:** New routes `/accepted` (list) and `/accepted/:proposalId` (detail) as a sibling to the existing Proposals surface.

The data model — accepted decisions with publish status — is semantically different from the proposal inbox which mixes pending/accepted/rejected. A tab/filter on the proposals list would conflate "accepted with publish info" with "accepted as a filter on proposals," making the publish-status column confusing. Dedicated routes make the accepted-versus-published state model explicit in navigation and URL space.

**Alternatives considered:**
- **Accepted-only filter tab within Proposals view**: Would overload the proposals inbox abstraction. The user story explicitly says "When I open Accepted Knowledge view," implying a distinct surface. Rejected.

### Decision 2: Reuse inventory.rs load_review_decisions for validation

**Chosen:** Use `scryrs_curator::proposals::inventory::load_review_decisions()` in the new endpoints, identical to how the proposals list endpoint works.

This ensures the accepted list validates identically to the proposals list — orphan reviews (accepted artifact with no matching proposal inbox file), conflicting states (accepted + rejected for same proposal ID), and malformed files all fail loud at the API boundary with 502 errors. Any new validation path would create divergence from the proposals endpoint and confuse operators.

For the list endpoint, orphan handling means: if a single accepted artifact has no matching proposal, the entire list endpoint returns 502 (consistent with current inventory behavior). The detail endpoint also fails if its specific target is orphaned.

**Alternatives considered:**
- **Parallel loader with degraded rendering**: A parallel loader could skip orphaned items and render remaining entries. However, silent partial rendering would hide data integrity problems. The fail-loud approach is consistent with existing behavior (proposals list 502s on any malformed file in `api.rs:904-1180`). Rejected.

### Decision 3: Three-state publish-status enum with explicit unknown label for generic Markdown

**Chosen:** Per-surface publish status with these states:
- `published` — Rspress file exists at deterministic path `.devagent/docs/docs/accepted-knowledge/<target-type>/<proposal-id>.md`
- `not_published` — Rspress-publishable target type but the expected file does not exist
- `not_publishable` — target type has no publish surface (memory_patch, semantic_graph_grouping)
- For generic Markdown: always a separate status of `unknown` with the reason "output root not persisted"

Rspress detection works because `scryrs-adapter-rspress` always writes to a well-known subtree within the repo tree (the standard `.devagent/docs/docs/` root). Generic Markdown output roots are operator-chosen via `--output <DIR>` in `scryrs publish markdown` and are never persisted. The `unknown` label is the only honest state.

**Alternatives considered:**
- **Boolean published/unpublished**: Unacceptable because it cannot distinguish non-publishable from unpublished, generating false-negative signals. Rejected.
- **Filesystem scanning for Markdown**: Would search arbitrary directories for matching `.md` files — fragile, slow, and potentially misleading. Rejected.

### Decision 4: Filesystem probe at query time, no persisted publish metadata

**Chosen:** The backend checks for the expected Rspress file on every request. No publish-state artifact is created or persisted.

The non-goal explicitly says "Do not add publish, accept, reject, or edit actions to the dashboard." Persisting publish state would require a write workflow and a new artifact contract, crossing from dashboard-only work into publish-command and adapter contract work.

**Alternatives considered:**
- **Persisted publish ledger**: Would require a new artifact format and write operations — out of scope. Rejected.
- **Trust Rspress _nav.json**: Would couple dashboard to Rspress internals. Rejected.

### Decision 5: Accepted content as primary truth in the detail DTO

**Chosen:** `AcceptedItemDetail` exposes `acceptedContent` as the primary content field, with an optional `originalProposedContent` field for comparison when the proposal inbox file still exists.

The current `ProposalDetail` DTO nests `acceptedContent` inside `ReviewDecisionMeta`, making it a secondary sub-object. For an accepted knowledge surface, reviewed content must be the primary field — the reviewer may have overridden content via `--content-file/--content-stdin`, and the accepted artifact is the source of truth.

**Alternatives considered:**
- **Reuse ProposalDetail with reviewDecision nesting**: Would render overridden content incorrectly in the primary position. Rejected.

### Decision 6: New nav item under LOCAL_NAV, unavailable in live mode

**Chosen:** An "Accepted" nav item with icon `check` (or similar) added to `LOCAL_NAV` in `dashboard-mode.ts`, following the existing pattern for Proposals, Sessions, Events, Routes. Live mode returns 404 with `routeUnavailableMessage` identical to the existing live-mode refusal pattern.

## Risks / Trade-offs

| Risk | Mitigation |
| --- | --- |
| Generic Markdown publish detection is inherently unknowable because `scryrs publish markdown --output <DIR>` does not persist the output root. | Use explicit `unknown` label with descriptive reason text. Document limitation prominently in dashboard UI and API docs. |
| Rspress publish-path detection via `.devagent/docs/docs/accepted-knowledge/` is a heuristic tied to the standard docs root. If an operator runs `scryrs publish rspress --docs-root /some/other/path`, detection misses. | Document that Rspress status reflects only the standard `.devagent/docs/docs/` root. This is the documented convention. |
| A malformed proposal document in `.scryrs/proposals/` will 502 the accepted list endpoint even if all accepted artifacts are well-formed (because `load_review_decisions` loads and validates all proposals first). | Consistent with existing proposals list behavior. Error messages should distinguish "proposal validation failed" from "accepted artifact validation failed." |
| Frontend scope creep: two new views, a new store, router entries, nav item, live-mode guard, and publish-status rendering may exceed the point budget. | Keep views minimal on first pass, mirroring ProposalsView/ProposalDetailView patterns. |

## Traceability

| Source | Artifact |
| --- | --- |
| Task 52aef972-f433-4814-88b2-1db413c6a465 | Dashboard Product 06 feature, AC, technical notes |
| Round 1: swarm-architect | Dedicated routes, publish-status model, inventory reuse, fail-loud semantics |
| Round 1: swarm-lead-dev | Three-state enum, Rspress filesystem probe, Markdown unknown label, frontend nav pattern |
| Round 1: swarm-reviewer | AcceptedContent-as-primary DTO, orphan handling granularity, test plan coverage |
| `server.rs:199-293` | Existing proposals endpoint pattern (inventory reuse, live-mode guard, 502 mapping) |
| `inventory.rs:373-412` | OrphanReview error on accepted artifact without matching proposal |
| `adapter-rspress/lib.rs:57,209-211` | Deterministic accepted-knowledge path structure |
| `adapter-markdown/lib.rs:42-76,212-227` | Caller-chosen output root, PublishableDecision filtering |
| `dashboard-mode.ts:26,72` | LOCAL_NAV/LIVE_NAV pattern for route gating |
| `proposals.md:208,264` | Accept-as-ledger-only, publish-as-separate-step semantics |
| `cli-v0-contract.md:441-455` | Publish CLI contract, operator-chosen output roots |