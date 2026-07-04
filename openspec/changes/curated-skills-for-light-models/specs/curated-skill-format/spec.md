# curated-skill-format

## ADDED Requirements

### Requirement: Curated skill provenance frontmatter
Every curated repo-specific skill in `.pi/skills/` SHALL declare a `metadata` frontmatter block containing `curated: true`, a `sources` list of repo-relative glob patterns covering the code the skill describes, a `last-verified` ISO date, and a `verified-commit` git SHA.

#### Scenario: Curated skill declares sources
- **WHEN** a curated skill's SKILL.md frontmatter is parsed
- **THEN** it contains `curated: true`, at least one `sources` glob, a `last-verified` date, and a `verified-commit` SHA that exists in the repository history

#### Scenario: Sources globs match real files
- **WHEN** each glob in a curated skill's `sources` list is expanded against the working tree
- **THEN** every glob matches at least one existing file

### Requirement: Routing-grade description format
Every curated skill's frontmatter `description` SHALL follow the format: one sentence stating the capability, followed by `Use when:` with concrete triggers (literal repo paths, symbols, or task phrasings), followed by `Do not use for:` naming the adjacent skill or territory it must not be selected for.

#### Scenario: Description contains routing clauses
- **WHEN** a curated skill's description is inspected
- **THEN** it contains both a `Use when:` clause with at least one literal repo path and a `Do not use for:` clause

### Requirement: Recipe skill body structure
Every curated recipe skill body SHALL contain, in order: a **Read first** section listing 1–3 files each with a one-line reason; a **Steps** section where every step that changes state ends with a `Verify:` line containing a runnable command and its expected observation; an **Easy-to-miss details** section; and a **Self-check** section of yes/no items to pass before reporting the work outcome.

#### Scenario: Steps carry executable exit conditions
- **WHEN** a step in a curated recipe skill's Steps section is read
- **THEN** it ends with a `Verify:` line whose command can be executed in the agent environment and whose expected result is stated

#### Scenario: Read-first list is bounded
- **WHEN** the Read first section is inspected
- **THEN** it lists at most 3 files, each with a stated reason

### Requirement: Convention is documented for future authors
The curated-skill convention (frontmatter contract, description format, body structure) SHALL be documented in a location referenced from `AGENTS.md`, so future skill authors and the curator follow the same format.

#### Scenario: Author discovers the convention
- **WHEN** a contributor reads `AGENTS.md` looking for skill-authoring guidance
- **THEN** it links to the curated-skill convention document
