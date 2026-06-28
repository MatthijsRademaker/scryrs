## MODIFIED Requirements

### Requirement: Source-repo Pi install remains non-canonical runtime state
When `scryrs init --agent pi` is used inside the scryrs source checkout, `hooks/pi/index.ts` SHALL remain the only canonical hook source in the repository. The installed file at `.pi/extensions/pi-trace/index.ts` SHALL be treated as runtime copy only. Repository maintainer guidance and checked-in Swarm/Pi scaffold guidance SHALL stay aligned on that ownership model so scaffold-sync work does not treat the installed runtime copy as leading source.

#### Scenario: AGENTS guidance defines canonical Pi hook source
- **WHEN** a maintainer or agent reads `AGENTS.md`
- **THEN** the file states that `hooks/pi/index.ts` is canonical source for the Pi hook
- **AND** the file states that `.pi/extensions/pi-trace/index.ts` is installed runtime copy only
- **AND** the file states that LLMs/agents MUST NOT edit the installed copy directly

#### Scenario: Installed Pi copy is excluded from normal git noise
- **WHEN** `scryrs init --agent pi` is run inside the scryrs source checkout
- **THEN** the created `.pi/extensions/pi-trace/` artifact path is ignored by repository ignore rules
- **AND** local dogfooding does not require committing installed Pi hook copy

#### Scenario: Claude Code consumer config remains blocked in source repo
- **GIVEN** CWD is the scryrs source checkout
- **WHEN** `scryrs init --agent claude-code` is invoked
- **THEN** the exit code is 2
- **AND** no `.claude/` consumer config files are created or modified inside the scryrs source checkout

#### Scenario: Scaffold sync does not elevate installed Pi runtime copy
- **WHEN** repository-local Swarm/Pi scaffold files under `.pi/` are synchronized with newer defaults
- **THEN** the synchronization does not treat `.pi/extensions/pi-trace/index.ts` as canonical hook source
- **AND** no maintainer guidance instructs agents to edit the installed runtime copy instead of `hooks/pi/index.ts`
