# Design: Hotspot Output Contract and Deterministic Ranking

## Context

scryrs observes agent coding sessions through a trace hook that emits data to `.scryrs/scryrs.db` (SQLite). The `TraceQuery` read model provides deterministic, indexed access to all persisted events. The current CLI `scryrs hotspots <PATH>` command is a placeholder. This design freezes the contract for the first real hotspot analysis pipeline.
The design space is well-constrained: the SQLite schema already persists every needed column (`id`, `subject_kind`, `subject`, `event_type`, `session_id`, `timestamp`, `outcome`, `failure_reason`), the `TraceQuery` already guarantees deterministic ordering (`timestamp ASC, id ASC`), and the normalized `subject_kind` mapping (file, search, symbol, command, document) already exists in `scryrs-types`. The gap is purely in the output schema, scoring logic, and CLI wiring.

## Goals

1. Replace placeholder hotspot output with one versioned machine-readable report shape for stdout and `.scryrs/hotspots.json`.
2. Define ranked hotspot entries preserving SQLite-backed evidence: subject kind, subject, score, event counts by type and outcome, session breadth, time span, and row references.
3. Specify a deterministic scoring formula and tie-break order using only persisted SQLite columns.
4. Make empty-but-valid analysis output explicit and separate it from MissingStore/UnsupportedStore failures.
5. Keep the contract aligned across CLI docs, `--help-json`, snapshots, and downstream consumers.

## Non-Goals

- Graph building, proposal generation, or adapter publishing.
- LLM-based scoring, summarization, or fuzzy clustering.
- Redesigning the trace event schema or hook capture flow.
- Speculative subject canonicalization beyond the existing `subject_kind + subject` SQLite grouping.
- Capping evidence row ID lists (documented as v2 consideration).

## Decisions

### D1: Independent hotspot schema version

**Decision:** The hotspot report SHALL use its own `HOTSPOT_SCHEMA_VERSION` constant starting at `"1.0.0"`, independent of trace event `SCHEMA_VERSION` (`"0.1.0"`).

**Rationale:** The trace event wire schema governs event serialization; the hotspot report governs a derived analysis output and will evolve independently. Coupling them would force unnecessary version bumps on one when only the other changes. Starting at `"1.0.0"` signals this is a first stable output contract, not a patch over the `"0.1.0"` placeholder.

**Artifacts:** `crates/scryrs-types/src/lib.rs` (new `HOTSPOT_SCHEMA_VERSION` constant), `HotspotsReport.schemaVersion` field.

### D2: HotspotReport envelope shape

**Decision:** The top-level envelope SHALL be:

```json
{
  "schemaVersion": "1.0.0",
  "command": "hotspots",
  "repositoryPath": "<absolute path>",
  "storePath": "<absolute path to .scryrs/scryrs.db>",
  "runMetadata": {
    "storeSchemaVersion": 1,
    "analyzedEventCount": 150,
    "analyzedSubjectCount": 42,
    "firstEventId": 1,
    "lastEventId": 150
  },
  "generatedAt": "2026-06-21T12:00:00Z",
  "entries": [...]
}
```

**Rationale:** `runMetadata` provides deterministic, reproducible fields for snapshot testing (derived entirely from SQLite state). `generatedAt` provides a wall-clock timestamp for auditability. Both are always present — no union, no discriminator field. `repositoryPath` and `storePath` satisfy Scenario 1 from the task prompt and enable multi-repository report correlation.

### D3: HotspotEntry schema

**Decision:** Each entry SHALL be:

```json
{
  "rank": 1,
  "subjectKind": "file",
  "subject": "src/main.rs",
  "score": 23,
  "counts": {
    "eventType": {"FileOpened": 5, "EditMade": 4},
    "outcome": {"success": 8, "failure": 1}
  },
  "sessionCount": 3,
  "firstSeen": "2026-06-21T09:00:00Z",
  "lastSeen": "2026-06-21T17:30:00Z",
  "evidence": {
    "rowIds": [3, 7, 12, 45, 67, 89, 102, 134, 143]
  }
}
```

