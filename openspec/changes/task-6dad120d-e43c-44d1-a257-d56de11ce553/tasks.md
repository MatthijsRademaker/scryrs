## 1. Create E2E integration test binary

- [ ] 1.1 Create `crates/scryrs-cli/tests/hotspot_e2e.rs` with:
  - A `static CWD_GUARD: Mutex<()>` and `fn with_cwd(dir: &Path, f: impl FnOnce())` helper for CWD serialization (duplicate the proven pattern from `init_tests`)
  - A `fn make_event_json(...)` helper that constructs valid JSONL lines for each subject-bearing event family: `FileOpened`, `SearchRun`, `SymbolInspected`, `CommandExecuted`, `DocRetrieved`, `EditMade`, `FailedLookup`
  - A `fn normalize_hotspot_json(json: &str) -> String` helper that parses JSON, replaces `generatedAt` with `"<GENERATED_AT>"`, `repositoryPath` with `"<REPO>"`, `storePath` with `"<STORE>"`, and re-serializes
- [ ] 1.2 Add `#[test] fn e2e_record_to_hotspots_pipeline()` that:
  - Creates a `tempfile::tempdir()` as the temp repo
  - Calls `with_cwd(dir.path(), || { ... })` to run under the temp repo's CWD
  - Pipes multi-event JSONL (at least one event from each of the 7 subject-bearing families plus one `Outcome::Failure` event) through `scryrs record --stdin` via calling `scryrs_cli::run_with_io`
  - Asserts record exit code 0 and expected accepted/rejected counts in stdout
  - Opens `.scryrs/scryrs.db` via `rusqlite::Connection` and asserts `trace_events` row count matches expected
  - Runs `scryrs hotspots <dir>` and asserts exit code 0
  - Asserts `.scryrs/hotspots.json` exists and matches stdout byte-for-byte (modulo stdout trailing newline)
  - Asserts expected ranking: subjects with higher scores rank above lower scores; verifies at least the top-ranked entry's `subjectKind`, `subject`, and `score`
  - Calls `insta::assert_snapshot!("hotspot_stdout", normalize_hotspot_json(&stdout))` for stdout drift detection
  - Calls `insta::assert_json_snapshot!("hotspot_artifact", &serde_json::from_str::<serde_json::Value>(&normalized_artifact).unwrap())` for artifact drift detection
- [ ] 1.3 Add `#[test] fn e2e_empty_store_produces_success()` that:
  - Creates a temp repo, creates `.scryrs/scryrs.db` with empty schema, then runs `scryrs hotspots <repo>`
  - Asserts exit code 0, `entries: []`, and snapshot-matches normalized stdout
- [ ] 1.4 Add `#[test] fn e2e_missing_store_exits_2()` that:
  - Creates a temp repo without `.scryrs/scryrs.db`, runs `scryrs hotspots <repo>`
  - Asserts exit code 2, stderr contains "datastore not found", no stdout JSON

## 2. Extend existing hotspot_integration_tests with full subject-family fixture

- [ ] 2.1 In `crates/scryrs-cli/src/lib.rs`, inside `mod hotspot_integration_tests`, add helper functions for each subject-bearing event family:
  - `make_search_run(session, query, timestamp) -> TraceEvent`
  - `make_symbol_inspected(session, name, timestamp) -> TraceEvent`
  - `make_command_executed(session, command, timestamp, outcome) -> TraceEvent`
  - `make_doc_retrieved(session, doc_ref, timestamp) -> TraceEvent`
  - `make_edit_made(session, target, timestamp, outcome) -> TraceEvent`
  - `make_failed_lookup(session, subject, reason, timestamp) -> TraceEvent`
- [ ] 2.2 Add `#[test] fn full_subject_family_fixture_produces_correct_ranking()` that:
  - Creates a `tempdir`, populates it with at least one event from each of the 7 families (FileOpened, SearchRun, SymbolInspected, CommandExecuted, DocRetrieved, EditMade, FailedLookup) plus one `Outcome::Failure` event on a non-FailedLookup type
  - Runs `scryrs hotspots <repo>` and asserts exit code 0
  - Asserts `entries` length >= expected distinct subjects
  - Asserts expected score for each entry based on documented weight table: `FileOpened=1, SearchRun=2, SymbolInspected=2, CommandExecuted=1, DocRetrieved=2, EditMade=3, FailedLookup=4` with `+2` failure bonus
  - Asserts `counts.eventType` and `counts.outcome` values are correct for the top entry
  - Asserts `.scryrs/hotspots.json` is written and matches stdout

## 3. Verify snapshot and existing test integrity

- [ ] 3.1 Run `cargo test -p scryrs-cli` to verify:
  - New E2E tests pass and generate initial snapshots
  - Existing hotspot_integration_tests still pass (no regression)
  - All record tests still pass
  - All init tests still pass
- [ ] 3.2 Run `cargo insta review` to accept initial snapshots
- [ ] 3.3 Run `cargo test -p scryrs-cli` a second time to confirm snapshots are stable (no drift on re-run)
- [ ] 3.4 Verify that intentionally changing a hotspot output field (e.g., modifying a weight constant) causes the snapshot test to fail — confirming drift detection works

## 4. Cleanup and documentation

- [ ] 4.1 Ensure no dead code, commented-out tests, or TODO comments remain
- [ ] 4.2 Verify `cargo clippy -p scryrs-cli --all-features --tests` passes with no new warnings
- [ ] 4.3 Verify `cargo fmt --check` passes on all changed files