## ADDED Requirements

### Requirement: scryrs ships a curated agent context skill that routes agents to retrieval

scryrs SHALL ship a curated skill whose purpose is to route a coding agent to scryrs retrieval before the agent falls back to unguided searching. The skill SHALL instruct only — it SHALL NOT inject context into agent prompts, register scryrs as an agent-callable tool, or modify agent-visible tool output. The observer-first non-interference boundary SHALL be preserved.

The skill SHALL be a curated artifact under `.pi/skills/CONVENTION.md` carrying `curated: true`, `sources` globs covering every path it makes claims about, `last-verified`, and `verified-commit`, and SHALL pass `scripts/skill-lint`.

#### Scenario: Skill instructs rather than injects

- **WHEN** the skill is installed and an agent session runs
- **THEN** nothing is injected into the agent's prompt and no scryrs tool is registered
- **AND** the agent invokes retrieval only by choosing to run the documented commands

#### Scenario: Skill carries the curated frontmatter contract

- **WHEN** the shipped skill's frontmatter is read
- **THEN** it declares `curated: true`, `sources`, `last-verified`, and `verified-commit`
- **AND** the `sources` globs cover every repository path the skill body makes claims about

#### Scenario: Skill passes the lint floor

- **WHEN** `scripts/skill-lint` runs
- **THEN** the shipped context skill passes

#### Scenario: Description is a routing trigger

- **WHEN** the skill's `description` is read
- **THEN** it states when to use the skill and when not to, in terms a model can route on

### Requirement: The skill documents the read-side CLI surface accurately

The skill SHALL document the scryrs read-side command surface with flags and behavior verified against the shipped CLI: `scryrs route bundle`, `scryrs route explain`, `scryrs hotspots`, `scryrs graph`, and `scryrs doctor`. It SHALL document the prerequisite chain between them, and SHALL distinguish `bundle` from `explain`.

#### Scenario: Prerequisite chain is documented

- **WHEN** the skill is read
- **THEN** it states that `scryrs graph <PATH>` writes `.scryrs/graph.json`
- **AND** that `scryrs route <PATH>` consumes the graph artifact and writes `.scryrs/routes.json`
- **AND** that `route explain` and `route bundle` are read-only over the route manifest

#### Scenario: bundle versus explain is unambiguous

- **WHEN** the skill is read
- **THEN** it identifies `route bundle` as the bounded context-loading planning surface
- **AND** identifies `route explain` as the unbounded diagnostic ranking surface
- **AND** directs an agent loading context to use `bundle`

#### Scenario: Documented flags match the shipped CLI

- **WHEN** each command documented in the skill is compared against `scryrs --help-json`
- **THEN** every documented subcommand and flag exists in the shipped CLI surface

### Requirement: The skill teaches the route hint contract needed to act on output

The skill SHALL document the `RouteHintItem` and `RouteBundleDocument` fields an agent must interpret in order to act: `loadTarget` kinds, the difference between `rank` and explain `relevance`, evidence citations, and `reason`.

#### Scenario: loadTarget kinds are explained

- **WHEN** the skill is read
- **THEN** it states that a `file` load target is a repository-relative path to read
- **AND** that a `doc_page` load target is `project-docs/<slug>`
- **AND** that a `non_loadable` target has nothing to read

#### Scenario: non_loadable is framed as a correct result

- **WHEN** the skill documents `non_loadable`
- **THEN** it states that `search`, `symbol`, `domain_term`, and `doc_group` routes are explicitly non-loadable
- **AND** that this is an expected result, not an error, and not something to retry or work around

#### Scenario: rank and relevance are distinguished

- **WHEN** the skill is read
- **THEN** it states that `rank` is the manifest ordinal
- **AND** that explain `relevance` is a packed score absent from plain route projection
- **AND** that the two are not interchangeable for ordering reads

### Requirement: The skill states its preconditions and the search fallback honestly

The skill SHALL document unmet preconditions as expected states with a defined response, and SHALL state the search fallback explicitly. It SHALL NOT present a missing manifest or an empty result as a failure to route around silently.

#### Scenario: Missing route manifest is a documented state

- **WHEN** `.scryrs/routes.json` does not exist
- **THEN** the skill tells the agent what that means and what to do

#### Scenario: Empty hints array is a documented state

- **WHEN** a query returns a valid document with an empty `hints` array
- **THEN** the skill states this is a valid zero-match result, not an error

#### Scenario: Stale manifest is a documented state

- **WHEN** the route manifest is older than the working tree
- **THEN** the skill states what that implies for the returned hints

#### Scenario: Search fallback is explicit

- **WHEN** retrieval returns no usable load targets
- **THEN** the skill directs the agent to search normally
- **AND** notes that the fallback is itself the signal scryrs captures
