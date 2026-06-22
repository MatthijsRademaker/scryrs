## ADDED Requirements

### Requirement: Trace-capture verification asserts Pi debug breadcrumbs

The trace-capture verification lane SHALL include coverage for the Pi hook and record endpoint with `SCRYRS_DEBUG` enabled. The verification SHALL assert presence of key debug breadcrumbs in addition to existing persistence assertions, without requiring exact full log ordering.

#### Scenario: Pi source-hook fixture observes debug stages

- **WHEN** `scripts/verification/pi-hook-e2e.mjs` exercises the source Pi hook with `SCRYRS_DEBUG` set
- **THEN** the fixture observes `[scryrs]` debug lines for hook load, tool result handling, record send, and record subprocess result
- **AND** the fixture observes `[scryrs-record]` debug lines echoed through the hook debug output for accepted record ingestion
- **AND** existing database persistence assertions still pass

#### Scenario: Untracked tool debug behavior is verified

- **WHEN** the Pi source-hook fixture invokes a simulated untracked tool result with `SCRYRS_DEBUG` set
- **THEN** the fixture observes a `[scryrs]` debug line with `tracked=false`
- **AND** the fixture verifies no TraceEvent row is persisted for that untracked tool

#### Scenario: Missing field debug behavior is verified

- **WHEN** the Pi source-hook fixture invokes a tracked tool result missing the expected mapped input field with `SCRYRS_DEBUG` set
- **THEN** the fixture observes a `[scryrs]` debug line identifying the missing field and available input keys
- **AND** existing fallback behavior still persists a TraceEvent with the fallback value

#### Scenario: Debug assertions avoid strict ordering

- **WHEN** the verification fixture checks debug output from session and tool events
- **THEN** it asserts required breadcrumb presence and relevant stable fields
- **AND** it does not require an exact total ordering between fire-and-forget `session_start` output and tool-result output

### Requirement: Installed Pi hook verification covers refreshed debug-capable artifact

The installed-hook e2e fixture SHALL validate that `scryrs init --agent pi` installs the debug-capable Pi hook from the canonical embedded source. The fixture SHALL load the installed artifact from the consumer `.pi/extensions/pi-trace/index.ts` path, enable `SCRYRS_DEBUG`, and assert debug breadcrumbs while preserving existing non-interference and persistence checks.

#### Scenario: Installed Pi hook emits debug breadcrumbs

- **WHEN** `scripts/verification/installed-hook-e2e.mjs` installs and loads the Pi hook with `SCRYRS_DEBUG` set
- **THEN** the installed artifact emits `[scryrs]` debug breadcrumbs during simulated tool-result handling
- **AND** it persists the expected TraceEvent through the real `scryrs record --stdin`
- **AND** the handler still returns `undefined`

#### Scenario: Installed artifact is not edited directly

- **WHEN** implementation refreshes `.pi/extensions/pi-trace/index.ts` for local dogfooding
- **THEN** the refresh is performed by removing the installed artifact and running `scryrs init --agent pi`
- **AND** the implementation does not directly patch `.pi/extensions/pi-trace/index.ts`
