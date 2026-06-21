## ADDED Requirements

### Requirement: Phase 3 Dashboard is defined on the roadmap

The roadmap SHALL define Phase 3 (Dashboard) as the phase following Phase 2 (Hotspot Materialization) and preceding Phase 4 (Graph and Route Manifests). The phase SHALL describe a `scryrs dashboard` CLI command, a local HTTP server, and a Vue.js SPA frontend that visualizes hotspot, session, and event data from the local `.scryrs/` store.

#### Scenario: Dashboard is next after Phase 2

- **WHEN** a reader inspects the roadmap
- **THEN** Phase 3 SHALL be titled "Dashboard" and follow Phase 2 (Hotspot Materialization) in delivery order
- **AND** Phase 4 SHALL be "Graph and Route Manifests" (moved from Phase 3)

#### Scenario: Phase 3 Dashboard has documented deliverables

- **WHEN** a reader inspects the Phase 3 section of the roadmap
- **THEN** it SHALL list the required deliverables: `scryrs dashboard` CLI command, local HTTP server, Vue.js SPA frontend, hotspot report visualization, session timeline view, event distribution view, per-subject drill-down, and component architecture extensible for future phases

#### Scenario: Phase 3 explicitly defers later features

- **WHEN** a reader inspects the Phase 3 section of the roadmap
- **THEN** it SHALL explicitly state that graph visualization, route exploration, hosted/multi-user dashboard, real-time updates, and data mutation are deferred to later phases

### Requirement: Scope guardrail #5 is reconciled

Scope guardrail #5 ("No dashboard or hosted analytics product before standalone CLI is useful") SHALL be updated to reflect that dashboard is now a planned phase. The guardrail MAY be removed entirely since Phase 2 is delivered and the CLI is useful, or rewritten to defer hosted analytics while permitting the local dashboard.

#### Scenario: Guardrail no longer blocks local dashboard

- **WHEN** a reader inspects the roadmap scope guardrails
- **THEN** guardrail #5 SHALL either be removed or rewritten to differentiate between "local CLI dashboard" (planned) and "hosted analytics" (still deferred)
- **AND** the rewritten guardrail SHALL NOT block the Phase 3 `scryrs dashboard` command

### Requirement: Phase numbering is consistent throughout the roadmap

All phases after Phase 2 SHALL be renumbered so that the dashboard phase occupies Phase 3 and every downstream phase shifts by one position. References to phase numbers elsewhere in the project (architecture docs, spec files, design documents) that mention Phase 3 through Phase 7 SHALL be updated to reflect the new numbering.

#### Scenario: Phase numbers shift by one

- **WHEN** a reviewer reads the roadmap after the change
- **THEN** Phase 3 SHALL be "Dashboard"
- **AND** Phase 4 SHALL be "Graph and Route Manifests" (was Phase 3)
- **AND** Phase 5 SHALL be "Proposal Engine" (was Phase 4)
- **AND** Phase 6 SHALL be "Publishing Adapters" (was Phase 5)
- **AND** Phase 7 SHALL be "Runtime Retrieval" (was Phase 6)
- **AND** Phase 8 SHALL be "Optional LLM Layer" (was Phase 7)
