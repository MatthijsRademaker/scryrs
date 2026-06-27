# live-hotspot-domain-docs Specification

## Purpose

Domain documentation for live hotspots in .devagent/docs/docs/live-hotspots.md — governs page content, discoverability, cross-links, field interpretation, casing corrections, and verification against server implementation.

## Requirements

### Requirement: A dedicated domain-oriented live hotspot page exists

The system SHALL include a new documentation page at `.devagent/docs/docs/live-hotspots.md` that explains the live hotspot concept in domain terms before referencing any schema, architecture, or implementation detail.

#### Scenario: Page opens with problem statement

- **GIVEN** the project docs tree at `.devagent/docs/docs/`
- **WHEN** a reader navigates to the live-hotspots page
- **THEN** the page opens with a problem statement explaining why local-only hotspot analysis is insufficient for multi-agent teams (isolated `.scryrs/scryrs.db` files, no shared live state, need to poll or re-run `scryrs hotspots` on each machine)
- **AND** the page explains what shared live mode achieves: central ingest, idempotent shared scoring, current rankings via query API, replayable threshold-crossing signals via SSE

#### Scenario: Page explains end-to-end live workflow in domain language

- **GIVEN** the live-hotspots.md page
- **WHEN** a reader inspects the workflow section
- **THEN** the section describes: hooks/`scryrs record` submit remote batches to `scryrs server`, the server owns persistence and first-writer-wins idempotency, accepted subject-bearing events update cumulative accumulator state atomically, clients query current rankings via `GET /v1/repositories/{id}/hotspots` or consume threshold crossing signals via `GET /v1/repositories/{id}/signals`
- **AND** the description matches the implemented behavior confirmed by `openspec/specs/live-hotspot-server-contract/spec.md` and `openspec/specs/live-hotspot-accumulators/spec.md`

#### Scenario: Page clearly distinguishes live mode from local batch hotspots

- **GIVEN** the live-hotspots.md page
- **WHEN** a reader inspects the mode comparison section
- **THEN** the section distinguishes live mode from local batch along these dimensions: server-owned source of truth vs. local SQLite file, exclusive modes (no dual-write/local merge), cumulative live state vs. deterministic batch re-scoring, optional session-scoped queries via `?session_id`, and signal streaming vs. polling `.scryrs/hotspots.json`
- **AND** the distinction is grounded in `openspec/specs/live-hotspot-server-contract/spec.md` (Remote vs Local Mode Separation requirement)

#### Scenario: Page includes domain-level field interpretation for LiveHotspotsResponse

- **GIVEN** the live-hotspots.md page
- **WHEN** a reader inspects the field interpretation section
- **THEN** the section explains `score` as cumulative agent effort using the same deterministic weight table as local batch scoring
- **AND** the section explains `sessionCount` semantics in cumulative queries (distinct sessions) vs. session-scoped queries (always 1)
- **AND** the section explains `cursor` as an opaque field reserved for future use
- **AND** the section explains `evidence.rowIds` as ordered server trace event row IDs for traceability
- **AND** the section does NOT duplicate the full JSON schema, allowed values, or endpoint tables — those are deferred to `cli-v0-contract.md` with an explicit cross-reference

#### Scenario: Page includes domain-level field interpretation for HotspotSignal

- **GIVEN** the live-hotspots.md page
- **WHEN** a reader inspects the field interpretation section
- **THEN** the section explains `delta` as the score contribution of the triggering event
- **AND** the section explains `threshold` and the threshold-crossing semantics (score crossing from below to at-or-above the configured threshold)
- **AND** the section explains `evidenceRowIds` as traceability references to server event rows
- **AND** the section explains `window` as the accumulator window model (currently only `"cumulative"`)
- **AND** the section does NOT duplicate the full JSON schema — that is deferred to `cli-v0-contract.md`

### Requirement: The page is discoverable from docs navigation