**Rationale:** `rank` (1-based integer) makes position explicit rather than inferred from array index. `counts.eventType` provides per-event-type breakdown so consumers can distinguish edits from reads within the same `subjectKind`. `counts.outcome` provides success/failure breakdown. `sessionCount` measures breadth across sessions. `firstSeen`/`lastSeen` provide time span. `evidence.rowIds` is the full ordered list of SQLite `trace_events.id` values for all contributing events (not a capped sample in v1) — consumers can join back to row-level data.

### D4: Scoring weight table

**Decision:** A single shared integer weight table keyed by `TraceEventType`, with an additive failure bonus:

| Event Type | Base Weight |
|---|---|
| `FileOpened` | 1 |
| `SearchRun` | 2 |
| `SymbolInspected` | 2 |
| `CommandExecuted` | 1 |
| `DocRetrieved` | 2 |
| `EditMade` | 3 |
| `FailedLookup` | 4 |

Plus **+2 bonus** for every event row where `outcome = 'Failure'`, applied regardless of event type, on top of the base weight.

**Score formula:** `score = Σ(base_weight_per_event) + Σ(failure_bonus_per_event)` for all subject-bearing events grouped by `(subject_kind, subject)`.

**Rationale:** A single shared table avoids per-subject-kind formula drift. Event-type weights differentiate information value: edits and failures are more informative than passive reads. The failure bonus applies to all event types with `Outcome::Failure` (e.g., a failed `EditMade` = 3+2 = 5; a `FailedLookup` = 4+2 = 6) — this makes edit/failure contributions explicit scoring dimensions as required by the acceptance criteria. Lifecycle events (`SessionStart`, `SessionEnd`) have no subject and are excluded from scoring.

### D5: Deterministic tie-break

**Decision:** Ranking SHALL use a six-key tie-break chain, all dimensions derived from persisted SQLite columns:

1. `score DESC`
2. `sessionCount DESC`
3. `lastSeen DESC`
4. `subjectKind ASC`
5. `subject ASC`
6. `firstEventId ASC` (the SQLite `id` of the chronologically first event for that subject)

**Rationale:** This order prioritizes total weighted activity, then session breadth, then recency, then lexical subject-kind grouping, then lexical subject order, then insertion order of the first contributing event. Every dimension is deterministic with no randomness or wall-clock dependency. The six-key chain ensures identical results on repeated runs over the same data.

### D6: Grouping key

**Decision:** Subjects SHALL be grouped by the composite key `(subject_kind, subject)` sourced from the SQLite `subject_kind` and `subject` columns.

**Rationale:** This is the existing grouping used by `idx_trace_events_subject`. The seven subject-bearing event families normalize into five subject kinds: `file` (FileOpened, EditMade), `search` (SearchRun), `symbol` (SymbolInspected, FailedLookup), `command` (CommandExecuted), and `document` (DocRetrieved). Two distinct subjects with different kinds are scored and ranked independently.

### D7: Exit code contract

**Decision:** The CLI exit codes SHALL be:

| Exit Code | Condition |
|---|---|
| 0 | Valid store with rankable subjects (populated `entries`) or valid store with zero rankable subjects (`entries: []`) |
| 1 | `StorageError` (corrupt file, I/O failure) |
| 2 | `MissingStore`, `UnsupportedStore`, or usage errors (missing PATH argument) |

**Rationale:** Exit 0 for valid-but-empty stores is required by the acceptance criteria and matches the existing `TraceQuery::EmptyStore` distinction from `MissingStore`. Exit 2 for `MissingStore`/`UnsupportedStore` follows the existing CLI convention where state/precondition errors exit 2. Exit 1 for `StorageError` follows Unix conventions for I/O-level failures.

### D8: CLI output contract

