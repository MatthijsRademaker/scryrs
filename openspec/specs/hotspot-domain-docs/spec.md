# hotspot-domain-docs Specification

## Purpose
TBD - created by archiving change task-df877d83-d698-4914-9e1a-d4978b9dcf0d. Update Purpose after archive.
## Requirements
### Requirement: A dedicated domain-oriented hotspot page exists

The system SHALL include a new documentation page at `.devagent/docs/docs/hotspots.md` that explains the hotspot concept in domain terms before referencing any schema, architecture, or implementation detail.

#### Scenario: Page file exists with required structure

- **GIVEN** the project docs tree at `.devagent/docs/docs/`
- **WHEN** a reader navigates to the hotspots page
- **THEN** the page opens with a problem statement explaining what user/developer pain hotspots surface (repeated context churn, missing durable knowledge, undiagnosed failure clusters)
- **AND** the page defines what a hotspot represents: repeated context churn around a subject across multiple agent sessions, indicating knowledge that should be documented or investigated
- **AND** the page explains that score is a proxy for context-churn effort, not a measure of code quality, business value, or correctness
- **AND** the page includes a decision-use guidance section explaining concrete actions a developer can take from hotspot output

### Requirement: The page explains the local batch workflow end-to-end

The page SHALL describe the implemented workflow from captured trace events through to the `HotspotsReport` output.

#### Scenario: Workflow section describes the record to SQLite to hotspots pipeline

- **GIVEN** the hotspots.md page
- **WHEN** a reader inspects the workflow section
- **THEN** the section describes: agent hook captures TraceEvent records, then scryrs record persists to .scryrs/scryrs.db, then scryrs hotspots PATH runs deterministic scoring, then HotspotsReport emitted to stdout and .scryrs/hotspots.json
- **AND** the description matches the implemented behavior in crates/scryrs-core/src/scoring.rs and crates/scryrs-cli/src/hotspots.rs

#### Scenario: Page cross-references CLI contract for technical details

- **GIVEN** the hotspots.md page
- **WHEN** the page mentions exit codes, the full JSON envelope shape, or the exact weight table
- **THEN** the page links to cli-v0-contract.md for those details rather than duplicating them

### Requirement: The page teaches field interpretation for core HotspotEntry fields

The page SHALL explain how to interpret each of the following HotspotEntry fields: subject, subjectKind, score, counts (both eventType and outcome sub-maps), sessionCount, and evidence.rowIds.

#### Scenario: Field interpretation covers subject and subjectKind

- **GIVEN** the hotspots.md page
- **WHEN** a reader inspects the field interpretation section
- **THEN** subject is explained as the identity of the hotspot (file path, search query, symbol name, command, or document reference)
- **AND** subjectKind is explained as the category tag: file, search, symbol, command, or document

#### Scenario: Field interpretation covers score

- **GIVEN** the hotspots.md page
- **WHEN** a reader inspects the field interpretation section
- **THEN** score is explained as the sum of weighted event contributions plus failure bonuses per event
- **AND** the explanation states that higher score reflects more agent effort spent around a subject, not semantic importance
- **AND** the explanation does not duplicate the full weight table from cli-v0-contract.md

#### Scenario: Field interpretation covers counts

- **GIVEN** the hotspots.md page
- **WHEN** a reader inspects the field interpretation section
- **THEN** counts.eventType is explained as a breakdown of which agent activity types (FileOpened, EditMade, FailedLookup, etc.) contribute to the subject's score
- **AND** counts.outcome is explained as the split between successful and failed interactions with the subject

#### Scenario: Field interpretation covers sessionCount

- **GIVEN** the hotspots.md page
- **WHEN** a reader inspects the field interpretation section
- **THEN** sessionCount is explained as the number of distinct agent sessions in which the subject appeared
- **AND** the explanation states that high session breadth means the subject is a cross-cutting concern, not a one-session anomaly

#### Scenario: Field interpretation covers evidence.rowIds

- **GIVEN** the hotspots.md page
- **WHEN** a reader inspects the field interpretation section
- **THEN** evidence.rowIds is explained as an ordered list of SQLite row IDs that reference the specific trace_events rows contributing to this hotspot entry
- **AND** the explanation states that these row IDs can be used to trace back to individual agent actions for detailed investigation

### Requirement: The page provides decision-use guidance

The page SHALL explain what decisions a developer can make from hotspot output, connecting score components to concrete actions.

#### Scenario: Decision guidance connects score components to actions

- **GIVEN** the hotspots.md page
- **WHEN** a reader inspects the decision-use section
- **THEN** the section explains that high-scored files are candidates for architecture documentation
- **AND** the section explains that subjects with repeated FailedLookup events indicate knowledge gaps that need documentation
- **AND** the section explains that high sessionCount indicates cross-cutting concerns deserving design decision records
- **AND** the section explains that high failure density in counts.outcome indicates fragile areas needing investigation
- **AND** the section explains that evidence.rowIds can be traced back to specific agent sessions and events for root-cause analysis

### Requirement: Live server hotspot features are labeled as adjacent

