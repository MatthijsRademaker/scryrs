## Context

The scryrs dashboard currently exposes hotspots, sessions, events, and live signals through a Vue 3 SPA backed by an Axum server. Route explanations exist as deterministic Rust logic in `scryrs-runtime` (`explain_hints`) and are available via `scryrs route explain --query <TEXT>`, but have no dashboard surface. This design adds a local-only Route explain view that reuses the existing deterministic ranking without introducing new matching behavior, model-based ranking, or artifact mutation.

The dashboard backend currently depends on `scryrs-types` (shared contract types) but not `scryrs-runtime`. Adding `scryrs-runtime` is safe — its only workspace dependency is `scryrs-types`, which is already a dashboard dependency.

The dashboard backend follows consistent error patterns: missing artifacts → `ApiError::missing()` (404), corrupt/unreadable artifacts → `ApiError::bad_gateway()` (502), live-mode gating → `ApiError::missing()` (404). The Route explain endpoint follows this same pattern.

The frontend uses hand-written typed API clients, Pinia stores for reactive state, and Vue Router with lazy-loaded views. The Route view follows the existing card/alert/empty-state patterns established by EventsView and SessionsView.

## Goals / Non-Goals

### Goals
- Expose route explain in the dashboard as a read-only browser flow backed by `scryrs_runtime::explain_hints`.
- Show match results with rank, relevance, target, load-target context, reason, and evidence citations from `RouteHintDocument`.
- Provide first-class missing/malformed artifact states with actionable CLI remediation guidance.
- Respect dashboard mode boundaries: local first, no hidden local fallback in live mode.
- Add focused automated coverage: Rust API contract tests + TypeScript store/client tests.
- Update CLI help surfaces to prevent public API documentation drift.

### Non-Goals
- Do not regenerate `.scryrs/routes.json`, mutate artifacts, or shell out to CLI commands from the dashboard.
- Do not invent new ranking, fuzzy matching, or model-based explanation behavior.
- Do not add live route-manifest export to `scryrs-server`.
- Do not build a broader graph/proposal explorer.
- Do not add `@vue/test-utils` or component-mount tests — store/client-level tests suffice for v1.

## Decisions

### 1. Backend rejects empty query with HTTP 400; frontend also gates on non-empty input
`explain_hints` with empty string matches every route entry via empty-string substring containment. To prevent unbounded result serialization, the backend returns HTTP 400 for empty `?query=`. The frontend additionally disables search submission until non-empty input is entered, providing defense in depth. This resolves the empty-query ambiguity flagged by all three refinement reviewers.

### 2. Manifest-loading helper lives in the dashboard server, not shared
CLI `route_common.rs` uses `writeln!` to stderr and returns exit codes — incompatible with the dashboard's `ApiError` HTTP pattern. A standalone load-and-validate helper in the dashboard server maps missing → 404, malformed JSON → 502, and schema mismatch → 502. Deliberate duplication over `route_common.rs` is documented in code comments.

### 3. Evidence renders as human-readable citations, not clickable links
Local trace row IDs have no browsable URL target in the dashboard. Evidence citations render as `sourceKind` + `subject` + `rowIds` plain text following the EventsView `payloadPreview` pattern.

### 4. Frontend tests are store-level (Vitest only), no `@vue/test-utils`
`package.json` lacks `@vue/test-utils` and the existing frontend test suite uses Vitest + Pinia mock patterns (`hotspots.test.ts`, `signals.test.ts`). Store/client-level tests cover loading, error, and data states. Component-mount tests are deferred.

### 5. CLI help surfaces updated synchronously
`dashboard.rs`, `help_json.rs`, `help_text.rs`, and their insta snapshots are updated to document `GET /api/routes/explain` in the same change.

### 6. Live-mode: nav hidden, backend returns 404, direct URL shows unavailable card
Following sessions/events pattern: no Route nav entry in `LIVE_NAV`, backend returns `ApiError::missing("route explain unavailable in live mode")` for `/api/routes/explain` in live mode, and the `RouteView` shows an explicit "Unavailable in live mode" card via `routeUnavailableMessage()`.

## Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| `scryrs-runtime` adds unexpected transitive deps | Low | Only depends on `scryrs-types` (already a dashboard dep); verified by `scryrs-runtime/Cargo.toml`. |
| Large manifest with short query produces many results | Low | Empty-query guard (decision 1) prevents accidental "match all". Short queries with many substring matches are expected and bounded by manifest size. No pagination in v1. |
| Live-mode direct `/routes` URL access shows confusing state | Low | Same pattern as sessions/events — card explains unavailability. Navigation hides entry, only bookmarked/history URLs hit this. |
| CLI help surfaces go stale if not updated synchronously | Low | Explicitly included in tasks. Snapshot tests catch drift. |
| Manifest-loading logic drifts between CLI and dashboard | Low | Deliberate duplication accepted per decision 2. Code comments document the relationship. |

## Traceability

- **Task**: `0e9b7d87-f856-4ad5-ad37-05030621da93` — Dashboard Product 03 feature description, scenarios, and acceptance criteria.
- **Exploration Dossier**: `2026-07-05T23:43:17.413Z` — Problem framing, goals, non-goals, affected areas, and consulted sources.
- **Decision: 1-swarm-architect-recommendation** — Proceed; extract manifest validation, reject empty queries, add tests, update CLI help.
- **Decision: 1-swarm-lead-dev-recommendation** — Proceed with standalone manifest helper, frontend query gating, plain-text evidence, store-level tests.
- **Decision: 1-swarm-reviewer-recommendation** — Proceed; five issues resolved by design decisions above.
- **Spec: openspec/specs/route-explain** — Deterministic explain matching, ordering, output contract.
- **Spec: openspec/specs/route-hint** — `RouteHintDocument`/`RouteHintItem` wire contract.
- **Spec: openspec/specs/dashboard-frontend-stack** — Hand-written typed API clients, Vitest/Bun conventions.
- **Spec: openspec/specs/live-dashboard-mode** — Same-origin API, live/local separation.