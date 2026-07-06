## 1. Extract proposal inventory logic to scryrs-curator

- [ ] 1.1 Create `scryrs-curator/src/proposals/inventory.rs` with public `load_proposals()`, `load_review_decisions()`, `collect_list_rows()`, `validate_proposal_document()`, and `validate_review_decision_artifact()` extracted from `scryrs-cli/src/proposals.rs`.
- [ ] 1.2 Define a shared `InventoryError` type (replacing CLI-specific `CommandError`) covering filesystem read failures, JSON parse failures, validation failures, duplicate IDs, and conflicting terminal states.
- [ ] 1.3 Re-export inventory functions from `scryrs-curator/src/proposals/mod.rs` and `scryrs-curator/src/lib.rs`.
- [ ] 1.4 Update `scryrs-cli/src/proposals.rs` to import and re-export inventory functions from `scryrs-curator::proposals::inventory`, mapping `InventoryError` to `CommandError`.
- [ ] 1.5 Run `scripts/test scryrs-cli` to verify existing CLI proposal tests still pass with the extracted logic.
- [ ] 1.6 Run `scripts/check` to verify no compilation or lint regressions.

## 2. Add proposal API endpoints to scryrs-dashboard

- [ ] 2.1 Add `scryrs-curator` as a production dependency in `crates/scryrs-dashboard/Cargo.toml`.
- [ ] 2.2 Define `ProposalListRow` and `ProposalDetail` response DTOs in the dashboard server module, mirroring `scryrs-types` `ProposalDocument` and `ProposalReviewDecision` fields.
- [ ] 2.3 Implement `GET /api/proposals` handler: local-only gate (404 in live mode), calls `scryrs-curator::proposals::inventory::collect_list_rows()`, returns `ProposalListRow[]` sorted by proposalId ascending, maps `InventoryError` to `ApiError` (404 for missing directory, 502 for parse/validation/conflict failures).
- [ ] 2.4 Implement `GET /api/proposals/:proposal_id` handler: local-only gate, loads the specific proposal document plus optional review decision from `.scryrs/accepted/` or `.scryrs/rejected/`, returns `ProposalDetail` with `ProposalDocument` fields and optional `reviewDecision`.
- [ ] 2.5 Register `/api/proposals` and `/api/proposals/:proposal_id` in `server.rs` router before the `api_not_found` catch-all route.
- [ ] 2.6 Add backend integration tests in `crates/scryrs-dashboard/tests/api.rs` covering: successful list with pending/accepted/rejected states, successful detail with and without review decision, 404 when `.scryrs/proposals/` missing, 502 when proposal JSON is malformed, 502 when conflicting accepted+rejected artifacts exist, 404 in live mode.
- [ ] 2.7 Run `scripts/test scryrs-dashboard` to verify all backend tests pass.

## 3. Add frontend proposal views, store, and client

- [ ] 3.1 Add proposal TypeScript DTOs to `crates/scryrs-dashboard/frontend/src/shared/api/client.ts` (`ProposalListRow`, `ProposalDetail`, `ProposedContent`, `EvidenceLink`, `ProposalReviewDecisionMeta`) and typed fetch functions (`getProposals()`, `getProposal(proposalId)`).
- [ ] 3.2 Create `crates/scryrs-dashboard/frontend/src/stores/proposals.ts` following the sessions store pattern: Pinia `defineStore` with refs for `rows`, `detail`, `loading`, `error`, and async `loadProposals`/`loadProposal` actions.
- [ ] 3.3 Create `crates/scryrs-dashboard/frontend/src/views/ProposalsView.vue`: list view displaying proposal rows (proposalId truncated, title, targetType, createdAt, state badge) with destructive Alert for errors, EmptyState for no proposals, Card-based row rendering, RouterLink to detail route.
- [ ] 3.4 Create `crates/scryrs-dashboard/frontend/src/views/ProposalDetailView.vue`: detail view showing rationale as text, proposedContent as variant-aware `<pre>` (Markdown as text, SemanticGraphGrouping as structured fields, MemoryPatch as formatted JSON), evidence links as structured list (sourceKind, subject, rowIds), optional reviewDecision section (outcome, reviewer, decidedAt, rationale).
- [ ] 3.5 Register `/proposals` and `/proposals/:proposalId` routes in `crates/scryrs-dashboard/frontend/src/router/index.ts`.
- [ ] 3.6 Add Proposals nav entry to `LOCAL_NAV` in `crates/scryrs-dashboard/frontend/src/shared/lib/dashboard-mode.ts` (with a new `icon` value or reusing an existing one) and add `routeUnavailableMessage` entries for `proposals` and `proposal-detail`.
- [ ] 3.7 Update `crates/scryrs-dashboard/frontend/src/shared/lib/dashboard-mode.test.ts` to assert Proposals is in local nav and absent from live nav, and that live-mode unavailable messages render.
- [ ] 3.8 Run `bun run check` and `bun run test` in `crates/scryrs-dashboard/frontend/` to verify TypeScript compilation and frontend tests pass.

## 4. Update documentation

- [ ] 4.1 Update `.devagent/docs/docs/cli-v0-contract.md` to document the new `/api/proposals` and `/api/proposals/:proposalId` endpoints in the dashboard REST contract.
- [ ] 4.2 Update `.devagent/docs/docs/proposals.md` to note the dashboard review flow now exists (remove the "No dashboard review flow exists yet" limitation).
- [ ] 4.3 Update `.pi/skills/crate-map/SKILL.md` if needed to reflect scryrs-curator's new inventory module.

## 5. Final verification

- [ ] 5.1 Run `scripts/check` to verify workspace-wide compilation, clippy, and formatting.
- [ ] 5.2 Run `scripts/test` to verify all workspace tests pass.
- [ ] 5.3 Run `bun run build` in `crates/scryrs-dashboard/frontend/` to verify production frontend build succeeds.
- [ ] 5.4 Verify no writes are performed: audit all new code paths for any mutation of `.scryrs/proposals/`, `.scryrs/accepted/`, `.scryrs/rejected/`, `.scryrs/graph.json`, or other persistent state.