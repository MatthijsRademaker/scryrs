# trace-hook-contract Specification

## ADDED Requirements

### Requirement: Rewrite-tool compatibility policy defines observed-command semantics

The canonical hook contract SHALL state that `CommandExecuted.payload.command` records the command string observed by the hook at capture time. The contract SHALL explicitly state that scryrs does not invoke rewrite tools, does not normalize or canonicalize commands in hooks, and does not reconstruct original agent intent from rewritten Bash input.

#### Scenario: Upstream rewrite tool has already changed a Bash command
- **GIVEN** a co-installed rewrite extension has transformed a Bash command before scryrs observes it
- **WHEN** the hook emits a `CommandExecuted` event
- **THEN** `payload.command` contains the command string presented to the hook at that point in the harness pipeline
- **AND** scryrs does not strip prefixes such as `rtk`
- **AND** scryrs does not attempt to recover the pre-rewrite command text

#### Scenario: Compatibility guidance preserves non-interference
- **WHEN** an integrator reads the rewrite-tool compatibility guidance
- **THEN** the document states that scryrs remains trace-collection only
- **AND** the document states that scryrs never rewrites tool stdout, stderr, exit status, or execution semantics
- **AND** the document does not instruct integrators to call `rtk rewrite` from scryrs

### Requirement: Hook-order and harness differences are documented conservatively

The hook contract SHALL describe rewrite-tool co-installation semantics for both supported harnesses without overclaiming unverified behavior. It SHALL document that Pi captures Bash commands from `tool_result`, while Claude Code captures Bash commands from PreToolUse, and it SHALL state that Claude Code capture beside rewrite hooks is order-dependent. If Pi mutation propagation or Claude Code updated-input forwarding is not empirically verified, the contract SHALL present those points as limitations rather than guarantees.

#### Scenario: Pi guidance explains post-execution capture point
- **WHEN** an integrator reads the Pi-specific rewrite guidance
- **THEN** the document states that the Pi reference hook reads `event.input.command` from `tool_result`
- **AND** the document explains that scryrs records whatever command string is present on that post-execution event
- **AND** the document avoids claiming more specific pre-rewrite or post-rewrite semantics unless they are verified

#### Scenario: Claude Code guidance explains hook-order caveat
- **WHEN** an integrator reads the Claude Code-specific rewrite guidance
- **THEN** the document states that the Claude Code reference hook reads `tool_input.command` during PreToolUse
- **AND** the document states that co-installed rewrite hooks may change what scryrs observes depending on hook order and platform forwarding behavior
- **AND** the document does not guarantee preservation of both original and rewritten commands under the current single-string schema
