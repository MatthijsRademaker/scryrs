## ADDED Requirements

### Requirement: scryrs publish skill exists and is discoverable

The system SHALL expose `scryrs publish skill <PATH> --output <DIR>` as a documented subcommand under the existing `publish` namespace, implemented in a `crates/scryrs-adapter-skill` crate behind its own Cargo feature, following the shape of the existing Markdown and Rspress adapters. The existing `publish markdown` and `publish rspress` surfaces SHALL remain unchanged.

#### Scenario: Skill publishing appears in help output

- **WHEN** `scryrs --help` and `scryrs publish --help` are run
- **THEN** the output includes a `scryrs publish skill <PATH> --output <DIR>` entry

#### Scenario: Skill publishing appears in help-json output

- **WHEN** `scryrs --help-json` is run
- **THEN** the JSON output includes a `skill` subcommand entry under the `publish` command

#### Scenario: Existing publish adapters are preserved

- **WHEN** `scryrs publish markdown` or `scryrs publish rspress` is run
- **THEN** its stdout, stderr, exit code, and written output are unchanged

### Requirement: Skill publishing reads accepted artifacts only

The skill adapter SHALL read `.scryrs/accepted/` only. Pending and rejected artifacts SHALL never publish. Publishing SHALL remain explicit: `scryrs proposals accept` SHALL NOT publish.

#### Scenario: Only accepted skill artifacts publish

- **GIVEN** skill proposals exist in `.scryrs/proposals/`, `.scryrs/accepted/`, and `.scryrs/rejected/`
- **WHEN** `scryrs publish skill` runs
- **THEN** only the accepted skill artifacts are published

#### Scenario: Accepting does not publish

- **WHEN** `scryrs proposals accept` is run on a skill proposal
- **THEN** no skill output is written

#### Scenario: Non-skill target types are ignored without failing

- **GIVEN** `.scryrs/accepted/` contains `docs_note`, `adr`, and `memory_patch` artifacts alongside `skill` artifacts
- **WHEN** `scryrs publish skill` runs
- **THEN** only `skill`-target artifacts are published
- **AND** the command exits 0

### Requirement: Published skills are structurally loadable

Each published skill SHALL be written as one directory containing a `SKILL.md` whose YAML frontmatter is valid and whose `name` matches its containing directory name, so that a harness can load it.

#### Scenario: Output layout is one directory per skill

- **WHEN** `scryrs publish skill <PATH> --output <DIR>` runs against accepted skill artifacts
- **THEN** each published skill occupies its own directory under `<DIR>` containing a `SKILL.md`

#### Scenario: Frontmatter name matches the directory

- **WHEN** a published `SKILL.md` is read
- **THEN** its frontmatter parses as valid YAML
- **AND** its `name` equals its containing directory name

#### Scenario: A published skill loads in a real harness

- **WHEN** a published skill directory is placed in a harness skill location
- **THEN** the harness loads it and the skill is routable by name

### Requirement: Skill publishing is deterministic and non-destructive

Output SHALL be deterministic for identical input, and the adapter SHALL never delete stale output — matching the existing Markdown adapter's contract. The command SHALL emit a JSON summary consistent with the other publish adapters.

#### Scenario: Repeated runs produce identical output

- **WHEN** `scryrs publish skill` runs twice against unchanged accepted artifacts
- **THEN** the written output is byte-identical

#### Scenario: Stale output is never deleted

- **GIVEN** the output directory contains a previously published skill with no corresponding accepted artifact
- **WHEN** `scryrs publish skill` runs
- **THEN** the stale skill directory is left in place

#### Scenario: JSON summary is emitted

- **WHEN** `scryrs publish skill` succeeds
- **THEN** it emits a JSON summary consistent in shape with the Markdown and Rspress publish summaries

#### Scenario: Invalid accepted artifact fails distinguishably

- **GIVEN** `.scryrs/accepted/` contains a malformed accepted artifact
- **WHEN** `scryrs publish skill` runs
- **THEN** it fails with a validation error distinguishable from a filesystem error

### Requirement: Automated skill publishing is gated by a pull request

When skill generation and publishing run as an automated loop, the human review gate SHALL be a pull request. The loop SHALL NOT write into a live skill directory and SHALL NOT merge its own output.

A review decision staged by automation SHALL record the automating identity in `reviewer` and SHALL NOT record a human name. Both the review-decision artifact and the generated `SKILL.md` SHALL appear in the same pull request so a reviewer sees the claim and the output together.

#### Scenario: The loop opens a pull request rather than writing live skills

- **WHEN** the automated loop generates and publishes a skill
- **THEN** the output appears as a proposed diff on a pull request branch
- **AND** no live harness skill directory is modified

#### Scenario: Staged decisions name the automation, not a human

- **WHEN** the loop stages a review decision in order to publish
- **THEN** the artifact's `reviewer` field is the automating identity
- **AND** it is not a human name

#### Scenario: Decision artifact and generated skill travel together

- **WHEN** the loop opens a pull request
- **THEN** the pull request contains both the review-decision artifact and the generated `SKILL.md`

#### Scenario: Merge is the acceptance event

- **WHEN** a human merges the pull request
- **THEN** the generated skill reaches the default branch
- **AND** when a human closes it without merging, the generated skill does not

#### Scenario: No auto-merge

- **WHEN** the automated loop completes
- **THEN** it does not merge its own pull request and does not bypass branch protection
