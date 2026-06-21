# Tasks: Hotspot Foundation 02

## 1. Define Types and Schema

- [ ] 1.1 Add `HOTSPOT_SCHEMA_VERSION` constant (`"1.0.0"`) to `crates/scryrs-types/src/lib.rs`, independent of `SCHEMA_VERSION`.
- [ ] 1.2 Define `HotspotEntry` struct with fields: `rank: u32`, `subjectKind: String`, `subject: String`, `score: u32`, `counts: HotspotCounts`, `sessionCount: u32`, `firstSeen: String`, `lastSeen: String`, `evidence: HotspotEvidence`.
- [ ] 1.3 Define `HotspotCounts` struct with `eventType: HashMap<String, u32>` and `outcome: HashMap<String, u32>`.
- [ ] 1.4 Define `HotspotEvidence` struct with `rowIds: Vec<u64>`.
- [ ] 1.5 Define `HotspotsReport` struct with fields: `schemaVersion`, `command`, `repositoryPath`, `storePath`, `runMetadata: RunMetadata`, `generatedAt`, `entries: Vec<HotspotEntry>`.
- [ ] 1.6 Define `RunMetadata` struct with fields: `storeSchemaVersion: i64`, `analyzedEventCount: u64`, `analyzedSubjectCount: u64`, `firstEventId: u64`, `lastEventId: u64`.
- [ ] 1.7 Derive `Serialize` for all new types; remove the old `Hotspot` struct.
- [ ] 1.8 Add unit tests for new type construction and serialization round-trips.

## 2. Implement Deterministic Scoring

- [ ] 2.1 Define the weight table as constants in `crates/scryrs-core/src/lib.rs` (or new `scoring` module): `FileOpened=1, SearchRun=2, SymbolInspected=2, CommandExecuted=1, DocRetrieved=2, EditMade=3, FailedLookup=4, failure_bonus=2`.
- [ ] 2.2 Implement `score_hotspots(events: &[TraceEvent]) -> Vec<HotspotEntry>` that:
  - [ ] 2.2.1 Filters out lifecycle events (no `subject()`).
  - [ ] 2.2.2 Groups events by `(subject_kind, subject)` from `TraceEvent::subject_kind()` and `TraceEvent::subject()`.
  - [ ] 2.2.3 Computes per-group: score (Σ base_weight + Σ failure_bonus), per-event-type counts, per-outcome counts, session count (unique `session_id`), first/last timestamps, and ordered `rowIds`.
  - [ ] 2.2.4 Sorts entries by the six-key tie-break: `score DESC`, `sessionCount DESC`, `lastSeen DESC`, `subjectKind ASC`, `subject ASC`, `firstEventId ASC`.
  - [ ] 2.2.5 Assigns `rank` (1-based) after sorting.
- [ ] 2.3 Remove the existing scaffold `score_events()` function.
- [ ] 2.4 Add unit tests for `score_hotspots`:
  - [ ] 2.4.1 Empty input returns empty `Vec`.
  - [ ] 2.4.2 Only lifecycle events returns empty `Vec`.
  - [ ] 2.4.3 Single subject with multiple event types scores correctly.
  - [ ] 2.4.4 Failure bonus applied correctly (e.g., FailedLookup = 6, EditMade+Failure = 5).
  - [ ] 2.4.5 Two subjects with identical score sort deterministically by tie-break chain.
  - [ ] 2.4.6 Session count is correct (two events from same session count as 1).
  - [ ] 2.4.7 Event-type and outcome counts are correct.
  - [ ] 2.4.8 `firstSeen`/`lastSeen` track min/max timestamps correctly.
  - [ ] 2.4.9 `evidence.rowIds` is ordered by `timestamp ASC, id ASC`.

## 3. Wire CLI Hotspot Command

- [ ] 3.1 Update `write_hotspots_json()` in `crates/scryrs-cli/src/lib.rs` to:
  - [ ] 3.1.1 Accept a repository path argument.
  - [ ] 3.1.2 Resolve the path to an absolute path for `repositoryPath`.
  - [ ] 3.1.3 Open `TraceQuery::open(repo_root)`.
  - [ ] 3.1.4 Map `QueryError` variants to exit codes: `MissingStore` → exit 2, `UnsupportedStore` → exit 2, `StorageError` → exit 1.
  - [ ] 3.1.5 Materialize events via `iter_events_ordered()`.
  - [ ] 3.1.6 Score subjects with `score_hotspots()`.
  - [ ] 3.1.7 Build `RunMetadata` from store state.
  - [ ] 3.1.8 Serialize `HotspotsReport` as single-line JSON to stdout.
  - [ ] 3.1.9 Write the same JSON to `<repo_root>/.scryrs/hotspots.json`.
- [ ] 3.2 Update the `cli_surface_doc()` function in `crates/scryrs-cli/src/lib.rs` to describe the new hotspot output fields.
- [ ] 3.3 Update the `write_help()` function to describe the new hotspot output contract.
- [ ] 3.4 Update inline snapshot tests and insta snapshots for the new hotspot output.
- [ ] 3.5 Add integration tests:
  - [ ] 3.5.1 Populated store produces correct `HotspotsReport` JSON.
  - [ ] 3.5.2 Empty store produces `entries: []` with exit 0.
  - [ ] 3.5.3 Missing store exits 2 with error message on stderr.
  - [ ] 3.5.4 Unsupported store exits 2 with error message on stderr.
  - [ ] 3.5.5 Corrupt/non-SQLite file exits 1 with error message on stderr.
  - [ ] 3.5.6 Deterministic ordering: same store produces identical output on repeated runs.
  - [ ] 3.5.7 Tie-break correctness: subjects with identical scores sort deterministically.
  - [ ] 3.5.8 `.scryrs/hotspots.json` artifact file is written when store is valid.

## 4. Update Documentation and Contracts

- [ ] 4.1 Create `openspec/specs/hotspot-report/spec.md` with ADDED requirements for `HotspotsReport` schema, scoring formula, tie-break rules, exit codes, and CLI behavior.
- [ ] 4.2 Update `openspec/specs/scryrs-trace-query/spec.md`: remove the "CLI hotspot command remains placeholder" requirement via a REMOVED delta.
- [ ] 4.3 Update `README.md` to show real hotspot output instead of placeholder.
- [ ] 4.4 Update the help-json snapshot at `crates/scryrs-cli/src/snapshots/`.

## 5. Update Downstream Consumer

- [ ] 5.1 Update `crates/scryrs-curator/src/lib.rs` `propose_from_hotspot()` signature to accept `&HotspotEntry` instead of `&Hotspot`.
- [ ] 5.2 Update curator tests to construct `HotspotEntry` instances.