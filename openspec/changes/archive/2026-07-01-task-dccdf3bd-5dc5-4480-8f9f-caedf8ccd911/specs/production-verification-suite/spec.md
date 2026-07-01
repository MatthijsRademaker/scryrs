## ADDED Requirements

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