Any mention of the live hotspot server (scryrs server), live accumulators, HotspotSignal records, SSE streaming, or the GET /v1/repositories/*/hotspots API SHALL be explicitly labeled as an adjacent deployment mode and SHALL NOT be described as part of the local batch workflow narrative.

#### Scenario: Live server content is scoped to a labeled callout

- **GIVEN** the hotspots.md page
- **WHEN** a reader encounters content about the live hotspot server
- **THEN** the content appears in a clearly labeled section such as "Related: Live Hotspot Server"
- **AND** the section states that the live server is a separate deployment mode via scryrs server
- **AND** the section references cli-v0-contract.md and roadmap.mdx for full live server details

#### Scenario: Live server features are not part of the core narrative

- **GIVEN** the hotspots.md page
- **WHEN** a reader reads the workflow and field interpretation sections, the core narrative
- **THEN** the content describes only the local batch workflow: scryrs record to .scryrs/scryrs.db to scryrs hotspots PATH to HotspotsReport
- **AND** the content does not mention live accumulators, HotspotSignal, SSE, or server API endpoints

### Requirement: The page is discoverable from docs navigation

The docs navigation at .devagent/docs/docs/_nav.json SHALL include an entry linking to the new hotspots page.

#### Scenario: Nav entry exists under Technical section

- **GIVEN** the updated _nav.json file
- **WHEN** a reader opens the docs navigation
- **THEN** an entry with text "Hotspots" and link "/hotspots" appears under the "Technical" navigation section
- **AND** the entry appears after the Trace Hook Contract entry

### Requirement: Existing related pages cross-link to the new hotspots page

At minimum, vision.md, architecture.mdx, cli-v0-contract.md, and trace-hook-contract.md SHALL include a link to the new hotspots page in their Related Pages sections.

#### Scenario: vision.md links to hotspots

- **GIVEN** the updated vision.md file
- **WHEN** a reader inspects the Related Pages section
- **THEN** a link to ./hotspots.md is present with a brief description

#### Scenario: architecture.mdx links to hotspots

- **GIVEN** the updated architecture.mdx file
- **WHEN** a reader inspects the Related Pages section
- **THEN** a link to ./hotspots.md is present with a brief description

#### Scenario: cli-v0-contract.md links to hotspots

- **GIVEN** the updated cli-v0-contract.md file
- **WHEN** a reader inspects the Related Pages section
- **THEN** a link to ./hotspots.md is present with a brief description

#### Scenario: trace-hook-contract.md links to hotspots

- **GIVEN** the updated trace-hook-contract.md file
- **WHEN** a reader inspects the Related Pages section
- **THEN** a link to ./hotspots.md is present with a brief description

### Requirement: All documentation claims are verified against source of truth

Every claim in the hotspots documentation page that describes hotspot scoring, field semantics, ranking, or workflow behavior SHALL be verified against the current source-of-truth files before the page is considered complete.

#### Scenario: Score and weight claims match scoring.rs

- **GIVEN** the hotspots.md page makes a claim about score computation or weight values
- **WHEN** a reviewer compares the claim against crates/scryrs-core/src/scoring.rs
- **THEN** the claim matches the implemented weight constants (WEIGHT_FILE_OPENED=1, WEIGHT_SEARCH_RUN=2, WEIGHT_SYMBOL_INSPECTED=2, WEIGHT_COMMAND_EXECUTED=1, WEIGHT_DOC_RETRIEVED=2, WEIGHT_EDIT_MADE=3, WEIGHT_FAILED_LOOKUP=4, FAILURE_BONUS=2) and the six-key tie-break chain

#### Scenario: Field semantics match the OpenSpec contract

- **GIVEN** the hotspots.md page describes a HotspotEntry field
- **WHEN** a reviewer compares the description against openspec/specs/hotspot-report/spec.md
- **THEN** the description is consistent with the canonical requirement scenarios

#### Scenario: Workflow claims match the CLI contract

- **GIVEN** the hotspots.md page describes the hotspot command workflow
- **WHEN** a reviewer compares the description against .devagent/docs/docs/cli-v0-contract.md
- **THEN** the exit codes, store paths, artifact paths, and output behavior match the documented contract

### Requirement: The docs site builds successfully

The Rspress documentation site SHALL build successfully from .devagent/docs/ with no broken links or build errors after adding the new page and cross-links.

#### Scenario: bun run build completes without errors

- **GIVEN** the updated docs tree including hotspots.md, modified _nav.json, and modified existing pages
- **WHEN** bun run build is executed in .devagent/docs/
- **THEN** the build completes with exit code 0
- **AND** no broken link warnings appear in the build output
- **AND** the new /hotspots route is present in the generated site

### Requirement: No production code, OpenSpec specs, or non-doc artifacts are modified

This change SHALL NOT modify any Rust source code, Cargo configuration, OpenSpec specification files (outside of this change's own specs directory), LikeC4 architecture diagrams, test fixtures, CI configuration, or the root README.md.

#### Scenario: Only documentation files under .devagent/docs/docs/ are changed

- **GIVEN** the diff of this change
- **WHEN** a reviewer inspects changed files
- **THEN** all changed files are under .devagent/docs/docs/
- **AND** no files in crates/, openspec/specs/, .devagent/architecture/, .github/, or the repository root (except this change's OpenSpec artifacts) are modified

#### Scenario: Scoring, ranking, and schema behavior is unchanged

- **GIVEN** the existing test suite for scryrs-core and scryrs-cli
- **WHEN** tests are run after this documentation change
- **THEN** all tests pass with identical results as before the change
- **AND** no snapshot files require updating

