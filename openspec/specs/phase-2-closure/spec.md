# phase-2-closure Specification

## Purpose

Defines the Phase 2 closure reconciliation contract. This spec documents that Phase 2 hotspot materialization is fully implemented in code and tests, maps each roadmap Phase 2 deliverable to concrete repository evidence, supersedes conflicting placeholder-era requirements in three stale OpenSpec specs (phase-1-closure, cli-foundation-closure, cli-golden-tests), and requires that four stale published docs (roadmap.mdx, cli-v0-contract.md, architecture.mdx, trace-hook-contract.md) be updated to describe the real shipped hotspot product boundary.

## Requirements

### Requirement: Phase 2 hotspot materialization is verified complete in code and tests

The live CLI (`crates/scryrs-cli/src/lib.rs`), scoring engine (`crates/scryrs-core/src/scoring.rs`), and end-to-end tests (`crates/scryrs-cli/tests/hotspot_e2e.rs`) SHALL serve as the authoritative evidence that Phase 2 is functionally complete. Each roadmap Phase 2 deliverable SHALL be traceable to concrete code paths or test artifacts.

#### Scenario: Roadmap deliverable 1 — Real scryrs hotspots output

- **GIVEN** the roadmap Phase 2 section lists 'Real scryrs hotspots' as a required deliverable
- **WHEN** a reviewer inspects `crates/scryrs-cli/src/lib.rs` function `write_hotspots_json`
- **THEN** the function opens `.scryrs/scryrs.db` through `TraceQuery::open(&repo_root)`
- **AND** it builds a `HotspotsReport` with `schemaVersion: "1.0.0"`, `runMetadata`, and `entries`
- **AND** it writes the report to stdout and `.scryrs/hotspots.json`
- **AND** `crates/scryrs-cli/tests/hotspot_e2e.rs` exercises the full public pipeline

#### Scenario: Roadmap deliverable 2 — Deterministic aggregation rules

- **GIVEN** the roadmap Phase 2 section lists 'deterministic aggregation rules' as a required deliverable
- **WHEN** a reviewer inspects `crates/scryrs-core/src/scoring.rs`
- **THEN** `WEIGHT_FILE_OPENED=1`, `WEIGHT_SEARCH_RUN=2`, `WEIGHT_SYMBOL_INSPECTED=2`, `WEIGHT_COMMAND_EXECUTED=1`, `WEIGHT_DOC_RETRIEVED=2`, `WEIGHT_EDIT_MADE=3`, `WEIGHT_FAILED_LOOKUP=4` are defined
- **AND** `FAILURE_BONUS=2` is applied per-failure
- **AND** the six-key tie-break chain uses `score DESC, sessionCount DESC, lastSeen DESC, subjectKind ASC, subject ASC, firstEventId ASC`
- **AND** `score_hotspots` groups by `(subject_kind, subject)`

#### Scenario: Roadmap deliverable 3 — Stable JSON contract

- **GIVEN** the roadmap Phase 2 section lists 'stable JSON contract' as a required deliverable
- **WHEN** a reviewer inspects `crates/scryrs-types/src/lib.rs`
- **THEN** `HotspotsReport` struct includes `schemaVersion`, `command`, `repositoryPath`, `storePath`, `runMetadata`, `generatedAt`, `entries`
- **AND** `HOTSPOT_SCHEMA_VERSION = "1.0.0"` is independent of `SCHEMA_VERSION = "0.1.0"`
- **AND** `HotspotEntry` includes `rank`, `subjectKind`, `subject`, `score`, `counts`, `sessionCount`, `firstSeen`, `lastSeen`, `evidence`

#### Scenario: Roadmap deliverable 4 — Optional file output .scryrs/hotspots.json

- **GIVEN** the roadmap Phase 2 section lists 'optional file outputs like .scryrs/hotspots.json' as a required deliverable
- **WHEN** a reviewer inspects `crates/scryrs-cli/src/lib.rs` write_hotspots_json
- **THEN** the function writes the same `HotspotsReport` JSON to `.scryrs/hotspots.json` after stdout
- **AND** artifact write failure exits with code 1 and writes an error to stderr
- **AND** `crates/scryrs-cli/tests/hotspot_e2e.rs` asserts artifact file existence and content matches stdout

#### Scenario: E2E verification proves the complete pipeline

- **GIVEN** the test file `crates/scryrs-cli/tests/hotspot_e2e.rs`
- **WHEN** `e2e_record_to_hotspots_pipeline` runs
- **THEN** it pipes multi-event-family JSONL through `scryrs record --stdin`
- **AND** verifies SQLite rows via `rusqlite::Connection`
- **AND** runs `scryrs hotspots <PATH>` and asserts exit code 0
- **AND** asserts `.scryrs/hotspots.json` exists and matches stdout
- **AND** asserts the top-ranked entry reflects the highest-scoring subject
- **AND** `e2e_empty_store_produces_success` verifies exit 0 with empty entries
- **AND** `e2e_missing_store_exits_2` verifies exit 2 with stderr error

### Requirement: Stale OpenSpec specs are reconciled

The three stale OpenSpec specs that still assert placeholder hotspot behavior SHALL have reconciliation headers explicitly superseding their conflicting requirements.

#### Scenario: phase-1-closure/spec.md is superseded for Phase 2 hotspot requirements

- **GIVEN** `openspec/specs/phase-1-closure/spec.md` Requirement 'Phase 2 behavior remains out of scope'
- **WHEN** the Phase 2 closure change is applied
- **THEN** a reconciliation header is added to the spec superseding this requirement
- **AND** the header references `openspec/specs/hotspot-report/spec.md` and `openspec/specs/hotspot-verification/spec.md` as the canonical Phase 2 contract
- **AND** the header cites the current closure change path for traceability

