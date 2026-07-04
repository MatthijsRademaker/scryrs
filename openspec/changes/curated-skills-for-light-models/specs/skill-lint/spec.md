# skill-lint

## ADDED Requirements

### Requirement: Referenced paths must exist
`scripts/skill-lint` SHALL scan every `.pi/skills/*/SKILL.md`, extract repo-relative path tokens (prefixes: `crates/`, `scripts/`, `hooks/`, `tests/`, `.pi/`, `.devagent/`, `openspec/`, `xtask/`, `examples/`, `fuzz/`), and exit non-zero listing each token that does not exist in the working tree, including the skill file and token that failed.

#### Scenario: Stale path fails the lint
- **WHEN** a SKILL.md references `src/manager/flowcontroller/controller.go` and no such path exists
- **THEN** `scripts/skill-lint` exits non-zero and its output names the skill file and the missing path

#### Scenario: Clean skills pass
- **WHEN** every extracted path token in every SKILL.md exists in the working tree
- **THEN** `scripts/skill-lint` exits zero

### Requirement: Placeholder and suppression handling
The lint SHALL skip tokens containing `<`, `>`, or `*` (placeholders and globs), SHALL validate `sources` frontmatter globs by requiring each to match at least one file, and SHALL skip any line containing the marker `skill-lint-ignore`.

#### Scenario: Placeholder path is ignored
- **WHEN** a SKILL.md contains `crates/scryrs-cli/src/commands/<name>.rs`
- **THEN** the lint does not report it as missing

#### Scenario: Unmatched sources glob fails
- **WHEN** a curated skill's `sources` includes a glob matching zero files
- **THEN** the lint exits non-zero naming the skill and the dead glob

#### Scenario: Intentional example suppressed
- **WHEN** a line references a nonexistent path and contains `skill-lint-ignore`
- **THEN** the lint does not report that line

### Requirement: Runs without language SDKs and in pre-commit
The lint SHALL be implemented in plain shell (bash + POSIX tools) so it runs on the host and inside agent containers without Rust, Node, or Python, and SHALL be registered as a hook in `.pre-commit-config.yaml` so it executes via the existing `scripts/precommit-run` path.

#### Scenario: Lint runs in agent container
- **WHEN** `scripts/skill-lint` is invoked in an environment with only bash and coreutils
- **THEN** it completes without requiring any language runtime

#### Scenario: Pre-commit catches skill rot
- **WHEN** a commit introduces a SKILL.md referencing a nonexistent path and `scripts/precommit-run` executes
- **THEN** the run fails on the skill-lint hook
