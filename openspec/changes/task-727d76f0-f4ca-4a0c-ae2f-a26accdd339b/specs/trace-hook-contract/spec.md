## MODIFIED Requirements

### Requirement: Non-interference and fail-open rules are stated unambiguously

The hook contract SHALL state unambiguously that scryrs remains trace-collection only and that Pi and Claude integrations remain transport-dumb. Hook shims and harness configuration SHALL keep delegating to the scryrs CLI and SHALL NOT embed direct HTTP fetch logic, remote-ingest request construction, or server-response handling. When remote mode is configured, any network submission performed on the hook-invoked CLI path SHALL remain inside Rust CLI code. Hook fail-open behavior SHALL be preserved: if that CLI path fails, the harness tool execution proceeds unmodified and the failure is surfaced only through hook warning diagnostics rather than by blocking the tool.

#### Scenario: Hook integrations do not gain HTTP logic

- **WHEN** a maintainer updates Pi or Claude integration for remote ingest support
- **THEN** the hook shim or harness configuration still delegates to the scryrs CLI
- **AND** no direct server HTTP client logic is added to the hook integration itself

#### Scenario: Remote failure on the hook path does not block the harness

- **WHEN** the CLI path invoked by a hook encounters a remote ingest failure
- **THEN** the original harness tool execution still proceeds unmodified
- **AND** the failure is surfaced through the hook's warning/fail-open diagnostics rather than by changing the tool result

### Requirement: scryrs record invocation is documented without alternate ingestion paths

The hook contract SHALL document that remote ingest, when configured, is a transport decision inside the CLI ingestion path rather than a new harness-facing protocol. Integrators SHALL continue to invoke scryrs through the documented CLI entrypoints, and the contract SHALL reference the CLI record contract for local and remote summary behavior. The contract SHALL NOT instruct Pi or Claude integrators to post trace events directly to the server from JavaScript or hook configuration.

#### Scenario: Integrator keeps using the CLI in remote mode

- **WHEN** an integrator reads the hook contract for remote ingest guidance
- **THEN** the contract directs them to the scryrs CLI entrypoints rather than a new direct HTTP protocol
- **AND** remote transport is described as CLI-owned behavior

#### Scenario: No direct server instructions appear in hook docs

- **WHEN** an integrator reads the hook contract
- **THEN** they do not find instructions to post trace events directly from Pi or Claude hook code
- **AND** they do not find a new socket or alternate IPC ingestion path
