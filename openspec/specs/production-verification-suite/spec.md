# production-verification-suite Specification

## Purpose
TBD - created by archiving change task-782aa74d-a495-4a24-b877-ae014b85137b. Update Purpose after archive.
## Requirements
### Requirement: `scripts/verify-production-suite` is the authoritative headless production-readiness entrypoint

The repository SHALL provide `scripts/verify-production-suite` as the single authoritative production-readiness gate. The command SHALL run headlessly in the existing Docker-backed verification posture, print clear per-lane headers, and exit non-zero when any required lane fails. `scripts/precommit-run --production` SHALL invoke this heavy lane explicitly without making it the default PR-gate path.

#### Scenario: Maintainer runs the production suite explicitly

- **GIVEN** Docker or DinD is available
- **WHEN** a maintainer runs `scripts/verify-production-suite`
- **THEN** the command runs the documented production-verification lanes headlessly
- **AND** each lane is identified clearly in the output
- **AND** the command exits non-zero if any lane fails

#### Scenario: Precommit wrapper exposes but does not default the heavy lane

- **WHEN** a maintainer runs `scripts/precommit-run --production`
- **THEN** the wrapper invokes the production suite
- **AND** the existing default `scripts/precommit-run` behavior remains lighter-weight than the production suite
- **AND** the production suite is not implied to be the default PR-gate lane in this change

### Requirement: Production verification includes a deterministic core artifact-loop lane through the real binary

The production suite SHALL include a dedicated deterministic verification lane for the local artifact loop: `record -> hotspots -> graph -> route -> propose -> proposals accept`. This lane SHALL run through the real `scryrs` binary against fixture input and SHALL assert that the expected deterministic artifacts are produced in their documented locations.

#### Scenario: Core artifact loop produces the expected deterministic artifacts

- **GIVEN** a deterministic fixture repository and the real `scryrs` binary
- **WHEN** the core artifact-loop verification runs
- **THEN** it executes `record`, `hotspots`, `graph`, `route`, `propose`, and `proposals accept`
- **AND** it verifies `.scryrs/scryrs.db`, `.scryrs/hotspots.json`, `.scryrs/graph.json`, `.scryrs/routes.json`, `.scryrs/proposals/`, and `.scryrs/accepted/` are produced or updated in the expected deterministic locations
- **AND** the lane fails loudly if any step or artifact assertion fails

#### Scenario: Proposal review acceptance is part of the verified loop

- **GIVEN** the deterministic core artifact-loop fixture generates at least one proposal
- **WHEN** the lane runs proposal review
- **THEN** it accepts the proposal through the shipped review command path
- **AND** it verifies accepted evidence is written under `.scryrs/accepted/`
- **AND** the lane does not depend on LLM output or network state

### Requirement: Production verification composes the existing release lanes instead of reimplementing them

`scripts/verify-production-suite` SHALL compose the existing verification lanes plus the new core artifact-loop lane rather than duplicating their logic. The required composed coverage is:

- `scripts/check`
- `scripts/test --full`
- `scripts/security`
- `scripts/verify-install`
- `scripts/verify-trace-capture`
- `scripts/verify-live-hotspots`
- the deterministic core artifact-loop lane
- `scripts/verify-docs-publish`
- a runnable privacy assertion lane

#### Scenario: Release gate covers the documented production lanes

- **GIVEN** a release candidate workspace
- **WHEN** `scripts/verify-production-suite` runs
- **THEN** it executes the documented production lanes in one authoritative flow
- **AND** it does not replace those lanes with duplicated inline logic
- **AND** the final exit status reflects the first failed required lane

#### Scenario: Live server coverage is included and dashboard automation stays out of scope

- **GIVEN** the production suite includes live verification
- **WHEN** the live lane runs
- **THEN** it uses the existing live server verification path to prove remote ingest, idempotent replay, hotspot query, and SSE replay/resume
- **AND** the automated gate does not claim live dashboard browser verification
- **AND** the dashboard boundary is documented explicitly instead

### Requirement: Privacy and security checks are runnable and prove the release boundaries called out by refinement

