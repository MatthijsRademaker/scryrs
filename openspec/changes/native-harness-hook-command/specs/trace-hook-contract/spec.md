## ADDED Requirements

### Requirement: Contract documents per-harness transport models

The hook contract SHALL document that harness integration transport differs by harness and that this is intentional. Claude Code SHALL be integrated via the native `scryrs hook claude-code` subprocess hook (no JavaScript, no node). Pi SHALL be integrated via a thin in-process extension (`hooks/pi/index.ts`) that delegates translation to `scryrs hook pi`, because Pi's runtime loads a module rather than spawning a subprocess hook. The contract SHALL state that translation lives once, in `scryrs-adapter-harness`.

#### Scenario: Integrator sees the two transport models

- **WHEN** an integrator reads the transport section of the contract
- **THEN** it states Claude Code uses the native `scryrs hook claude-code` command
- **AND** it states Pi uses an in-process shim delegating to `scryrs hook pi`
- **AND** it states tool→event translation is owned by the `scryrs-adapter-harness` crate

## MODIFIED Requirements

### Requirement: Non-interference and fail-open rules are stated unambiguously

The hook contract SHALL state that scryrs is trace-collection only: it never rewrites tool stdout, stderr, exit status, or semantics; it does not proxy business-tool execution; it is never registered as an agent-callable business tool or MCP/tool-catalog surface. The contract SHALL document fail-open: harness tool execution SHALL always proceed regardless of scryrs. For the native `scryrs hook <harness>` command, fail-open means the command SHALL always exit 0 with empty stdout and log any error to `.scryrs/hooks/<harness>-warnings.log`. For the Pi shim, a failed `scryrs hook pi` invocation SHALL not alter the agent-visible tool result.

#### Scenario: native hook command never blocks the tool

- **WHEN** `scryrs hook claude-code` encounters any internal error
- **THEN** it exits 0 with empty stdout
- **AND** Claude Code proceeds with the original tool unmodified
- **AND** a warning is appended to `.scryrs/hooks/claude-code-warnings.log`

#### Scenario: scryrs is never an agent-callable surface

- **WHEN** an integrator reads the contract
- **THEN** scryrs is documented as trace-collection only, never a callable business tool

### Requirement: scryrs record invocation is documented without alternate ingestion paths

The hook contract SHALL document two layers: (1) the harness-facing `scryrs hook <harness>` command, which accepts a harness's native event (Claude Code on stdin; Pi via `--file`), translates it, and persists it; and (2) the low-level `scryrs record --stdin` / `scryrs record --file <PATH>` canonical-JSONL ingestion primitive. The contract SHALL reference `.devagent/docs/docs/cli-v0-contract.md` for record's output shape and exit codes. It SHALL NOT invent any other ingestion path, wrapper command, socket, or HTTP/IPC surface beyond `scryrs hook` and `scryrs record`.

#### Scenario: Harness integration goes through scryrs hook

- **WHEN** an integrator wires a harness to scryrs
- **THEN** the contract directs them to `scryrs hook <harness>` as the harness transport
- **AND** documents `scryrs record` as the underlying canonical ingestion primitive

#### Scenario: No alternate ingestion paths exist

- **WHEN** an integrator reads the contract
- **THEN** the only documented ingestion surfaces are `scryrs hook <harness>` and `scryrs record` (`--stdin`/`--file`)
- **AND** no pipe wrapper, socket, HTTP endpoint, or other IPC mechanism is documented