The docs navigation at `.devagent/docs/docs/_nav.json` SHALL include an entry linking to the new live-hotspots page.

#### Scenario: Nav entry exists under Technical section as a sibling after Hotspots

- **GIVEN** the updated `_nav.json` file
- **WHEN** a reader opens the docs navigation
- **THEN** an entry with text "Live Hotspots" and link "/live-hotspots" appears under the "Technical" navigation section
- **AND** the entry appears after the existing "Hotspots" entry

### Requirement: The existing hotspots.md live server callout is updated to a concise cross-link

The "Related: Live Hotspot Server" section in `hotspots.md` SHALL be replaced with a concise cross-link to `live-hotspots.md`.

#### Scenario: Callout replaced with cross-link preserving adjacent-mode label

- **GIVEN** the updated `hotspots.md` file
- **WHEN** a reader encounters the live server reference
- **THEN** the former "Related: Live Hotspot Server" section (lines 117-124) is replaced with a concise cross-link paragraph stating that `scryrs server` is a separate deployment mode and directing readers to `live-hotspots.md` for the domain narrative and mode comparison
- **AND** the replacement does NOT duplicate server endpoint descriptions
- **AND** the adjacent-mode labeling required by `openspec/specs/hotspot-domain-docs/spec.md` is preserved

### Requirement: Stale server endpoint wording is corrected

The stale "HTTP server with a single POST endpoint" text at `cli-v0-contract.md` in the agent-facing Server command section SHALL be corrected to match the implemented three-endpoint surface.

#### Scenario: Single-POST-endpoint wording corrected to three REST endpoints

- **GIVEN** the updated `cli-v0-contract.md` file
- **WHEN** a reader inspects the agent-facing Server command section
- **THEN** the Output description reads "HTTP server with three REST endpoints", not "HTTP server with a single POST endpoint"
- **AND** the correction matches the accurate endpoint table earlier in the same file (lines 125-230)
- **AND** the correction matches the user-facing server help text at `crates/scryrs-cli/src/server.rs:17-28`

#### Scenario: The already-correct main command table is unchanged

- **GIVEN** the updated `cli-v0-contract.md` file
- **WHEN** a reader inspects the main `scryrs server` command table at line 132
- **THEN** the Output field still reads "HTTP server with three REST endpoints (see table below)" (unchanged, already correct)

### Requirement: subjectKind casing is corrected in live response examples

The `subjectKind` values in `LiveHotspotsResponse` and `HotspotSignal` JSON examples at `cli-v0-contract.md` SHALL use lowercase to match the actual server serialization.

#### Scenario: LiveHotspotsResponse example uses lowercase subjectKind

- **GIVEN** the updated `cli-v0-contract.md` file
- **WHEN** a reader inspects the `LiveHotspotsResponse` JSON example
- **THEN** `subjectKind` values are lowercase (e.g., `"file"`, `"search"`, `"symbol"`, `"command"`, `"document"`)
- **AND** no PascalCase subjectKind values (e.g., `"File"`, `"Search"`) appear

#### Scenario: HotspotSignal example uses lowercase subjectKind

- **GIVEN** the updated `cli-v0-contract.md` file
- **WHEN** a reader inspects the `HotspotSignal` JSON example
- **THEN** `subjectKind` values are lowercase
- **AND** the casing matches both the `LiveHotspotsResponse` example and the local `HotspotsReport` example earlier in the same file

#### Scenario: subjectKind casing is verified against server implementation

- **GIVEN** the `subject_kind()` method at `crates/scryrs-types/src/lib.rs:66-79`
- **WHEN** a reviewer compares the docs against the implementation
- **THEN** the docs lowercase casing matches the implementation's lowercase string literals (`"file"`, `"search"`, `"symbol"`, `"command"`, `"document"`)

### Requirement: Adjacent pages cross-link to the new live-hotspots page

The following pages SHALL include a cross-link to `live-hotspots.md` in their Related Pages sections: `cli-v0-contract.md`, `trace-hook-contract.md`, `architecture.mdx`, and `roadmap.mdx`.