#### Scenario: cli-foundation-closure/spec.md is superseded for hotspot placeholder requirements

- **GIVEN** `openspec/specs/cli-foundation-closure/spec.md` Requirement 'Single placeholder command operates correctly'
- **WHEN** the Phase 2 closure change is applied
- **THEN** a reconciliation header is added to the spec superseding the placeholder JSON and no-backend-wiring scenarios
- **AND** the header references `hotspot-report/spec.md` and the live implementation in `crates/scryrs-cli/src/lib.rs`

#### Scenario: cli-golden-tests/spec.md is superseded for hotspot placeholder snapshot requirements

- **GIVEN** `openspec/specs/cli-golden-tests/spec.md` Requirement 'hotspots placeholder output is verified by inline snapshot'
- **WHEN** the Phase 2 closure change is applied
- **THEN** a reconciliation header is added to the spec superseding the placeholder snapshot requirement
- **AND** the header references `hotspot_e2e.rs` and `hotspot_integration_tests` as the current canonical hotspot test coverage

### Requirement: Stale published docs describe the real hotspot product boundary

The four stale `.devagent/docs/docs/` pages that still describe placeholder or deferred Phase 2 hotspot behavior SHALL be updated to describe the real shipped hotspot contract.

#### Scenario: roadmap.mdx reflects delivered Phase 2

- **GIVEN** `.devagent/docs/docs/roadmap.mdx` Current Starting Point section
- **WHEN** the Phase 2 closure change is applied
- **THEN** `scryrs hotspots <PATH>` is no longer labeled '(placeholder)'
- **AND** the sentence 'Phase 2 hotspot materialization and later-suite features are deferred' is replaced with text describing the shipped hotspot product
- **AND** the Phase 2 section header reflects delivered status

#### Scenario: cli-v0-contract.md describes real hotspot contract

- **GIVEN** `.devagent/docs/docs/cli-v0-contract.md` 'scryrs hotspots <PATH> (v0 placeholder)' section
- **WHEN** the Phase 2 closure change is applied
- **THEN** the section title no longer includes '(v0 placeholder)'
- **AND** the documented JSON envelope matches the `HotspotsReport` schema (schemaVersion 1.0.0, command, repositoryPath, storePath, runMetadata, generatedAt, entries)
- **AND** the exit-code table covers exit 0 (success with entries or empty), exit 1 (I/O or storage error), exit 2 (missing/unsupported store, usage error)
- **AND** the `.scryrs/hotspots.json` artifact file is documented
- **AND** the agent-facing hotspot contract section describes real SQLite-derived analysis

#### Scenario: architecture.mdx accurately describes hotspot production status

- **GIVEN** `.devagent/docs/docs/architecture.mdx` Current Limitations section
- **WHEN** the Phase 2 closure change is applied
- **THEN** 'Behavior is scaffold-level: commands print placeholders' is updated to reflect that `scryrs hotspots` is production-level (real SQLite analysis, deterministic scoring, artifact file)
- **AND** scaffold-level caveat is retained for graph, curator, and adapter crates

#### Scenario: trace-hook-contract.md canonicalization wording is accurate

- **GIVEN** `.devagent/docs/docs/trace-hook-contract.md` Limitations section
- **WHEN** the Phase 2 closure change is applied
- **THEN** 'Canonicalization for hotspot grouping is deferred to Phase 2' is replaced with text stating that command canonicalization remains a known limitation not scheduled for any current roadmap phase

### Requirement: Accepted limitations are honestly documented

The Phase 2 closure SHALL document limitations that remain after Phase 2 completion.

#### Scenario: No command-subject canonicalization

- **GIVEN** the Phase 2 closure documentation
- **WHEN** a reader inspects the accepted limitations
- **THEN** it states that `CommandExecuted` subject canonicalization is not implemented
- **AND** rewritten and non-rewritten commands (e.g., `ls -la` vs `rtk ls -la`) remain distinct hotspot subjects

#### Scenario: No graph, proposal, or runtime integration

- **GIVEN** the Phase 2 closure documentation
- **WHEN** a reader inspects the accepted limitations
- **THEN** it states that graph building, proposal engine, adapter publishing, runtime retrieval, dashboard, MCP, and LLM features are deferred to Phase 3+

#### Scenario: No per-command help introspection in v0

- **GIVEN** the Phase 2 closure documentation
- **WHEN** a reader inspects the accepted limitations
- **THEN** it states that `--help-json` after command exits 2 (no per-command introspection)

### Requirement: No Phase 3+ scope is introduced

The Phase 2 closure change SHALL NOT introduce graph, proposal, adapter, runtime retrieval, dashboard, MCP, or LLM features, and SHALL NOT regress the existing CLI/tested hotspot pipeline.

#### Scenario: Production code is untouched

- **GIVEN** the diff of the Phase 2 closure change
- **WHEN** a reviewer inspects changed files
- **THEN** no changes exist in `crates/scryrs-cli/src/`, `crates/scryrs-core/src/`, `crates/scryrs-types/src/`, or any test files
- **AND** `cargo test --workspace` passes unchanged

#### Scenario: README.md is not modified

- **GIVEN** the Phase 2 closure change diff
- **WHEN** a reviewer inspects `README.md`
- **THEN** no changes are present — README already accurately describes real hotspot output

#### Scenario: hotspot-report and hotspot-verification specs are not modified

- **GIVEN** the Phase 2 closure change diff
- **WHEN** a reviewer inspects `openspec/specs/hotspot-report/spec.md` and `openspec/specs/hotspot-verification/spec.md`
- **THEN** no changes are present — these specs are already canonical
