## Context

The dashboard currently exposes `/api/meta`, `/api/hotspots`, `/api/signals`, `/api/sessions`, `/api/sessions/:session_id`, and `/api/events` endpoints, with corresponding Vue 3 frontend views, stores, and navigation. Proposal artifacts live under `.scryrs/proposals/`, `.scryrs/accepted/`, and `.scryrs/rejected/` but have no dashboard surface. The CLI's `scryrs proposals list` already provides deterministic listing and validation semantics, but its implementation is private (`pub(crate)`) within `scryrs-cli` and cannot be imported by `scryrs-dashboard`.

Three architectural decisions were resolved during refinement:

1. The shared proposal read path extracts to `scryrs-curator` (not scryrs-core or vendored duplication), because curator already owns proposal-domain logic and depends only on scryrs-types + serde_json.
2. The API follows the established Sessions pattern: separate `GET /api/proposals` (list) and `GET /api/proposals/:proposalId` (detail) endpoints.
3. The frontend follows the Sessions routing pattern: separate `/proposals` and `/proposals/:proposalId` routes rather than a single-page master/detail with in-page selection.

## Goals / Non-Goals

### Goals
- Expose proposal inbox state in the dashboard from local `.scryrs` artifacts by reusing the same deterministic listing and validation rules as `scryrs proposals list`.
- Provide a read-only proposal list showing proposalId, title, targetType, createdAt, and state.
- Provide a read-only proposal detail view showing rationale, proposedContent (as `<pre>`), evidence links, and optional review decision metadata.
- Fail loudly on malformed artifacts: a single unparseable JSON file or conflicting accepted+rejected terminal state returns a JSON 502 error for the entire endpoint.
- Add automated coverage for backend endpoint behavior and frontend navigation/data/error paths.

### Non-Goals
- No accept/reject/review actions or any write endpoint.
- No mutation of proposal inbox, accepted/rejected ledgers, docs, graph, or routes.
- No rich markdown rendering in the frontend (deferred to follow-up).
- No live-mode proposal browsing.
- No editing of canonical OpenSpec specs as part of implementation.

## Decisions

### Decision 1: Extract shared proposal read logic to scryrs-curator
**Choice**: Extract `load_proposals`, `load_review_decisions`, `collect_list_rows`, `validate_proposal_document`, and `validate_review_decision_artifact` from `scryrs-cli/src/proposals.rs` into a new `scryrs-curator::proposals::inventory` public module.
**Rationale**: curator already owns proposal-domain logic and depends only on scryrs-types + serde_json. Both scryrs-cli and scryrs-dashboard can depend on curator without introducing circular crate dependencies. Duplicating the logic in the dashboard would create behavioral drift against the CLI's executable test spec.
**Traceability**: decision:1-swarm-architect-recommendation, decision:1-swarm-lead-dev-recommendation, decision:1-swarm-reviewer-recommendation

### Decision 2: Separate list and detail API endpoints
**Choice**: `GET /api/proposals` returns `ProposalListRow[]` (proposalId, title, targetType, createdAt, state). `GET /api/proposals/:proposalId` returns the full `ProposalDocument` + optional `ProposalReviewDecision` metadata.
**Rationale**: Follows the established Sessions API pattern (`/api/sessions` for summaries, `/api/sessions/:session_id` for full detail). Embedding full `ProposalDocument` payloads in the list response would create unnecessary payload bloat.
**Traceability**: decision:1-swarm-architect-recommendation, decision:1-swarm-reviewer-recommendation

### Decision 3: Local-only endpoints with live-mode 404
**Choice**: Both proposal endpoints return `404` with JSON error when the dashboard runs in live mode. The frontend uses `routeUnavailableMessage('proposals', meta.mode)` to display an explicit unavailable message.
**Rationale**: Proposal artifacts are local filesystem artifacts only. Following the Sessions/Events local-only pattern is consistent and prevents silent empty states in live mode.
**Traceability**: decision:1-swarm-lead-dev-recommendation, routes from `crates/scryrs-dashboard/src/server.rs:234-236`, `crates/scryrs-dashboard/frontend/src/shared/lib/dashboard-mode.ts:54-68`