#### Scenario: cli-v0-contract.md links to live-hotspots

- **GIVEN** the updated `cli-v0-contract.md` file
- **WHEN** a reader inspects the Related Pages section
- **THEN** a link to `./live-hotspots.md` is present with a brief description

#### Scenario: trace-hook-contract.md links to live-hotspots

- **GIVEN** the updated `trace-hook-contract.md` file
- **WHEN** a reader inspects the Related Pages section
- **THEN** a link to `./live-hotspots.md` is present with a brief description

#### Scenario: architecture.mdx links to live-hotspots

- **GIVEN** the updated `architecture.mdx` file
- **WHEN** a reader inspects the Related Pages section
- **THEN** a link to `./live-hotspots.md` is present with a brief description

#### Scenario: roadmap.mdx links to live-hotspots

- **GIVEN** the updated `roadmap.mdx` file
- **WHEN** a reader inspects the Related Pages section
- **THEN** a link to `./live-hotspots.md` is present with a brief description

### Requirement: All documentation claims are verified against source of truth

Every claim in the live-hotspots documentation page that describes live server behavior, scoring, accumulator semantics, signal behavior, or endpoint surface SHALL be verified against canonical sources before the page is considered complete.

#### Scenario: Endpoint and workflow claims match OpenSpec server contract

- **GIVEN** the live-hotspots.md page makes a claim about server endpoints or deduplication
- **WHEN** a reviewer compares the claim against `openspec/specs/live-hotspot-server-contract/spec.md`
- **THEN** the claim matches the canonical requirement scenarios

#### Scenario: Accumulator and signal claims match OpenSpec accumulator spec

- **GIVEN** the live-hotspots.md page makes a claim about cumulative accumulators or threshold-crossing signals
- **WHEN** a reviewer compares the claim against `openspec/specs/live-hotspot-accumulators/spec.md`
- **THEN** the claim matches the canonical requirement scenarios

#### Scenario: Endpoint surface matches server help text

- **GIVEN** the live-hotspots.md page references the server endpoint surface
- **WHEN** a reviewer compares the reference against `crates/scryrs-cli/src/server.rs:17-28`
- **THEN** the three endpoints (`POST /v1/trace-events/batch`, `GET /v1/repositories/{id}/hotspots`, `GET /v1/repositories/{id}/signals`) match

### Requirement: The docs site builds successfully

The Rspress documentation site SHALL build successfully from `.devagent/docs/` with no broken links or build errors after adding the new page, updating navigation, and modifying existing pages.

#### Scenario: Docs build completes without errors

- **GIVEN** the updated docs tree including live-hotspots.md, modified _nav.json, modified hotspots.md, modified cli-v0-contract.md, and modified cross-link target pages
- **WHEN** `bun run build` is executed in `.devagent/docs/`
- **THEN** the build completes with exit code 0
- **AND** no broken link warnings appear in the build output
- **AND** the new `/live-hotspots` route is present in the generated site

### Requirement: No production code, OpenSpec specs, or non-doc artifacts are modified

This change SHALL NOT modify any Rust source code, Cargo configuration, OpenSpec specification files (outside of this change's own specs directory), LikeC4 architecture diagrams, test fixtures, CI configuration, or the root README.md.

#### Scenario: Only documentation files under .devagent/docs/docs/ are changed

- **GIVEN** the diff of this change
- **WHEN** a reviewer inspects changed files
- **THEN** all changed files are under `.devagent/docs/docs/`
- **AND** no files in `crates/`, `openspec/specs/`, `.devagent/architecture/`, `.github/`, or the repository root (except this change's OpenSpec artifacts) are modified

#### Scenario: Server behavior is unchanged

- **GIVEN** the existing test suite for scryrs-server and scryrs-core
- **WHEN** tests are run after this documentation change
- **THEN** all tests pass with identical results as before the change
