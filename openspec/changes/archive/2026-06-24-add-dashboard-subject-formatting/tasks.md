## 1. Backend: expose repository root

- [x] 1.1 Add `GET /api/meta` route in `crates/scryrs-dashboard/src/server.rs` returning JSON `{ repositoryPath }` from `Config.repo_root`
- [x] 1.2 Add an API test asserting `/api/meta` returns `200` with `repositoryPath` equal to the configured repo root
- [x] 1.3 Confirm `/api/hotspots` payload already carries `repositoryPath` (served raw); add/extend a test asserting the field is present

## 2. Frontend: client types & data source

- [x] 2.1 Add `repositoryPath` to the `HotspotsReport` interface in `shared/api/client.ts`
- [x] 2.2 Add a `DashboardMeta` type and `fetchMeta()` to `shared/api/client.ts`
- [x] 2.3 Add a meta/repo store that fetches `repositoryPath` once and exposes it to all views

## 3. Frontend: formatting helper

- [x] 3.1 Implement `formatSubject(subject, repoRoot, subjectKind)` in `shared/lib/` returning a structured result (`kind`, `label`, `isExternal`, `full`)
- [x] 3.2 Handle in-repo case: strip repo-root prefix (normalize trailing separator), no leading separator
- [x] 3.3 Handle external case: `isExternal` true, label = last two path segments (graceful for <2 segments)
- [x] 3.4 Handle pass-through cases: non-`file` kind, absent subject, and unknown repo root → raw value, no badge
- [x] 3.5 Add unit tests covering every spec scenario (in-repo, trailing-separator, external, single-segment external, non-file, absent, unknown root)

## 4. Frontend: wire into views

- [x] 4.1 `HotspotsView` — render shortened label (Badge for external) in list and table; keep `:title` full-path tooltip; subject stays the `subject-detail` route param
- [x] 4.2 `SubjectDetailView` — show full absolute path in `<h1>`; use helper for constellation center label
- [x] 4.3 `EventsView` — render shortened subject span with full-path `title` tooltip
- [x] 4.4 `SessionDetailView` — render shortened subject span with full-path `title` tooltip

## 5. Verification

- [x] 5.1 Run `scripts/precommit-run` (fmt, check, clippy, cargo test) for the backend changes
- [x] 5.2 Run frontend unit tests / build for the helper and views
- [x] 5.3 Manually verify on the running dashboard: in-repo path stripped, external path shows `EXTERNAL` badge + last two segments, hover tooltip reveals full path, detail heading shows full path
