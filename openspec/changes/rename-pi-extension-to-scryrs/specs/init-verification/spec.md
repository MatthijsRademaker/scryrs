## MODIFIED Requirements

### Requirement: Installed-hook e2e validates consumer-installed artifacts

The installed-hook e2e fixture SHALL run `scryrs init --agent claude-code` and `scryrs init --agent pi` in temporary consumer project directories, load the installed hook artifacts from their consumer install paths (NOT from `hooks/` in the repository source tree), exercise tool-capture forwarding against the real `scryrs` binary, and prove at least one event is persisted in `.scryrs/scryrs.db`.

#### Scenario: Claude Code installed hook is loadable and functional

- **GIVEN** a temporary consumer project directory
- **WHEN** `scryrs init --agent claude-code` is executed in that directory
- **THEN** the installed `scryrs-hook.mjs` file exists at `.claude/hooks/scryrs-hook.mjs`
- **AND** the installed file can be loaded as a Node.js module without import errors
- **AND** when exercised with a valid tool input, the hook forwards the event to `scryrs record --stdin`
- **AND** event persistence is confirmed via `scryrs hotspots .` showing `analyzedEventCount >= 1`

#### Scenario: Pi installed hook is loadable and functional

- **GIVEN** a temporary consumer project directory
- **WHEN** `scryrs init --agent pi` is executed in that directory
- **THEN** the installed Pi hook artifact exists at `.pi/extensions/scryrs/index.ts`
- **AND** the installed artifact can be loaded (TypeScript transpiled via tsx) without errors
- **AND** when exercised with a simulated `tool_result` event, the hook forwards the event to `scryrs record --stdin`
- **AND** event persistence is confirmed via `scryrs hotspots .` showing `analyzedEventCount >= 1`

#### Scenario: Installed hook e2e does not depend on repository source

- **GIVEN** the installed-hook e2e fixture is executing
- **WHEN** the fixture loads hook artifacts
- **THEN** artifacts are loaded from the temporary consumer project directory (`.claude/hooks/` or `.pi/extensions/scryrs/`)
- **AND** artifacts are NOT loaded from `hooks/claude-code/` or `hooks/pi/` in the repository source tree

#### Scenario: Failed init is detected by the e2e fixture

- **GIVEN** `scryrs init --agent claude-code` produces a corrupt or unloadable hook file
- **WHEN** the installed-hook e2e fixture attempts to load the installed artifact
- **THEN** the fixture fails with a diagnostic message indicating which artifact could not be loaded
- **AND** the fixture does not silently skip the load step
