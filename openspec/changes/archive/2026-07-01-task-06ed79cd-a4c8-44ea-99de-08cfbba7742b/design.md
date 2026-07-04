## Context

`scryrs hotspots <PATH>` currently materializes `.scryrs/hotspots.json` only from local `.scryrs/scryrs.db` state, while `scryrs server` already exposes cumulative live rankings through `GET /v1/repositories/{repository_id}/hotspots?window=cumulative` as `LiveHotspotsResponse`. Downstream commands (`graph`, `route`, `propose`) already consume the hotspot artifact rather than querying the server directly. The missing contract is a narrow live export path that turns the live response into the existing local artifact shape without silently reading or blending local SQLite data.

Refinement converged on extending the existing `hotspots` command instead of adding a broader new pipeline, preserving the current `HotspotsReport` schema, validating remote failures loudly, and proving downstream compatibility with smoke coverage.

## Goals / Non-Goals

### Goals

- Add an explicit live export path on `scryrs hotspots <PATH>`.
- Preserve the existing `HotspotsReport` envelope so `graph`, `route`, and `propose` run unchanged after export.
- Make the live source identity explicit in both artifact metadata and command output.
- Fail before replacing `.scryrs/hotspots.json` when the live server is unreachable, returns a non-2xx status, returns malformed JSON, reports the wrong schema version, or reports the wrong repository identity.
- Keep local and live sources mutually exclusive and test that boundary.

### Non-Goals

- Changing the live server ranking algorithm, ingest API, accumulator schema, or SSE behavior.
- Adding background synchronization, polling, or watch behavior for `.scryrs/hotspots.json`.
- Changing downstream commands to accept new live-only flags.
- Merging local SQLite evidence with server-owned evidence.
- Teaching graph output to relabel live evidence as a new source kind in this change.

## Decisions

### Decision 1: Extend `scryrs hotspots` with explicit live mode

Add `--mode local|live`, `--server-url`, and `--repository-id` to `scryrs hotspots <PATH>`. Omitted `--mode` keeps the current local path; `--mode live` activates the remote export path.

**Rationale**: refinement consistently favored the narrowest public contract and identified the existing `hotspots` command as the simplest place to add export behavior.

### Decision 2: Reuse the existing remote identity precedence chain for live mode

When `--mode live` is selected, resolve server URL and repository ID through: CLI flags → process environment (`SCRYRS_REMOTE_INGEST_URL`, `SCRYRS_REPOSITORY_ID`) → `.scryrs/.env` → `scryrs.json remote`.

**Rationale**: this resolves the refinement-room disagreement in favor of the accepted reviewer recommendation and matches the remote-target resolution patterns already cited in the dossier and architect rationale.

### Decision 3: Live export uses a separate code path that never opens local SQLite

Route live mode to a dedicated helper that fetches the cumulative hotspot endpoint with `ureq`, validates the response, converts it to `HotspotsReport`, and writes the artifact. The live branch must not call `TraceQuery::open`, read `.scryrs/scryrs.db`, or score local events.

**Rationale**: the architectural boundary for this task is strict source exclusivity.

### Decision 4: Materialize live responses into the existing `HotspotsReport` envelope

The live-exported artifact keeps the current schema and command identity:
- `schemaVersion: "1.0.0"`
- `command: "hotspots"`
- `repositoryPath`: resolved export target path
- `storePath`: `live:<server_url>/v1/repositories/<repository_id>/hotspots?window=cumulative`
- `generatedAt`: copied from the live response
- `entries`: copied from the live response
- `runMetadata`: derived from live entries with `analyzedSubjectCount = entries.len()`, `analyzedEventCount = sum(evidence.rowIds.len())`, `storeSchemaVersion = 0`, `firstEventId = 0`, `lastEventId = 0`

**Rationale**: downstream consumers already deserialize `entries` and `generatedAt`, while refinement required explicit live provenance without pretending the artifact came from `.scryrs/scryrs.db`.

