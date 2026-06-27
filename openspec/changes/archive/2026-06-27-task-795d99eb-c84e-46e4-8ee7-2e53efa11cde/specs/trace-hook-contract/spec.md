## ADDED Requirements

### Requirement: Hook-owned remote configuration resolves from the target project root

When hook-triggered remote transport is active, the CLI-owned hook path SHALL resolve `scryrs.json` and warning-log paths from the target project's event `cwd` / base path rather than from the harness process cwd. Pi SHALL forward `process.cwd()` in every emitted raw event so `scryrs hook pi` can discover live configuration from the intended project root. Remote submission failures SHALL remain fail-open and SHALL NOT trigger local SQLite fallback or direct HTTP logic in the shim.

#### Scenario: Pi forwards cwd with each raw event

- **WHEN** the Pi shim forwards a `session_start` or `tool_result` event to `scryrs hook pi --file <tmp>`
- **THEN** the raw event includes a `cwd` field derived from `process.cwd()`
- **AND** the shim still contains no direct HTTP logic

#### Scenario: Hook remote config discovery uses the event base path

- **GIVEN** a hook event includes a `cwd` pointing at a project subdirectory with a `scryrs.json` in an ancestor directory
- **AND** the harness process cwd is different from that project root
- **WHEN** `scryrs hook <harness>` resolves remote configuration
- **THEN** it discovers the nearest ancestor `scryrs.json` from the event cwd/base path
- **AND** it does not rely on the harness process cwd for that resolution

#### Scenario: Remote submission failure stays fail-open

- **GIVEN** a live-configured project and an unreachable remote ingest server
- **WHEN** `scryrs hook pi` or `scryrs hook claude-code` processes an event
- **THEN** the command exits 0 with empty stdout
- **AND** a warning is recorded under `.scryrs/hooks/<harness>-warnings.log` rooted at the event base path
- **AND** no direct hook-side HTTP retry or local SQLite fallback occurs