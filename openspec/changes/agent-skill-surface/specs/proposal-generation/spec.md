## MODIFIED Requirements

### Requirement: Skill proposals carry a valid SKILL.md draft grounded in evidence

A `skill`-target proposal's `proposed_content` SHALL be a Markdown document that is a structurally valid `SKILL.md` draft, not a metadata summary. It SHALL carry YAML frontmatter with a kebab-case `name` and a `description` written as a routing trigger, and a body grounded in the evidence that triggered generation.

Generation SHALL remain fully deterministic. No model-backed drafting SHALL be introduced into this path; `scryrs-curator-llm` remains library-only and off the deterministic path.

The generated artifact SHALL be marked as machine-generated, carrying its source proposal id, so it is never mistaken for a hand-curated skill.

#### Scenario: Generated skill content has valid frontmatter

- **WHEN** a `skill` proposal is generated
- **THEN** its `proposed_content` begins with YAML frontmatter that parses
- **AND** the frontmatter carries a kebab-case `name` and a `description`

#### Scenario: Description is written as a routing trigger

- **WHEN** the generated frontmatter `description` is read
- **THEN** it states the condition under which an agent should load the skill

#### Scenario: Body is grounded in the triggering evidence

- **WHEN** a `skill` proposal is generated from a hotspot entry
- **THEN** the body names the subject and the failure pattern that triggered generation
- **AND** it cites the evidence row ids a reviewer can verify against the store

#### Scenario: Generated content is marked machine-generated

- **WHEN** the generated content is read
- **THEN** it is identifiably machine-generated and carries its source proposal id

#### Scenario: Generation is deterministic

- **WHEN** a `skill` proposal is generated twice from identical hotspot input
- **THEN** the `proposed_content` and the computed proposal id are identical

#### Scenario: No model-backed drafting in the deterministic path

- **WHEN** `scryrs propose` runs
- **THEN** no model call is made during `skill` proposal generation

#### Scenario: Generated content passes the skill lint structural floor

- **WHEN** the generated draft is written out as a `SKILL.md`
- **THEN** `scripts/skill-lint` passes its structural checks

#### Scenario: Proposal ids change from the prior stub content

- **GIVEN** `skill` proposals previously generated the metadata-stub content
- **WHEN** proposals are regenerated after this change
- **THEN** the content-addressed proposal ids differ from the prior ids
- **AND** this is a documented, accepted replacement rather than a preserved-compatibility path
