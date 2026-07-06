## 1. Backend — Add accepted knowledge API endpoints

- [x] 1.1 Define `AcceptedItemListRow` DTO with `proposalId`, `title`, `targetType`, `reviewer`, `decidedAt`, `evidenceSummary`, and `publishStatus` block in `crates/scryrs-dashboard/src/server.rs`.
- [x] 1.2 Define `AcceptedItemDetail` DTO with `acceptedContent` as primary, optional `originalProposedContent`, `reviewer`, `rationale`, `evidence`, and per-surface `publishStatus` in `crates/scryrs-dashboard/src/server.rs`.
- [x] 1.3 Define `PublishStatus` DTO with an explicit enum variant per surface (`rspress`, `markdown`) containing status (`published`, `not_published`, `not_publishable`, `unknown`) and reason text.
- [x] 1.4 Implement `GET /api/accepted` handler: gate live mode to 404, load proposals + accepted decisions via `scryrs_curator::proposals::inventory::load_proposals` + `load_review_decisions(ReviewOutcome::Accepted)`, map each accepted decision + proposal into `AcceptedItemListRow`, compute `publishStatus` per surface, return sorted by `proposalId` ascending.
- [x] 1.5 Implement `GET /api/accepted/:proposalId` handler: gate live mode to 404, load proposals + accepted decisions, fail 404 if proposal ID is not found among accepted decisions, assemble `AcceptedItemDetail` with accepted content as primary truth.
- [x] 1.6 Implement Rspress publish-status detection: at query time, check if `.devagent/docs/docs/accepted-knowledge/<target-type>/<proposal-id>.md` exists on the filesystem. Map target type to slug using the same mapping as `target_type_slug()` in `scryrs-adapter-markdown`.
- [x] 1.7 Implement generic Markdown publish status as `unknown` with reason `"output root not persisted — publish path is operator-chosen at publish time"`.
- [x] 1.8 Implement `not_publishable` status for target types `memory_patch` and `semantic_graph_grouping`.
- [x] 1.9 Register `.route("/api/accepted", get(accepted_list))` and `.route("/api/accepted/:proposal_id", get(accepted_detail))` in the `router()` function.
- [x] 1.10 Map `InventoryError::OrphanReview` to `ApiError::bad_gateway` (502) via the existing `map_inventory_error` function.

## 2. Backend — Add API contract tests

- [x] 2.1 Test `GET /api/accepted` returns 200 with accepted items when `.scryrs/accepted/` fixtures are present.
- [x] 2.2 Test `GET /api/accepted` returns empty array (`[]`) 200 when accepted directory is present but empty.
- [x] 2.3 Test `GET /api/accepted` returns 404 when `.scryrs/accepted/` directory does not exist.
- [x] 2.4 Test `GET /api/accepted` returns 502 when an accepted artifact is malformed JSON.
- [x] 2.5 Test `GET /api/accepted` returns 502 when an accepted artifact has no matching proposal (orphan).
- [x] 2.6 Test `GET /api/accepted` returns 502 when conflicting accepted+rejected artifacts exist for a proposal.
- [x] 2.7 Test `GET /api/accepted/:proposalId` returns 200 with detail including acceptedContent as primary when content was overridden via review.
- [x] 2.8 Test `GET /api/accepted/:proposalId` returns 404 when proposal ID is not among accepted decisions.
- [x] 2.9 Test `GET /api/accepted` returns 404 in live mode.
- [x] 2.10 Test `GET /api/accepted/:proposalId` returns 404 in live mode.
- [x] 2.11 Test Rspress publish-status: when `.devagent/docs/docs/accepted-knowledge/<target-type>/<proposal-id>.md` exists, status is `published`.
- [x] 2.12 Test Rspress publish-status: when the expected file does not exist, status is `not_published`.
- [x] 2.13 Test non-publishable target types (`memory_patch`, `semantic_graph_grouping`) show `not_publishable` status.
- [x] 2.14 Test generic Markdown status is always `unknown`.

## 3. Frontend — Add API DTOs and client

- [x] 3.1 Add `AcceptedItemListRow`, `AcceptedItemDetail`, `PublishStatus`, `PublishSurfaceStatus` TypeScript interfaces to `crates/scryrs-dashboard/frontend/src/shared/api/client.ts`, matching Rust camelCase wire contract.
- [x] 3.2 Add `getAcceptedList(): Promise<AcceptedItemListRow[]>` client function using existing `fetchJson` pattern.
- [x] 3.3 Add `getAcceptedDetail(proposalId: string): Promise<AcceptedItemDetail>` client function.