The production suite SHALL include runnable privacy/security checks that cover dependency policy and telemetry/privacy defaults. Privacy defaults SHALL be asserted programmatically through compiled tests, not by grepping source text. The release documentation SHALL map the remaining privacy boundaries to their proving lanes or documented checks: telemetry opt-in defaults, redaction defaults, debug-gated Bash capture, fail-open hook behavior, and remote-mode no-dual-write / no-local-fallback behavior.

#### Scenario: Telemetry privacy defaults are asserted programmatically

- **WHEN** the privacy assertion lane runs as part of the production suite
- **THEN** it verifies the compiled telemetry/privacy defaults programmatically
- **AND** it fails if telemetry opt-in, redaction defaults, or remote prompt-storage defaults drift from their safe release posture
- **AND** it does not rely on source-text inspection alone

#### Scenario: Release docs explain how trace/privacy boundaries are proved

- **WHEN** a maintainer reads the production verification documentation
- **THEN** they can see which runnable lane proves dependency policy and privacy defaults
- **AND** they can see where debug-gated Bash capture, fail-open hooks, and remote no-dual-write behavior are covered or checked
- **AND** the documentation does not imply privacy guarantees without a stated proving path

### Requirement: Production documentation links diagnosis, gate entrypoints, runtime posture, and packaging boundaries

The production-hardening docs SHALL update the authoritative operator pages to link `scryrs doctor` and `scripts/verify-production-suite`, explain lane prerequisites, expected runtime/posture, and failure interpretation, and state the packaging verification boundary. Linux install verification SHALL remain automated through the existing installer verification path. macOS verification SHALL be documented as an explicit manual maintainer lane with exact commands and a clear statement that current Linux-only automation does not prove Darwin behavior.

#### Scenario: Production Suite Plan links the operator entrypoints

- **WHEN** `.devagent/docs/docs/production-suite.md` is read
- **THEN** it links to the diagnostic path for `scryrs doctor`
- **AND** it links to `scripts/verify-production-suite` as the authoritative release gate
- **AND** it explains how those entrypoints fit into Production Hardening 01

#### Scenario: Verification README documents runtime posture and failure interpretation

- **WHEN** `scripts/verification/README.md` is read
- **THEN** it documents each production-suite lane, prerequisites, expected posture, and how to interpret failures
- **AND** it states that live dashboard verification remains a documented manual smoke path for this change

#### Scenario: macOS verification is documented honestly

- **WHEN** the packaging or verification docs describe macOS verification
- **THEN** they provide the exact manual maintainer commands to run on macOS
- **AND** they state explicitly that current Linux Docker automation proves Linux install behavior only
- **AND** they do not claim automated native macOS proof unless a real macOS runner is added

### Requirement: Publish verification exercises the shipped CLI surface

The production verification suite SHALL verify accepted-knowledge publishing through the real `scryrs` binary rather than only through adapter examples. `scripts/verify-docs-publish` or the production lane it invokes SHALL execute both `scryrs publish markdown <PATH> --output <DIR>` and `scryrs publish rspress <PATH> --docs-root <DIR>` against deterministic fixtures before completing the existing docs build and llms-surface assertions.

#### Scenario: Production verification exercises markdown publish through the CLI

- **GIVEN** deterministic accepted, pending, and rejected publish fixtures
- **WHEN** the publish verification lane runs
- **THEN** it invokes `scryrs publish markdown <PATH> --output <DIR>` through the real `scryrs` binary
- **AND** it verifies deterministic Markdown output for accepted decisions only
- **AND** it verifies pending and rejected artifacts do not publish

#### Scenario: Production verification exercises rspress publish through the CLI

- **GIVEN** deterministic accepted publish fixtures and a docs root fixture
- **WHEN** the publish verification lane runs
- **THEN** it invokes `scryrs publish rspress <PATH> --docs-root <DIR>` through the real `scryrs` binary
- **AND** it verifies `accepted-knowledge/` pages and `_nav.json` are updated
- **AND** it then runs the docs build and verifies the published proposal IDs and links appear in `doc_build/llms.txt` and `doc_build/llms-full.txt`

