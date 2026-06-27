## ADDED Requirements

### Requirement: Automated verification proves proposal generation only writes to the proposal inbox

The test suite SHALL include a repeatable automated verification that runs `write_proposals` in a temp repository seeded with all protected source-of-truth artifacts and proves that only `.scryrs/proposals/**` files are created or modified.

#### Scenario: Protected files are byte-for-byte unchanged after proposal generation

- **GIVEN** a temp repository seeded with `.scryrs/graph.json`, `.scryrs/routes.json`, and `.devagent/docs/` files with known content
- **WHEN** `write_proposals` runs successfully
- **THEN** `.scryrs/graph.json` content is byte-for-byte identical to its pre-run content
- **AND** `.scryrs/routes.json` content is byte-for-byte identical to its pre-run content
- **AND** every file under `.devagent/docs/` is byte-for-byte identical to its pre-run content

#### Scenario: File inventory confinement proves only inbox writes occur

- **GIVEN** a temp repository seeded with all required input and protected artifacts
- **WHEN** a full file inventory (relative paths with SHA-256 hashes) is computed before and after `write_proposals`
- **THEN** any file that was added or whose hash changed after the run has a path matching `.scryrs/proposals/**`
- **AND** no file outside `.scryrs/proposals/` was created, modified, or deleted

#### Scenario: Hotspots artifact is treated as input, not protected output

- **GIVEN** a temp repository seeded with `.scryrs/hotspots.json` as required input
- **WHEN** the verification runs
- **THEN** `.scryrs/hotspots.json` is NOT included in the protected-sources non-mutation assertion
- **AND** the verification still passes because `write_proposals` does not mutate its input

### Requirement: Verification helper accepts a dynamic protected-paths list

The verification helper SHALL accept a caller-provided list of protected paths rather than hardcoding paths internally. This allows future source-of-truth destinations (e.g., adapter-managed ADR, skill, or memory outputs) to be added without rewriting the test harness.

#### Scenario: Protected paths are configurable at the call site

- **GIVEN** the verification helper function
- **WHEN** a caller invokes it with a list of protected paths
- **THEN** the helper uses the caller-provided list for byte-for-byte comparison
- **AND** the helper does not reference any hardcoded path constants for the protected set

#### Scenario: Default canonical protected set covers the three spec-named artifacts

- **GIVEN** the `source_of_truth_not_mutated()` test
- **WHEN** the test calls the verification helper
- **THEN** it passes `.scryrs/graph.json`, `.scryrs/routes.json`, and `.devagent/docs/` as the protected paths

### Requirement: Test fixtures seed the full protected artifact set

The `source_of_truth_not_mutated()` test SHALL seed `.scryrs/routes.json` with minimal valid content and `.devagent/docs/` with representative files before running `write_proposals`, alongside the existing `.scryrs/graph.json` seed.

#### Scenario: Routes artifact is seeded and verified

- **GIVEN** the `source_of_truth_not_mutated()` test setup
- **WHEN** the test seeds the temp repository
- **THEN** `.scryrs/routes.json` is written with minimal valid content (e.g., `{}`)
- **AND** after `write_proposals`, `.scryrs/routes.json` content is byte-for-byte unchanged

#### Scenario: Docs artifacts are seeded and verified

- **GIVEN** the `source_of_truth_not_mutated()` test setup
- **WHEN** the test seeds the temp repository
- **THEN** `.devagent/docs/docs/_nav.json` is written with representative content
- **AND** at least one `.devagent/docs/docs/*.md` page is written with representative content
- **AND** after `write_proposals`, every file under `.devagent/docs/` is byte-for-byte unchanged

### Requirement: Test coverage lives in the existing proposal CLI test surface

The verification SHALL live in the `#[cfg(test)]` module of `crates/scryrs-cli/src/propose.rs` and SHALL follow the established test patterns (`tempfile::TempDir`, `write_test_hotspots`, `write_test_graph`).

#### Scenario: Verification runs with normal proposal-command test execution

- **GIVEN** a full `cargo test -p scryrs-cli --features curator` invocation
- **WHEN** the test suite executes
- **THEN** the strengthened `source_of_truth_not_mutated` test runs and passes
- **AND** the test fails loudly (assertion panic) if any protected path is mutated or any non-proposals write occurs

#### Scenario: Verification uses existing test infrastructure

- **GIVEN** the test module in `propose.rs`
- **WHEN** the verification test is inspected
- **THEN** it uses `tempfile::TempDir` for repository isolation
- **AND** it reuses `write_test_hotspots` and `write_test_graph` helpers
- **AND** it introduces no new external crate dependencies

### Requirement: Bug-contingent fix is scoped to the offending write instruction only

If the strengthened verification reveals a write-path bug in production code, the fix SHALL be limited to the specific instruction that writes outside `.scryrs/proposals/`. The fix SHALL NOT redesign proposal heuristics, target types, inbox semantics, or curator engine behavior.

#### Scenario: Production code is not changed unless verification exposes a real bug

- **GIVEN** the strengthened `source_of_truth_not_mutated` test
- **WHEN** the test passes without production code changes
- **THEN** `write_proposals` remains unchanged
- **AND** the delivered value is stronger executable safety proof alone

#### Scenario: Write-path bug fix is surgical

- **GIVEN** the strengthened test reveals that `write_proposals` writes outside `.scryrs/proposals/`
- **WHEN** the bug is fixed
- **THEN** only the offending write instruction is modified
- **AND** proposal heuristics, target types, and inbox semantics are unchanged
- **AND** the fixed code passes the strengthened test