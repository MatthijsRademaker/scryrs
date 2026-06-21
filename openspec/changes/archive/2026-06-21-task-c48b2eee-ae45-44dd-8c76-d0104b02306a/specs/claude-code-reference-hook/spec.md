# claude-code-reference-hook Specification

## ADDED Requirements

### Requirement: Bash rewrite-tool commands are recorded exactly as observed on PreToolUse

For Claude Code `Bash` PreToolUse events, the reference hook SHALL copy `tool_input.command` into `CommandExecuted.payload.command` exactly as observed when the scryrs hook runs. The hook SHALL NOT normalize, strip rewrite prefixes, split compound commands, or reconstruct original intent.

#### Scenario: RTK-prefixed Bash command is persisted as-is
- **WHEN** the Claude Code hook processes a Bash PreToolUse event whose `tool_input.command` is `rtk ls -la`
- **THEN** the emitted `CommandExecuted` event has `payload.command` equal to `rtk ls -la`
- **AND** the hook returns `{continue: true}` to Claude Code

#### Scenario: Compound rewritten Bash command remains a single observed command string
- **WHEN** the Claude Code hook processes a Bash PreToolUse event whose `tool_input.command` is `echo "=== BACKEND ===" && rtk ls backend/api/ && rtk ls backend/cmd/`
- **THEN** the emitted `CommandExecuted` event has `payload.command` equal to the full compound command string
- **AND** the hook does not split the command into multiple trace events

### Requirement: Companion README documents rewrite-tool ordering caveat

The `hooks/claude-code/README.md` SHALL document that `CommandExecuted.payload.command` reflects `tool_input.command` at the time the scryrs PreToolUse hook runs. The README SHALL state that co-installed rewrite hooks can change this observed value based on hook order and platform behavior, and it SHALL avoid guaranteeing preservation of the original agent-entered command.

#### Scenario: README explains hook-order dependency
- **WHEN** a Claude Code integrator reads the companion README
- **THEN** it states that scryrs records the command string visible to the scryrs hook in the PreToolUse pipeline
- **AND** it states that running scryrs before or after a rewrite hook can change the recorded command value
- **AND** it states that scryrs does not perform rewrites or preserve both original and rewritten commands under the current schema