## 4. Frontend — Add accepted knowledge Pinia store

- [x] 4.1 Create `crates/scryrs-dashboard/frontend/src/stores/accepted.ts` with `useAcceptedStore` using `defineStore`, exposing reactive `items`, `loading`, `error`, and async `fetchList()` action.
- [x] 4.2 Add `selectedItem`, `detailLoading`, `detailError`, and async `fetchDetail(proposalId: string)` action for the detail view.
- [x] 4.3 Gate `fetchList()` and `fetchDetail()` to skip API call when mode is `"live"` (frontend defense-in-depth).

## 5. Frontend — Add Accepted List view

- [x] 5.1 Create `crates/scryrs-dashboard/frontend/src/views/AcceptedListView.vue` with: table/list of accepted items showing title, targetType, reviewer, decidedAt, publish-status badges, and a link to detail.
- [x] 5.2 Render distinct publish-status badges per surface: green for `published`, gray for `not_published`, muted for `not_publishable`, and amber for `unknown` Markdown.
- [x] 5.3 Show destructive Alert for API errors (malformed, orphan, conflicting artifacts).
- [x] 5.4 Show EmptyState when no accepted items exist.
- [x] 5.5 Show "Unavailable in live mode" card via `routeUnavailableMessage()` per existing proposal/sessions/events pattern.
- [x] 5.6 Show loading state during API fetch.

## 6. Frontend — Add Accepted Detail view

- [x] 6.1 Create `crates/scryrs-dashboard/frontend/src/views/AcceptedDetailView.vue` with: accepted content as primary rendered field, reviewer metadata (reviewer name, rationale, decidedAt), evidence list, and publish-status block per surface.
- [x] 6.2 When `originalProposedContent` is present and differs from `acceptedContent`, show a comparison section or diff indicator.
- [x] 6.3 Handle error states: 404 (not found), 502 (malformed/conflicting), loading, and live-mode unavailable.

## 7. Frontend — Register routes and navigation

- [x] 7.1 Add `/accepted` route with `name: "accepted"` and lazy-loaded `AcceptedListView.vue` in `crates/scryrs-dashboard/frontend/src/router/index.ts`.
- [x] 7.2 Add `/accepted/:proposalId` route with `name: "accepted-detail"` and lazy-loaded `AcceptedDetailView.vue`.
- [x] 7.3 Add Accepted nav item to `LOCAL_NAV` in `crates/scryrs-dashboard/frontend/src/shared/lib/dashboard-mode.ts` with icon `check` and match array `["accepted", "accepted-detail"]`.
- [x] 7.4 Add `"accepted"` and `"accepted-detail"` cases to `routeUnavailableMessage()` in `dashboard-mode.ts` with descriptive messages.

## 8. Frontend — Add tests

- [x] 8.1 Create `crates/scryrs-dashboard/frontend/src/stores/accepted.test.ts`: test store dispatches API calls, loading→data transition, error propagation, live-mode skip.
- [x] 8.2 Add navigation/mode-gating test: Accepted nav item appears in `navigationForMode("local")`, absent in `navigationForMode("live")`.

## 9. Documentation

- [x] 9.1 Update `.devagent/docs/docs/proposals.md` to document the new Accepted Knowledge dashboard surface, the new `/accepted` and `/accepted/:proposalId` routes, and the accepted-versus-published boundary.
- [x] 9.2 Update `.devagent/docs/docs/cli-v0-contract.md` REST API table to include `GET /api/accepted` and `GET /api/accepted/:proposalId` endpoints.
- [x] 9.3 Document Rspress detection heuristic, generic Markdown `unknown` limitation, and non-publishable target types in both docs.

## 10. Integration verification

- [x] 10.1 Run `cargo test -p scryrs-dashboard` — all new and existing tests pass.
- [x] 10.2 Run `cargo test -p scryrs-curator` — existing inventory tests pass unchanged.
- [x] 10.3 Run `bun test` in `crates/scryrs-dashboard/frontend/` — all frontend tests pass.
- [x] 10.4 Run `bun run check` in `crates/scryrs-dashboard/frontend/` — no TypeScript errors.
- [x] 10.5 Run `cargo test -p scryrs-cli` — CLI help snapshots remain consistent.