**Decision:** `scryrs hotspots <PATH>` SHALL:
1. Resolve `<PATH>` to an absolute path for `repositoryPath`
2. Open `TraceQuery::open(path)` with the resolved repo root
3. Handle `QueryError::MissingStore` → exit 2, error to stderr
4. Handle `QueryError::UnsupportedStore` → exit 2, error to stderr
5. Handle `QueryError::StorageError` → exit 1, error to stderr
6. Materialize all events via `iter_events_ordered()`
7. Score subjects with `score_hotspots()`
8. Emit `HotspotsReport` as single-line JSON to stdout
9. Optionally write the same JSON to `.scryrs/hotspots.json` at the repo root
10. Exit 0

**Rationale:** Stdout and artifact file share the same JSON schema per the dossier assumption. The PATH argument is resolved at the CLI level; the absolute path is included in the report for downstream correlation.

## Risks

### R1: Evidence payload size

**Risk:** For subjects with hundreds of events, the `evidence.rowIds` array will be unbounded, increasing JSON output size.
**Severity:** Low for v1 (single-repo analysis with typical <10K events).
**Mitigation:** Document that v1 emits all contributing row IDs. If payload size becomes a problem in practice, a future schema version can introduce capped sampling (e.g., first 100 + `totalCount`).

### R2: Weight table is initial defaults

**Risk:** The weight values have no empirical validation and may produce poor ranking quality in real-world usage.
**Severity:** Medium.
**Mitigation:** Document weights as initial defaults with a clear evolution plan. The contract supports weight table versioning through the independent `HOTSPOT_SCHEMA_VERSION`.

### R3: Memory pressure from materialization

**Risk:** The scorer will call `iter_events_ordered()` which loads all events into memory. For repos with hundreds of thousands of events, this could be expensive.
**Severity:** Low for v1. **Mitigation:** The contract is defined first; a SQLite `GROUP BY` optimization is a safe follow-up implementation refinement.

### R4: Downstream curator coupling

**Risk:** `scryrs-curator::propose_from_hotspot` currently takes `&Hotspot` (subject + score only). The new `HotspotEntry` type changes the public API.
**Severity:** Low — this is a workspace-internal breaking change within one implementation task.
**Mitigation:** Update the curator signature in the same implementation change. The new `HotspotEntry` retains `subject` and `score` fields, so the migration is straightforward.

### R5: Snapshot test determinism

**Risk:** The `generatedAt` field introduces wall-clock dependency that breaks exact snapshot assertions.
**Severity:** Low.
**Mitigation:** Snapshot tests should either use a feature-flagged reproducible mode that replaces `generatedAt` with a fixed sentinel, or strip the field before comparison. The `runMetadata` fields are always deterministic.

## Traceability

- **Task prompt:** Scenarios 1-3 define required output, entry, and ranking rule behaviors.
- **Exploration dossier:** Defines problem framing, goals, non-goals, assumptions, and open questions.
- **Round 1 architect:** Accepted — eight architectural decisions adopted.
- **Round 1 lead-dev:** Accepted — three refinements (decoupled version, always-present `runMetadata`, flat `rowIds`).
- **Round 1 reviewer:** Needs-work blockers resolved — `runMetadata | generatedAt` union → both always present; evidence format → full `rowIds` array; failure bonus scope → all `Outcome::Failure` rows; exit code contract → explicit; schema version decoupling → "1.0.0".
- **Codebase:** `crates/scryrs-types/src/lib.rs` (current `Hotspot` struct), `crates/scryrs-core/src/store.rs` (SQLite schema), `crates/scryrs-core/src/query.rs` (deterministic read model), `crates/scryrs-core/src/lib.rs` (scaffold scorer), `crates/scryrs-cli/src/lib.rs` (placeholder output and snapshot tests), `crates/scryrs-curator/src/lib.rs` (downstream consumer).
- **Docs:** Project docs — roadmap (Phase 2 requirements), vision (hotspot detection as standalone value), CLI v0 contract (current placeholder), trace hook contract (event families and outcome semantics).
- **OpenSpec:** `scryrs-trace-query/spec.md` (placeholder requirement to be superseded).