### Decision 5: Validate fully before atomic replacement

Successful live export serializes the final `HotspotsReport`, writes it to a temp file in `<PATH>/.scryrs/`, and atomically renames it into place. Any transport, HTTP-status, JSON, schema-version, or repository-identity failure exits nonzero, emits a clear `scryrs hotspots:` error, and leaves any pre-existing artifact untouched.

**Rationale**: acceptance requires loud failure and no partial or misleading artifact writes.

### Decision 6: Keep downstream workflow unchanged and prove it with smoke tests

The implementation must prove `export -> graph -> route -> propose` works with the live-exported artifact and no downstream live flags.

**Rationale**: the entire point of the export is to feed the existing artifact-driven workflow unchanged.

### Decision 7: Add a test seam for live HTTP fetching

Introduce a small live-fetch abstraction, parallel to existing remote submission patterns, so export logic can be tested deterministically without depending on a real network server for every case.

**Rationale**: refinement explicitly called out the need for testable HTTP mocking around the live export path.

## Risks

| Risk | Mitigation |
| --- | --- |
| Graph evidence links created from live-exported artifacts still use the existing local trace-row source semantics even though the row IDs came from the server store. | Keep this as an explicit non-goal/risk for this task; preserve downstream compatibility now and defer provenance relabeling to a later change. |
| Adding live flags changes the current strict parser behavior around `scryrs hotspots <PATH>`. | Update dispatch/help tests deliberately so valid flags are accepted while unexpected extra positionals still fail. |
| Live `runMetadata` cannot truthfully mirror local SQLite metadata fields. | Use explicit live-derived/sentinel values and document them in the conversion contract instead of pretending a local store exists. |

## Conflict Resolution

1. **Live input sourcing**: refinement disagreed on explicit live-only flags versus config-precedence reuse. Adopt config-precedence reuse because the accepted reviewer decision resolved the open question directly and the architect rationale already pointed to existing remote-resolution patterns.
2. **`storePath` format**: adopt the architect's `live:<query_url>` requirement with the reviewer's full query URL specificity so the artifact source is unambiguous.
3. **`generatedAt` source**: use `LiveHotspotsResponse.generatedAt` rather than export-time wall clock so the exported artifact reflects the server-owned ranking snapshot being materialized.

## Traceability

| Source | Use in this design |
| --- | --- |
| Task `06ed79cd-a4c8-44ea-99de-08cfbba7742b` | Defines feature scope, scenarios, technical notes, and acceptance criteria. |
| Dossier `2026-07-01T10:24:06.730Z` | Supplies goals, non-goals, open questions, affected areas, and the initial proposal sketch. |
| Accepted decision `1-swarm-architect-recommendation` | Fixes separate live code path, `live:<query_url>` provenance, atomic write, and test seam direction. |
| Accepted decision `1-swarm-lead-dev-recommendation` | Fixes extension of the existing `hotspots` subcommand, conversion shape, and downstream smoke scope. |
| Accepted decision `1-swarm-reviewer-recommendation` | Fixes config precedence reuse, concrete `runMetadata` mapping, repository/schema validation, and atomic-write requirement. |
| `crates/scryrs-cli/src/hotspots.rs`, `dispatch.rs`, `dispatch_tests.rs` | Identify the current local-only command path and parser/error surfaces that the contract must extend. |
| `crates/scryrs-types/src/lib.rs` | Provides the `HotspotsReport`, `LiveHotspotsResponse`, and shared `HotspotEntry` shapes used by the conversion. |
| `crates/scryrs-server/src/server.rs` and `.devagent/docs/docs/cli-v0-contract.md` | Confirm the cumulative live hotspot endpoint path and response contract. |
| `crates/scryrs-cli/src/graph.rs`, `propose.rs`, and `route.rs` | Confirm downstream artifact compatibility requirements and smoke-test scope. |
