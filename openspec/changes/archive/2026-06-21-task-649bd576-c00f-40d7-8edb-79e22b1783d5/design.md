## Context

`Hotspot Foundation 03` is a closure task over functionality that is already largely present in the repository. Refinement evidence shows that `TraceQuery::open()` already reads `<PATH>/.scryrs/scryrs.db` in read-only mode, `score_hotspots()` already groups and scores subject-bearing events deterministically, `HotspotsReport` types already exist, and the CLI already serializes real hotspot output.

The task stays open because three behavior gaps still separate the current implementation from the canonical `hotspot-report` contract:

1. `runMetadata.firstEventId` / `lastEventId` are derived from timestamp-order first/last subject-bearing events instead of min/max SQLite ids.
2. The final ranking tie-break uses the minimum evidence row id instead of the row id of the chronologically first contributing event.
3. Artifact write failures are tolerated after stdout emission even though successful analysis is defined to write `.scryrs/hotspots.json`.

The same refinement pass also found stale placeholder wording in `scryrs --help`, README examples, and snapshots, plus missing edge-case CLI coverage for lifecycle-only stores, non-monotonic timestamp/id ordering, and artifact failure behavior.

## Goals / Non-Goals

### Goals

- Make `scryrs hotspots <PATH>` fully match the canonical SQLite-backed hotspot report contract.
- Preserve standalone behavior: open `.scryrs/scryrs.db`, analyze persisted trace rows, emit deterministic JSON, and avoid inventing hotspots.
- Remove user-visible placeholder wording from hotspot help/output documentation surfaces.
- Add repeatable CLI coverage for the contract edges that can silently regress.

### Non-Goals

- No new graph, proposal, adapter, runtime, dashboard, or LLM-backed behavior.
- No scoring-model redesign, fuzzy clustering, subject canonicalization, or datastore schema changes.
- No archival doc cleanup or historical artifact editing outside the active CLI/user-facing surfaces named by refinement.
- No new public contract beyond what the task and canonical hotspot-report spec already require.

## Decisions

### D1. Treat this as a hardening/closure change, not a new subsystem build
The proposal focuses on reconciling already-merged hotspot behavior with the existing `hotspot-report` contract. Work stays localized to contract fixes, stale placeholder removal, and targeted tests.

### D2. `runMetadata.firstEventId` / `lastEventId` use min/max subject-bearing SQLite ids
The CLI must compute these metadata fields from the minimum and maximum subject-bearing row ids, not from the first and last rows in timestamp order.

### D3. The final ranking tie-break uses the chronologically first contributing event id
When the first five sort keys tie, comparison uses the SQLite row id of the first contributing event in the subject's `timestamp ASC, id ASC` evidence order. It must not use the minimum row id across that subject's evidence set.

### D4. Artifact persistence is part of success
A run is only successful if `.scryrs/hotspots.json` is written. If artifact creation/overwrite fails for either populated or empty-report success paths, the command exits 1 and reports the write failure on stderr instead of returning success.

### D5. Placeholder-only hotspot wording must disappear from the live surface
The implementation must remove hotspot placeholder wording from CLI help text, help snapshots/help-json expectations, README hotspot examples, and tests that still encode the placeholder language.

### D6. Verification must cover the edge cases that drove the refinement blockers
The change is not complete without repeatable tests for lifecycle-only stores, non-monotonic timestamp/id ordering, evidence row ordering, byte-for-byte artifact equality, artifact write failure behavior, and absence of stale placeholder text.

## Conflict Resolution

- **Artifact write failure behavior**: refinement flagged this as ambiguous in the current canonical text, but the accepted implementation direction consistently treated the current "log and still succeed" behavior as a contract gap. This proposal resolves the ambiguity in favor of exit code 1 on artifact write failure because successful hotspot analysis is defined to write `.scryrs/hotspots.json`.
- **Documentation scope**: refinement evidence noted stale internal project docs, but the accepted decisions and blockers consistently named `--help`, help snapshots/help-json, and `README.md` as the required cleanup surfaces. This change keeps scope there and does not expand into unrelated historical or internal doc cleanup.

## Risks

| Risk | Mitigation |
| --- | --- |
| Artifact write failures now become non-zero exits, which is stricter than current behavior. | Keep the change explicit in proposal/spec/tasks and cover it with CLI tests for both populated and empty-report paths. |
| Fixing `firstEventId` semantics can flip deterministic ordering for non-monotonic timestamp/id data. | Add dedicated tests that build those exact fixtures and assert the corrected ordering and metadata. |
| Help/snapshot/README cleanup can drift from live command behavior. | Update snapshot assertions together with help text changes and keep README examples aligned with the emitted report contract. |

## Traceability

- Task: `649bd576-c00f-40d7-8edb-79e22b1783d5`
- Dossier: `2026-06-21T12:00:43.203Z`
- Accepted decisions: `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, `1-swarm-reviewer-recommendation`
- Round outputs: `round:1:agent:swarm-architect`, `round:1:agent:swarm-lead-dev`, `round:1:agent:swarm-reviewer`
- Canonical spec: `openspec/specs/hotspot-report/spec.md`
- Repository evidence: `crates/scryrs-cli/src/lib.rs`, `crates/scryrs-core/src/query.rs`, `crates/scryrs-core/src/scoring.rs`, `crates/scryrs-types/src/lib.rs`, `README.md`