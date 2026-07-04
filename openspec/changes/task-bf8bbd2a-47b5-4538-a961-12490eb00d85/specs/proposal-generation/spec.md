## ADDED Requirements

### Requirement: Repeated FailedLookup evidence generates debugging playbook proposals

The curator engine SHALL generate a `debugging_playbook` proposal for hotspot entries where `counts.eventType["FailedLookup"] >= 2`. The generated proposal SHALL use `ProposedContent::Markdown` with a deterministic template that includes a subject header, an Observed Failure Signal section citing the FailedLookup count and score, a Likely Causes section with placeholder bullets, and an Evidence section referencing row IDs. `debugging_playbook` proposals SHALL be additive with `skill` and `memory_patch` proposals — a hotspot crossing multiple thresholds SHALL generate all qualifying proposal types.

#### Scenario: Repeated FailedLookup events generate a playbook proposal

- **GIVEN** a hotspot entry with `counts.eventType["FailedLookup"] >= 2`
- **WHEN** the curator engine processes it
- **THEN** a `debugging_playbook` proposal is generated with `targetType = debugging_playbook`
- **AND** `proposedContent` is non-empty markdown containing a subject header
- **AND** the markdown includes an "Observed Failure Signal" section citing the FailedLookup count and score
- **AND** the markdown includes a "Likely Causes" section with placeholder bullets
- **AND** the markdown includes an "Evidence" section referencing row IDs
- **AND** the proposal's evidence links use `sourceKind = hotspot_subject` and carry the hotspot entry's `rowIds`
- **AND** the proposal validates as a `ProposalDocument`

#### Scenario: Below threshold generates no debugging_playbook

- **GIVEN** a hotspot entry with `counts.eventType["FailedLookup"]` equal to 0 or 1
- **WHEN** the curator engine processes it
- **THEN** no `debugging_playbook` proposal is generated
- **AND** single low-signal failed lookups do not spam the proposal inbox

#### Scenario: debugging_playbook is additive with skill and memory_patch

- **GIVEN** a hotspot entry with `counts.eventType["FailedLookup"] >= 2`, at least one failure outcome, and a score crossing the memory_patch threshold
- **WHEN** the curator engine processes it
- **THEN** a `debugging_playbook` proposal, a `skill` proposal, and a `memory_patch` proposal SHALL all be generated for the same hotspot subject
- **AND** the additive overlap is intentional v1 behavior — different target types serve different consumption paths

## MODIFIED Requirements

### Requirement: V1 deterministic rules map evidence to target types

The curator engine SHALL apply concrete, testable heuristics to generate proposals for each supported target type including `debugging_playbook`. Rules SHALL be deterministic: the same inputs always produce the same proposals.

#### Scenario: docs_note — every hotspot entry

- **GIVEN** a hotspot report with N entries
- **WHEN** the curator engine processes them
- **THEN** exactly N `docs_note` proposals are generated (one per entry)
- **AND** each proposal cites the entry's subject and score
- **AND** each proposal's evidence links use `sourceKind = hotspot_subject`

#### Scenario: skill — hotspot entries with Failure outcomes

- **GIVEN** hotspot entries where some have `outcome.Failure > 0` and some have only `Success` outcomes
- **WHEN** the curator engine processes them
- **THEN** `skill` proposals are generated only for entries with at least one `Failure` outcome event
- **AND** entries without any `Failure` outcomes do not generate skill proposals

#### Scenario: memory_patch — high-failure-ratio entries

- **GIVEN** a hotspot entry with `score >= 4` and failure-ratio >= 0.5 (Failure events >= 50% of total outcome events)
- **WHEN** the curator engine processes it
- **THEN** a `memory_patch` proposal is generated with structured JSON `proposedContent`
- **AND** entries with `score < 4` or failure-ratio < 0.5 do not generate memory_patch proposals

#### Scenario: adr — cross-kind hotspot clusters

- **GIVEN** hotspot entries for the same subject string appearing across >= 2 distinct `subjectKind` values with aggregate score >= 10
- **WHEN** the curator engine processes the hotspot report
- **THEN** an `adr` proposal is generated for each such cluster
- **AND** the proposal cites evidence from all contributing hotspot entries
- **AND** clusters with only one subject kind or aggregate score < 10 do not generate ADR proposals

#### Scenario: semantic_graph_grouping — cross-kind graph node families

- **GIVEN** graph nodes sharing the same subject stem (e.g., `auth`) across >= 2 distinct subject kinds with at least one shared hotspot-backed evidence link
- **WHEN** the curator engine processes graph nodes and hotspot evidence
- **THEN** a `semantic_graph_grouping` proposal is generated with `targetType = semantic_graph_grouping`
- **AND** `proposedContent.sourceNodeIds` lists the exact source graph node IDs
- **AND** `proposedContent.targetGroupNodeId` is set to `domain_term:<subject>`
- **AND** `proposedContent.targetGroupLabel` is set to the subject stem

#### Scenario: debugging_playbook — generated for repeated FailedLookup events (see ADDED requirement)

- **GIVEN** a hotspot entry with `counts.eventType["FailedLookup"] >= 2`
- **WHEN** the curator engine generates proposals
- **THEN** a `debugging_playbook` proposal is generated as specified in the "Repeated FailedLookup evidence generates debugging playbook proposals" requirement

## REMOVED Requirements

None — the "No `debugging_playbook` proposals SHALL be generated" clause is removed from the "V1 deterministic rules map evidence to target types" requirement header, and the "debugging_playbook is never generated" scenario is replaced by the MODIFIED scenario above.