# production-suite-truth Specification

## Purpose
TBD - created by archiving change task-7f0b6e6e-7c4b-46a7-ac4d-70036c6c0529. Update Purpose after archive.
## Requirements
### Requirement: Route-manifests current-state gap describes shipped explain, not missing command

The Current State table row for Route manifests in `production-suite.md` SHALL NOT claim that route explanation is missing. It SHALL state that the explain surface is shipped while the automatic context-loading loop is still missing.

#### Scenario: Gap text does not claim explanation is missing

- **GIVEN** `scryrs route explain --query` is a shipped CLI command
- **WHEN** `production-suite.md` Current State table is read
- **THEN** the Route manifests row SHALL NOT contain text claiming "Runtime explanation and context loading decisions missing"

#### Scenario: Gap text describes the actual limitation

- **GIVEN** `scryrs route explain` uses deterministic manifest matching without automatic context loading
- **WHEN** `production-suite.md` Current State table is read
- **THEN** the Route manifests row SHALL state that `scryrs route explain` is shipped
- **AND** it SHALL state the remaining gap: automatic context-loading loop still missing (deterministic manifest matching only, no semantic retrieval or autonomous source/docs loading)

### Requirement: P4 milestone marks shipped items with ✅ Shipped

The Milestones table row P4 (Runtime explain) in `production-suite.md` SHALL mark shipped items with the ✅ Shipped marker.

#### Scenario: P4 milestone has ✅ Shipped marker

- **GIVEN** route hint schema, `scryrs route explain`, and deterministic evidence-backed reasons are all shipped
- **WHEN** the Milestones table is read
- **THEN** the P4 row SHALL include a ✅ Shipped marker in the Must-ship column for route hint schema, `scryrs route explain`, and deterministic evidence-backed reasons
- **AND** the format SHALL match the existing P3 milestone shipped marker convention

#### Scenario: P4 milestone notes remaining gap

- **GIVEN** route explain is shipped but automatic context loading is not
- **WHEN** the Milestones table is read
- **THEN** the P4 row SHALL note the remaining gap is semantic usefulness / automated context loading

### Requirement: Production-suite links real remaining gaps

The `production-suite.md` page SHALL link to remaining real production gaps rather than referencing stale missing-feature claims.

#### Scenario: Live hotspot browser verification gap is linked

- **GIVEN** the live-hotspot browser automation gap is tracked by an active OpenSpec change or board task
- **WHEN** `production-suite.md` is read
- **THEN** the relevant gap SHALL reference real remaining work
- **AND** it SHALL NOT claim a shipped command surface is missing

#### Scenario: Release distribution gap is linked

- **GIVEN** public binary and image distribution is tracked by an active OpenSpec change or board task
- **WHEN** `production-suite.md` is read
- **THEN** the relevant gap SHALL reference real remaining work
- **AND** it SHALL NOT claim a shipped command surface is missing

