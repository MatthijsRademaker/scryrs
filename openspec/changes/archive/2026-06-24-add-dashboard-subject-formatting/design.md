## Context

`subject` is stored per trace-event (`subjectKind` + `subject`) and aggregated per hotspot entry. For `subjectKind == "file"` the value is a raw absolute filesystem path. The dashboard renders it verbatim in four places: `HotspotsView` (list + table, each a `RouterLink` to `subject-detail`), `SubjectDetailView` (`<h1>` heading + constellation center label), `EventsView` (inline span per row), and `SessionDetailView` (inline span per row).

The repository root is already known to the backend (`Config.repo_root`) and is written into `.scryrs/hotspots.json` as `repositoryPath`. The `/api/hotspots` handler serves that file's raw bytes, so the field already reaches the client — it is simply absent from the `HotspotsReport` TypeScript type. The Events and Session-detail views, however, query the database directly and never see a repository root.

The `subject` value is load-bearing as an identity: it is the route param for `subject-detail` and the dedup/group key for hotspot aggregation. Display formatting must not mutate it.

## Goals / Non-Goals

**Goals:**
- Replace raw absolute paths in the UI with readable labels (repo-relative for in-repo files, `EXTERNAL` badge + tail for external files).
- Keep the full absolute path one hover away everywhere, and always visible on the detail page.
- Make the repository root available to every view, including Events and Session detail.
- Keep `subject` unchanged as the canonical identity; formatting is pure presentation and fully unit-testable in isolation.

**Non-Goals:**
- No change to the trace event schema, hotspot report schema, database, or stored `subject` values.
- No backend-computed display field (`displaySubject`) — formatting stays in the frontend.
- No collision-proofing of external labels; identical-looking tails are acceptable because the full path is always reachable via tooltip/detail.
- No Windows path-separator handling beyond best-effort (this repo targets POSIX paths).

## Decisions

### Decision: Formatting lives in a single frontend helper, not the backend

A pure function `formatSubject(subject, repoRoot, subjectKind)` in `shared/lib/` holds the entire decision tree and returns a structured result (e.g. `{ kind: 'internal' | 'external' | 'raw', label, isExternal, full }`) so views can render a `Badge` for external entries without re-parsing strings.

- **Why over backend:** `subject` must remain the canonical identity (routing/dedup key). Computing the label backend-side would mean adding a parallel `displaySubject` to the events contract as well as hotspots — more surface area for a cosmetic concern. A pure frontend function is trivially unit-testable and has zero contract impact.
- **Alternative considered:** backend `displaySubject` field — rejected for contract bloat and duplicated identity.

### Decision: Repository root delivered via a new `GET /api/meta` endpoint + a shared store

A tiny `GET /api/meta` returns `{ repositoryPath }` from `Config.repo_root`. A frontend meta/repo store fetches it once and shares it across all views. `repositoryPath` is additionally added to the `HotspotsReport` type (already present in the payload).

- **Why:** Events and Session-detail views have no repository root today. A single endpoint + store is one source of truth for all four views, rather than threading the value out of the hotspot report (which those views do not fetch).
- **Alternatives considered:** (a) strip only on Hotspots views — rejected, leaves Events showing raw absolute paths; (b) embed repo root in `index.html` at serve time — rejected as less testable and couples HTML rendering to config.

### Decision: External label = `EXTERNAL` badge + last two path segments

External `file` subjects render the existing shadcn-vue `Badge` component with text `EXTERNAL`, followed by the final two path segments (parent dir + filename).

- **Why:** Basename-only (`proposal.md`) collapses common filenames indistinguishably; the parent segment disambiguates the overwhelming majority of real cases while staying short. A badge reads more clearly than literal `<EXTERNAL>` angle-bracket text and visually separates external from in-repo entries.

### Decision: Full-path reveal = tooltip everywhere + full path in detail heading

Every shortened label carries a `title` attribute with the full absolute path (`HotspotsView` already uses `:title` on its link). `SubjectDetailView`'s `<h1>` shows the full absolute path rather than the shortened form.

- **Why:** Hover gives a zero-layout-cost peek in dense lists/rows; the detail page is the natural place for the authoritative full value. Together they cover both quick scanning and deliberate inspection.

## Risks / Trade-offs

- **Repo root not yet loaded when a view first renders** → `formatSubject` falls back to the raw subject value (no error, no badge); the store hydrates and labels shorten on next render.
- **External tail collisions** (two different `proposal.md` look identical) → accepted; full path is always available via tooltip and detail heading.
- **Trailing-separator / path normalization mismatches** → normalize the repository root (strip a trailing separator) before prefix comparison; covered by a spec scenario and unit tests.
- **Non-absolute or non-`file` subjects misclassified as external** → only `subjectKind == "file"` is formatted; everything else renders raw, so non-path subjects can never acquire a badge.
