# pi-reference-hook Specification

## ADDED Requirements

### Requirement: Bash rewrite-tool commands are recorded exactly as observed on tool_result

For Pi `bash` tool results, the reference hook SHALL copy `event.input.command` into `CommandExecuted.payload.command` exactly as observed at the `tool_result` boundary. The hook SHALL NOT normalize, strip rewrite prefixes, split compound commands, or reconstruct original intent.

#### Scenario: RTK-prefixed Bash command is persisted as-is
- **WHEN** the Pi hook processes a `tool_result` event for `bash` whose `event.input.command` is `rtk ls -la`
- **THEN** the emitted `CommandExecuted` event has `payload.command` equal to `rtk ls -la`
- **AND** the handler returns `undefined`

#### Scenario: Compound rewritten Bash command remains a single observed command string
- **WHEN** the Pi hook processes a `tool_result` event for `bash` whose `event.input.command` is `echo "=== BACKEND ===" && rtk ls backend/api/ && rtk ls backend/cmd/`
- **THEN** the emitted `CommandExecuted` event has `payload.command` equal to the full compound command string
- **AND** the hook does not split the command into multiple trace events

### Requirement: Companion README documents rewrite-tool compatibility limits

The `hooks/pi/README.md` SHALL document that Bash command capture occurs from `tool_result` and that `CommandExecuted.payload.command` reflects the command string observed there. The README SHALL state that scryrs does not perform rewrites itself and SHALL present upstream rewrite propagation behavior as a verified fact only if it has been empirically confirmed.

#### Scenario: README explains observed-command semantics
- **WHEN** a Pi integrator reads the companion README
- **THEN** it states that scryrs records the `event.input.command` value seen on `tool_result`
- **AND** it states that rewrite prefixes such as `rtk` are persisted as-is
- **AND** it does not claim that the current schema preserves original agent intent
