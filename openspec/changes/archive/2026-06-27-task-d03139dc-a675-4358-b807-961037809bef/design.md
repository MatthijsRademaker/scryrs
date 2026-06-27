## Context

Live hotspot coverage currently exists across five project docs pages, but every reference is architecture or contract framed:

| Page | Current live coverage | Gap |
|------|----------------------|-----|
| `hotspots.md:117-124` | 7-line "Related: Live Hotspot Server" callout | No domain narrative; duplicates endpoint descriptions |
| `cli-v0-contract.md:125-230` | Correct three-endpoint REST API table | Contract-heavy, no problem/outcome framing |
| `cli-v0-contract.md:419` | Stale "single POST endpoint" wording | Contradicts the correct table at line 132 |
| `architecture.mdx:135-138` | Live accumulator and endpoint bullets | Framed as crate responsibilities, not user outcomes |
| `roadmap.mdx:136-156` | Phase 4 deliverables and limitations | Describes what must be built, not when to use it |
| `trace-hook-contract.md:413-459` | Remote ingestion envelope and identity rules | Transport contract, not domain story |

Additionally:
- `_nav.json` has no live-hotspot entry — live content is undiscoverable from navigation
- `cli-v0-contract.md:206` and `:235` show `subjectKind` as PascalCase (`"File"`, `"Search"`) but the actual server serializes lowercase (`"file"`, `"search"`) as confirmed by `crates/scryrs-types/src/lib.rs`

The `hotspots.md` domain page (created under `openspec/specs/hotspot-domain-docs/spec.md`) provides the precedent pattern: domain-first narrative, field interpretation, decision guidance, and adjacent-mode callout — with full schemas deferred to contract pages. This change applies that same pattern to live hotspots.

## Goals / Non-Goals

### Goals

1. Add a discoverable, domain-first live-hotspot documentation page that opens with the user problem and outcome: why shared live hotspots exist, what they solve, and who benefits.
2. Explain the end-to-end live workflow in domain language: hooks/`scryrs record` submit remote batches, `scryrs server` owns persistence and deduplication, accepted subject-bearing events update cumulative state, and readers can query current hotspots or consume signals.
3. Clearly distinguish live mode from local batch hotspots along key dimensions: server-owned source of truth, no dual-write/local merge, cumulative live state, session-scoped queries, and signal streaming vs. polling.
4. Improve discoverability and consistency: add nav entry, cross-link from adjacent pages, fix stale/conflicting server wording, correct subjectKind casing.
5. Verify all field and behavior claims against canonical OpenSpec specs (`live-hotspot-server-contract`, `live-hotspot-accumulators`) and the server implementation.

### Non-Goals

- Changing Rust/server behavior, APIs, schemas, scoring, or signal semantics.
- Reworking local hotspot documentation beyond the minimum cross-links and callout update.
- Duplicating full JSON schemas and endpoint tables that already belong in `cli-v0-contract.md`.
- Editing canonical OpenSpec specs outside this change.
- Modifying README.md, Rust help text in `server.rs`, or any source code.
- Reconciling every remote/server wording inconsistency beyond the two identified fixes.
- Adding websocket transport, graph, proposal, route, or runtime retrieval API documentation.

## Decisions

### Decision 1: Dedicated live-hotspots.md page (mirroring hotspots.md pattern)

**What:** Create a new `.devagent/docs/docs/live-hotspots.md` page following the domain-first structure of `hotspots.md`: problem statement → what it achieves → end-to-end workflow → mode comparison → field interpretation → cross-links.

**Why:** The existing `hotspots.md` callout at lines 117-124 is 7 lines and cannot contain the domain depth needed. A dedicated page gives live mode the same narrative treatment local hotspots already have. This was unanimously endorsed by architect and lead-dev (round 1).

**Alternatives considered:** Expanding `hotspots.md` to cover live mode inline. Rejected because mixing two deployment modes with fundamentally different operational models (local SQLite vs. shared server) within a single page would confuse readers and violate the `hotspot-domain-docs` spec's requirement that live features be labeled as adjacent.

### Decision 2: Nav placement — sibling under Technical, after Hotspots

**What:** Add `{ "text": "Live Hotspots", "link": "/live-hotspots" }` as a sibling entry under the Technical nav section, positioned immediately after the existing "Hotspots" entry.

**Why:** The reviewer (round 1) identified nav placement as a blocking gap. The sibling structure under Technical places live hotspots alongside the existing Hotspots entry for discoverability while the distinct label ("Live Hotspots") signals it is a deployment-mode extension, not a peer abstraction. Placing it as a child of Hotspots would hide it from navigation; omitting it entirely would violate the acceptance criteria requiring discoverability.

### Decision 3: Replace hotspots.md callout with concise cross-link

**What:** Replace the "Related: Live Hotspot Server" section in `hotspots.md` (lines 117-124) with a single cross-link paragraph:

> **Related: Live Hotspot Server** — The live hotspot server (`scryrs server`) is a separate deployment mode that provides central ingestion, shared live state, and signal streaming for multi-agent teams. See [Live Hotspots](./live-hotspots.md) for the domain narrative and mode comparison.

**Why:** The reviewer (round 1) identified the undecided fate of this section as a blocking gap. The current 7-line callout duplicates server endpoint descriptions that now belong in `live-hotspots.md`. Trimming it to a concise cross-link preserves the adjacent-mode label required by the `hotspot-domain-docs` spec while directing readers to the authoritative domain page.

