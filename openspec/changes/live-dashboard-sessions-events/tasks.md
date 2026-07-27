## 1. Server contract and storage

- [x] 1.1 Define repository-scoped session summary, session detail, event page, and cursor DTOs.
- [x] 1.2 Add server-store queries over `server_trace_events` for sessions, session detail, and paginated events.
- [x] 1.3 Add repository/session/event indexes and verify query plans under concurrent ingest.
- [x] 1.4 Add server routes with repository scoping, bounded limits, cursor validation, and cross-repository not-found behavior.
- [x] 1.5 Add server unit and HTTP tests for ordering, pagination, empty results, malformed cursors, and repository isolation.

## 2. Dashboard backend

- [x] 2.1 Add live proxy handlers for `/api/sessions`, `/api/sessions/:sessionId`, and `/api/events`.
- [x] 2.2 Preserve local handlers and ensure live handlers never read local `.scryrs` artifacts.
- [x] 2.3 Add upstream error mapping and response-contract tests.

## 3. Dashboard frontend

- [x] 3.1 Extend live-mode API clients/stores for sessions, session detail, and cursor-paginated events.
- [x] 3.2 Enable Sessions and Events in live navigation while retaining Routes and Proposals gating.
- [x] 3.3 Render loading, empty, pagination, upstream error, and direct-route states for live sessions/events.
- [x] 3.4 Add Vitest coverage for live stores, cursor behavior, and mode-aware navigation.

## 4. Verification and documentation

- [x] 4.1 Add live dashboard integration coverage against a seeded server database.
- [x] 4.2 Run Rust, frontend, and Docker-backed repository verification lanes.
- [x] 4.3 Document live sessions/events endpoints, pagination, and local-vs-live behavior.
