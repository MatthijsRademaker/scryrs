## MODIFIED Requirements

### Requirement: Claude Code fixture proves success capture against real scryrs

The Claude Code verification fixture SHALL drive the native `scryrs hook claude-code` subcommand by piping a real PreToolUse payload on stdin, and SHALL assert the event is persisted to the trace store under the payload `cwd`. It SHALL NOT invoke any `.mjs` file or `node`.

#### Scenario: native command captures a tracked tool

- **GIVEN** a PreToolUse payload for a tracked tool on stdin
- **WHEN** `scryrs hook claude-code` runs against a real scryrs binary
- **THEN** exit code is 0 with empty stdout
- **AND** the corresponding `TraceEvent` is present in the store

### Requirement: Both fixtures prove fail-open behavior

Both harness fixtures SHALL prove fail-open against the native command path. For Claude Code, malformed stdin and an unwritable store SHALL each exit 0 and append to `.scryrs/hooks/claude-code-warnings.log`. For Pi, a failing `scryrs hook pi --file` invocation SHALL not break the extension.

#### Scenario: Claude Code malformed input fails open

- **GIVEN** non-JSON bytes piped to `scryrs hook claude-code`
- **THEN** exit code is 0
- **AND** a warning line is appended to the claude-code warning log

#### Scenario: Pi delegation failure fails open

- **GIVEN** `scryrs hook pi --file` returns non-zero
- **WHEN** the Pi shim processes a `tool_result`
- **THEN** the agent-visible tool result is unchanged

### Requirement: Verification does not modify hook sources, CLI behavior, or schema

Verification SHALL exercise the shipped `scryrs hook` subcommand and the slimmed Pi shim without modifying them. Retired `.mjs`-based fixtures SHALL be removed.

#### Scenario: no .mjs fixtures remain

- **WHEN** inspecting the cross-harness verification fixtures
- **THEN** none reference `scryrs-hook.mjs` or `node`