### Decision 4: Detail response includes review decision metadata
**Choice**: The `/api/proposals/:proposalId` response includes an optional `reviewDecision` field (nullable) containing `ProposalReviewDecision` fields when an accepted or rejected artifact exists.
**Rationale**: While the acceptance criteria specify only proposal-authored fields (rationale, proposedContent, evidence), showing review metadata is consistent with the "inspect proposal inbox state" goal and helps reviewers understand terminal state context without drilling into separate files.
**Traceability**: decision:1-swarm-architect-recommendation (question about review decision metadata inclusion)

### Decision 5: Separate frontend routes following Sessions pattern
**Choice**: `/proposals` for list view and `/proposals/:proposalId` for detail view, with separate Vue route components, consistent with the Sessions list/detail pattern.
**Rationale**: This is the established dashboard routing convention (Sessions → `/sessions` + `/sessions/:sessionId`). It enables deep-linking to specific proposals, provides route-level code-splitting, and avoids custom state-driven selection patterns that deviate from existing views.
**Traceability**: decision:1-swarm-architect-recommendation, `crates/scryrs-dashboard/frontend/src/router/index.ts:7-37` (existing route pattern)

## Conflict Resolution

**Frontend routing: separate routes vs. single-page master/detail**

The swarm-architect and swarm-reviewer recommended following the existing Sessions pattern (separate list + detail routes at `/proposals` and `/proposals/:proposalId`), while the swarm-lead-dev recommended a single-page master/detail approach with in-page selection. The conflict is resolved in favor of separate routes because: (1) the architect's recommendation explicitly references the established Sessions pattern, which is the team's standard for list/detail navigation; (2) separate routes enable deep-linking to individual proposals, which the lead-dev acknowledged as a tradeoff; (3) separate routes provide route-level code-splitting; and (4) consistency with existing dashboard routing conventions (all list/detail pairs use separate routes) reduces implementation risk and reviewer cognitive load.

## Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| scryrs-curator gains filesystem I/O dependency, widening its scope beyond pure proposal generation | Medium | Keep the inventory module minimal (load, validate, collect) and pure-functional; avoid side-effectful write or curation logic in this module |
| Malformed-artifact failure mode: a single bad `.json` returns 502 for the entire `/api/proposals` endpoint, which may confuse users with many valid proposals | Medium | The 502 error response includes the specific file path and parse failure message so users can identify and fix the bad artifact |
| No markdown renderer exists in the frontend; proposal content may look poor in raw `<pre>` blocks | Low | Render content as plain text in `<pre>` with word-wrap (consistent with EventsView.vue); defer rich markdown rendering to follow-up |
| The `api_not_found` catch-all route at `/api/*path` could intercept proposal routes if registered after them | Low | Register `/api/proposals` and `/api/proposals/:proposal_id` before the catch-all route (as all existing routes are) |
| ProposalContent variant rendering (`Markdown` vs `SemanticGraphGrouping` vs `MemoryPatch`) needs differentiated display | Low | Render Markdown as formatted text in `<pre>`, SemanticGraphGrouping as structured fields (sourceNodeIds, targetGroupNodeId, targetGroupLabel), MemoryPatch as formatted JSON |

## Traceability

All design decisions trace back to:
- Task prompt: c02ccc65-168e-421d-8912-f4e1dba0fee8
- Dossier: 2026-07-06T00:00:37.496Z
- Decisions: 1-swarm-architect-recommendation, 1-swarm-lead-dev-recommendation, 1-swarm-reviewer-recommendation
- Round outputs: round:1:agent:swarm-architect, round:1:agent:swarm-lead-dev, round:1:agent:swarm-reviewer
- Evidence: `crates/scryrs-cli/src/proposals.rs:471-592`, `crates/scryrs-dashboard/src/server.rs:142-147`, `crates/scryrs-dashboard/frontend/src/router/index.ts:7-37`, `crates/scryrs-dashboard/frontend/src/shared/lib/dashboard-mode.ts:11-68`, `crates/scryrs-dashboard/frontend/src/stores/sessions.ts`, `crates/scryrs-dashboard/frontend/src/views/SessionsView.vue`, `crates/scryrs-dashboard/tests/api.rs:343-371`, `crates/scryrs-types/src/lib.rs:436-736`, `crates/scryrs-dashboard/Cargo.toml`, `crates/scryrs-curator/Cargo.toml`, `.pi/skills/crate-map/SKILL.md`