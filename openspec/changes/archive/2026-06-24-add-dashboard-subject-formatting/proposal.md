## Why

The dashboard renders trace `subject` values as raw absolute filesystem paths (e.g. `/Users/matthijsrademaker/repos/dignitas/cl-sessions/dignitas-agentic-docs/openspec/changes/agentic-docs-presentation/proposal.md`). These are unreadable in lists, tables, and event rows — the meaningful part of the path is buried under the user's home directory and repository prefix, and externally-sourced files are indistinguishable from in-repo files at a glance.

## What Changes

- Introduce a presentation-only formatting layer that shortens `subject` paths for display while keeping the raw `subject` as the canonical identity (routing key, dedup key) — no contract change to stored events or hotspot entries.
- **In-repo files** (path under the repository root): strip the repository-root prefix, showing a repo-relative path (e.g. `.devagent/doc_build/architecture.md`).
- **External files** (absolute `file` path outside the repository root): render an `EXTERNAL` badge chip followed by the last two path segments (e.g. `[EXTERNAL] agentic-docs-presentation/proposal.md`).
- **Non-file subjects** (e.g. `routing`, `topic`) and null/lifecycle subjects: rendered unchanged.
- Reveal the full absolute path on demand: a hover tooltip (`title`) on every shortened label across all views, and the full absolute path always shown in the `SubjectDetailView` heading.
- Add a lightweight `GET /api/meta` endpoint exposing `{ repositoryPath }` so the Events and Session-detail views (which never fetch the hotspot report) can resolve the repository root and strip it too.
- Expose `repositoryPath` on the `HotspotsReport` type already returned by `/api/hotspots`.

## Capabilities

### New Capabilities
- `dashboard-subject-formatting`: Defines how the dashboard transforms raw `subject` paths into human-readable display labels (in-repo stripping, external-file badging, full-path reveal) and the `/api/meta` repository-root endpoint that supports it across all views.

### Modified Capabilities
<!-- No existing spec's requirements change; the formatting layer is purely additive presentation. -->

## Impact

- **Frontend** (`crates/scryrs-dashboard/frontend/src/`):
  - New `shared/lib/` helper `formatSubject(subject, repoRoot, subjectKind)` (unit-testable, the entire decision brain).
  - New meta/repo store fetching `repositoryPath` once and sharing it across views.
  - `shared/api/client.ts`: add `repositoryPath` to `HotspotsReport`; add `fetchMeta()` and a `DashboardMeta` type.
  - Consumers updated to use the helper + tooltips: `HotspotsView` (list + table labels), `SubjectDetailView` (full path in `<h1>`, helper for constellation center label), `EventsView`, `SessionDetailView`.
  - External label uses the existing shadcn-vue `Badge` component.
- **Backend** (`crates/scryrs-dashboard/src/server.rs`): new `GET /api/meta` route returning `{ repositoryPath }` from `Config.repo_root`.
- **No changes** to the trace event schema, hotspot report schema, database, or stored `subject` values.
