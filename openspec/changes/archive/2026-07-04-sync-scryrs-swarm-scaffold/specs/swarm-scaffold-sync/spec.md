## ADDED Requirements

### Requirement: Checked-in Swarm/Pi scaffold SHALL include current default runtime files needed by active Swarm workflows
The checked-in `.pi/` tree in the `scryrs` source repository SHALL include the current default agent/prompt/skill/rule/readme files that active Swarm workflows and operators expect, unless `scryrs` intentionally replaces them with project-specific equivalents.

#### Scenario: Missing default planning prompts are restored
- **WHEN** the repository is synchronized with current Swarm defaults
- **THEN** `.pi/prompts/swarm-plan.md` exists
- **AND** `.pi/prompts/swarm-execute-plan.md` exists
- **AND** `.pi/prompts/swarm-execute-task.md` exists

#### Scenario: Current docs-ingestion skill is present
- **WHEN** the repository is synchronized with current Swarm defaults
- **THEN** `.pi/skills/read-project-docs/SKILL.md` exists
- **AND** the checked-in `.pi` tree does not rely solely on the retired `project-docs` default skill name

#### Scenario: Scaffold readme is present for maintainers
- **WHEN** a maintainer inspects the repository-local Pi scaffold
- **THEN** `.pi/README.md` exists
- **AND** it explains which checked-in `.pi` resources are runtime configuration versus scaffold/update sources

### Requirement: Swarm agent definitions SHALL use current default skill wiring unless `scryrs` intentionally overrides behavior
Checked-in Swarm agent definitions in `.pi/agents/*.md` SHALL use the current default skill names for shared Swarm capabilities, and SHALL remove stale default dependencies that are no longer part of the active Swarm contract.

#### Scenario: Docs skill wiring uses current name
- **WHEN** a maintainer inspects Swarm agent definitions in `.pi/agents/`
- **THEN** agents that consume repository documentation use `read-project-docs`
- **AND** they do not keep `project-docs` as the default skill name

#### Scenario: Retired semantic-search skill is not part of default wiring
- **WHEN** a maintainer inspects default Swarm agent skill lists in `.pi/agents/`
- **THEN** the skill lists do not require `ccc` as part of the default Swarm scaffold contract
- **AND** any retained semantic-search support is an explicit `scryrs` project decision rather than stale inherited default wiring

### Requirement: Scaffold sync SHALL preserve intentional `scryrs`-specific customizations
Synchronizing `scryrs` with current Swarm defaults SHALL preserve intentional repository-specific behavior and guardrails rather than flattening the repository into a generic scaffold clone.

#### Scenario: Model routing overrides are preserved
- **WHEN** `.pi/agents/*.md` already contains active runtime model override fields such as `modelEasy`, `modelModerate`, or `modelComplex`
- **THEN** synchronization keeps those fields unless the change explicitly replaces that routing policy

#### Scenario: Scryrs-specific UI and hook guidance is preserved
- **WHEN** the repository is synchronized
- **THEN** `scryrs`-specific guidance such as `shadcn-vue` usage and Pi trace-hook ownership rules remains present
- **AND** synchronization does not delete or rewrite those repository-specific rules as dead code

### Requirement: Scaffold sync SHALL NOT import foreign stack-specific verification contracts
Synchronizing the `scryrs` Swarm/Pi scaffold SHALL NOT copy stack-specific verification metadata or tooling contracts from unrelated repositories when those contracts do not match the `scryrs` Rust and dashboard verification model.

#### Scenario: Rust verification remains authoritative
- **WHEN** the repository is synchronized
- **THEN** `scripts/check`, `scripts/test`, `scripts/security`, and `scripts/precommit-run` remain the authoritative verification surface for `scryrs`
- **AND** the change does not replace them with Go-specific verification scripts or contracts from another repository

#### Scenario: Foreign verification metadata is not introduced blindly
- **WHEN** synchronization reviews newer Swarm defaults from another repository
- **THEN** it does not copy unrelated `.devagent/verification.json` semantics, changed-scope metadata, or stack-specific command inventories into `scryrs` without an explicit `scryrs` requirement for them