### Decision 4: Fix stale wording at cli-v0-contract.md:419 only

**What:** Correct line 419 from "HTTP server with a single POST endpoint" to "HTTP server with three REST endpoints".

**Why:** The architect (round 1) flagged this as a must-fix inconsistency. The lead-dev (round 1) scoped the fix to line 419 only. The reviewer (round 1) asked for both line 132 and line 419 to be confirmed. Line 132 already reads "HTTP server with three REST endpoints (see table below)" — it is already correct and requires no change. Only line 419 (the agent-facing Server command section) contains the stale wording. This is a narrow, high-confidence correction.

### Decision 5: Fix subjectKind casing to lowercase in live response examples

**What:** Change `"subjectKind": "File"` to `"subjectKind": "file"` and `"subjectKind": "Search"` to `"subjectKind": "search"` in the `LiveHotspotsResponse` and `HotspotSignal` JSON examples at `cli-v0-contract.md:206` and `cli-v0-contract.md:235`.

**Why:** The reviewer (round 1) identified this as a blocking inconsistency. Investigation confirmed the actual server serializes lowercase: `crates/scryrs-types/src/lib.rs` `subject_kind()` returns `Some("file")`, `Some("search")`, etc., and the Rust test at line 1087 asserts `parsed["subjectKind"] == "file"`. The PascalCase in the docs is a documentation error. Fixing to lowercase aligns the docs with the implementation and removes confusion when readers compare local and live hotspot output format.

### Decision 6: Cross-link targets

**What:** Add `[Live Hotspots](./live-hotspots.md)` links to the Related Pages sections of:

| Page | Cross-link placement |
|------|---------------------|
| `hotspots.md` | Replace existing callout section with cross-link paragraph |
| `cli-v0-contract.md` | Add to existing Related Pages section after the Hotspots link |
| `trace-hook-contract.md` | Add to existing Related Pages section after the Hotspots link |
| `architecture.mdx` | Add to existing Related Pages section after the Hotspots link |
| `roadmap.mdx` | Add to existing Related Pages section |

**Why:** All three refinement agents (architect, lead-dev, reviewer) agreed cross-links are required. The reviewer specifically identified these five pages as cross-link targets. Each page currently references live hotspot concepts but lacks a link to a domain narrative page.

### Decision 7: Field interpretation boundary

**What:** The `live-hotspots.md` page SHALL include domain-level field interpretation for `LiveHotspotsResponse` and `HotspotSignal` only where it adds understanding beyond the contract schema. Specifically:
- Score: cumulative agent effort, matching the same weight table as local batch scoring
- Score delta semantics (HotspotSignal): the contribution of the triggering event
- Threshold-crossing meaning: score crossing from below to at-or-above the configured threshold
- Session-scoped vs. cumulative query semantics: `?session_id` recomputes from raw events; defaults to accumulator state
- `cursor` field: opaque, reserved for future use

Full JSON schemas, allowed values, endpoint tables, and exit codes remain in `cli-v0-contract.md`.

**Why:** The architect (round 1) recommended field-level interpretation where it adds domain understanding. The `hotspots.md` precedent handles this by explaining behavioral semantics without duplicating the contract page's wire format. The live page should do the same.

## Risks

| Risk | Likelihood | Mitigation |
|------|-----------|------------|
| Scope creep into full info-architecture redesign | Medium | Explicit non-goals section; only the identified five cross-link targets and two stale-wording fixes are in scope |
| Field-level over-interpretation duplicating contract details | Low | Follow `hotspots.md` precedent: behavioral semantics only, no wire format duplication |
| Cross-link drift (missing or broken link in one of five pages) | Low | Exhaustive list of cross-link targets in tasks; docs build verification catches broken links |
| Navigation confusion (Live Hotspots perceived as peer abstraction) | Low | Distinct label "Live Hotspots" signals deployment-mode extension; cross-page prose reinforces the relationship |
| subjectKind casing fix introduces drift if docs are regenerated | Low | Fix targets the authoritative doc source; no auto-generation pipeline exists to revert it |

## Traceability

| Source | Evidence |
|--------|----------|
| Task prompt | "live hotspot docs — Currently the live hotspot docs are very architecture driven but not domain driven" |
| Dossier | Acceptance criteria, affected areas, non-goals, proposal sketch |
| Round 1: Architect | Endorsed proposal sketch, identified stale wording blocker, nav placement question, field interpretation boundary |
| Round 1: Lead dev | Endorsed with refinements (end-to-end workflow diagram, scope stale fix to line 419) |
| Round 1: Reviewer | Identified 4 precision gaps (nav placement, callout handling, stale wording scope, subjectKind casing) |
| `openspec/specs/hotspot-domain-docs/spec.md` | Precedent pattern for domain-first docs structure |
| `openspec/specs/live-hotspot-server-contract/spec.md` | Canonical three-endpoint contract, deduplication, remote/local separation |
| `openspec/specs/live-hotspot-accumulators/spec.md` | Cumulative accumulators, threshold crossings, batch/live alignment |
| `crates/scryrs-cli/src/server.rs:6-31` | User-facing help confirms three-endpoint surface |
| `crates/scryrs-types/src/lib.rs:66-79` | `subject_kind()` returns lowercase tags |
| `crates/scryrs-types/src/lib.rs:1087` | Test asserts `parsed["subjectKind"] == "file"` |