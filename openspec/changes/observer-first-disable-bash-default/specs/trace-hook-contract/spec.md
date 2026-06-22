## MODIFIED Requirements

### Requirement: Non-interference and fail-open rules are stated with observer-first default scope

The hook contract SHALL state unambiguously that scryrs is trace-collection only and observer-first. It SHALL document that default capture prioritizes stable harness-native tools and excludes Bash unless `SCRYRS_DEBUG` is set to a non-empty value. The contract SHALL continue to state that scryrs never rewrites tool stdout, stderr, exit status, or semantics; scryrs does not proxy business-tool execution; hooks contain no business logic beyond formatting plus subprocess delegation; and scryrs is never registered as an agent-callable business tool or MCP/tool catalog surface.

#### Scenario: Integrator reads observer-first default boundary

- **WHEN** an integrator reads the contract
- **THEN** they see that Bash is not part of default trace capture
- **AND** they see that `SCRYRS_DEBUG` is the explicit opt-in switch for diagnostic Bash capture

### Requirement: Rewrite-tool compatibility policy defines debug-gated observed-command semantics

The canonical hook contract SHALL state that `CommandExecuted.payload.command` records the command string observed by the hook at capture time only in sessions where Bash capture is enabled through `SCRYRS_DEBUG`. The contract SHALL explicitly state that scryrs does not invoke rewrite tools, does not normalize or canonicalize commands in hooks, and does not reconstruct original agent intent from rewritten Bash input.

#### Scenario: Upstream rewrite tool has already changed a debug-gated Bash command

- **GIVEN** a co-installed rewrite extension has transformed a Bash command before scryrs observes it and `SCRYRS_DEBUG` is set
- **WHEN** the hook emits a `CommandExecuted` event
- **THEN** `payload.command` contains the command string presented to the hook at that point in the harness pipeline
- **AND** scryrs does not strip prefixes such as `rtk`
- **AND** scryrs does not attempt to recover the pre-rewrite command text

#### Scenario: No observed-command semantics are promised outside debug mode

- **WHEN** an integrator reads the compatibility guidance
- **THEN** the document states that no Bash trace event is emitted when `SCRYRS_DEBUG` is unset
- **AND** the document does not present Bash command capture as default product behavior

### Requirement: Hook-order and harness differences are documented conservatively for debug-gated Bash capture

The hook contract SHALL describe rewrite-tool co-installation semantics for both supported harnesses without overclaiming unverified behavior. It SHALL document that Pi captures debug-gated Bash commands from `tool_result`, while Claude Code captures debug-gated Bash commands from PreToolUse, and it SHALL state that Claude Code capture beside rewrite hooks is order-dependent. If Pi mutation propagation or Claude Code updated-input forwarding is not empirically verified, the contract SHALL present those points as limitations rather than guarantees.

#### Scenario: Pi guidance explains post-execution capture point only for debug-enabled Bash

- **WHEN** an integrator reads the Pi-specific rewrite guidance
- **THEN** the document states that the Pi reference hook reads `event.input.command` from `tool_result` only when Bash capture is enabled via `SCRYRS_DEBUG`
- **AND** the document explains that scryrs records whatever command string is present on that post-execution event

#### Scenario: Claude Code guidance explains hook-order caveat only for debug-enabled Bash

- **WHEN** an integrator reads the Claude Code-specific rewrite guidance
- **THEN** the document states that the Claude Code reference hook reads `tool_input.command` during PreToolUse only when Bash capture is enabled via `SCRYRS_DEBUG`
- **AND** the document states that co-installed rewrite hooks may change what scryrs observes depending on hook order and platform forwarding behavior
