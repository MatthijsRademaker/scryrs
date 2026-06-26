# trace-hook-contract Specification

## Purpose

Defines requirements for the trace hook contract document — the canonical harness integration reference covering non-interference rules, TraceEvent schema mapping, session demarcation, scryrs.json manifest shape, and integration tiers.
## Requirements
### Requirement: Canonical hook-contract documentation exists and is discoverable

The system SHALL publish a single canonical hook-contract document at `.devagent/docs/docs/trace-hook-contract.md` that serves as the source of truth for harness integrators. The document SHALL be registered in `.devagent/docs/docs/_nav.json` under the Technical section and SHALL clearly identify itself as the canonical integration contract.

#### Scenario: Integrator discovers the hook contract

- **GIVEN** a harness integrator wants to add scryrs trace capture support
- **WHEN** they navigate the project documentation
- **THEN** they find `trace-hook-contract.md` listed under Technical in the docs navigation
- **AND** the document identifies itself as the single source of truth for harness integration

#### Scenario: Contract is self-contained

- **WHEN** an integrator reads the hook contract document
- **THEN** they can understand what to capture, how to invoke `scryrs record`, and what boundaries must not be crossed
- **AND** they do not need to read refinement-room dossiers or task comments to implement integration

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

### Requirement: TraceEvent schema is referenced and event families are documented

The hook contract SHALL reference `crates/scryrs-types/src/lib.rs` as the canonical TraceEvent schema source. It SHALL document all required event families — SessionStart, SessionEnd, FileOpened, SearchRun, SymbolInspected, CommandExecuted, DocRetrieved, EditMade, FailedLookup — and the required envelope fields: `schema_version`, `timestamp`, `session_id`, `event_type`, `tool_name`, `payload`, and `outcome`. The contract SHALL NOT redefine the schema from scratch.

#### Scenario: Integrator maps harness events to TraceEvent

- **WHEN** an integrator needs to map their harness's tool events to scryrs trace events
- **THEN** the contract lists the nine event families with their required payload fields
- **AND** the contract references `crates/scryrs-types/src/lib.rs` for the exact Rust type definitions

#### Scenario: Envelope fields are documented

- **WHEN** an integrator constructs a TraceEvent
- **THEN** the contract documents that every event must carry `schema_version` (the current `SCHEMA_VERSION`), `timestamp` (RFC3339 string), `session_id` (unique session identifier), `event_type`, `tool_name` (required for subject-bearing events, optional for lifecycle events), `payload` (self-describing JSON with `type` tag), and `outcome` (Success or Failure with optional reason)

### Requirement: Session demarcation uses first-class SessionStart and SessionEnd events

The hook contract SHALL document that session boundaries are represented as first-class `SessionStart` and `SessionEnd` events from the shared schema. Every session SHALL be bounded by explicit lifecycle events with a unique `session_id` spanning all events in the session. The contract SHALL NOT define implicit session boundaries or heuristics.

#### Scenario: Session starts with explicit event

- **WHEN** a trace session begins
- **THEN** the harness hook emits a `SessionStart` event with a new unique `session_id`
- **AND** the event may omit `tool_name` since it is a lifecycle event with no hotspot subject

#### Scenario: Session ends with explicit event

- **WHEN** a trace session completes
- **THEN** the harness hook emits a `SessionEnd` event with the same `session_id`
- **AND** downstream consumers can detect session completion from the event stream

#### Scenario: All session events share one session_id

- **WHEN** a harness produces trace events throughout a session
- **THEN** every event in that session carries the same `session_id` value
- **AND** the `session_id` remains stable from `SessionStart` through `SessionEnd`

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

### Requirement: scryrs.json manifest shape is documented with explicit scope boundary

The hook contract SHALL document the `scryrs.json` manifest purpose, intended location (repository root), and an example minimal JSON shape. The documentation SHALL explicitly state that `scryrs.json` is a hook-interface and record-invocation manifest only, is not a tool catalog, MCP descriptor, or business-tool surface, and that its shape is provisional v0.1 subject to change before Phase 1 stabilization. No `scryrs.json` file is created in this task.

#### Scenario: Manifest shape is documented

- **WHEN** an integrator reads the `scryrs.json` documentation
- **THEN** they see the manifest's purpose (describe hook interface and `scryrs record` invocation)
- **AND** they see the intended location (repository root)
- **AND** they see an example minimal JSON skeleton

#### Scenario: Manifest is explicitly not a tool catalog

- **WHEN** an integrator reads the manifest documentation
- **THEN** the document states explicitly that `scryrs.json` is not a tool catalog, MCP descriptor, or business-tool surface
- **AND** the document includes an anti-pattern warning against interpreting the manifest as describing callable tools

#### Scenario: Manifest shape is provisional

- **WHEN** an integrator reads the manifest documentation
- **THEN** the document states that the `scryrs.json` shape is provisional v0.1
- **AND** the document notes that field names, file location, and schema may change before Phase 1 stabilization

### Requirement: Integration-tier matrix defines full hook, plugin, and rules-file fallback

The hook contract SHALL define three integration tiers — full hook, plugin, and rules-file fallback — in a matrix format. Each tier SHALL list supported or planned harness coverage and explicit limitations. The matrix SHALL only name harnesses with confirmed extension-point evidence (Pi and Claude Code); all others SHALL be marked TBD.

#### Scenario: Full hook tier is defined

- **WHEN** an integrator reads the integration tier matrix
- **THEN** the full hook tier is defined as harness-native subprocess hook support (e.g., Pi `.pi/hooks/`, Claude Code hook system)
- **AND** full hook provides automatic event coverage and session demarcation
- **AND** Pi and Claude Code are listed as planned for full hook support

#### Scenario: Plugin tier is defined

- **WHEN** an integrator reads the integration tier matrix
- **THEN** the plugin tier is defined as harness-specific plugin/extension API
- **AND** plugin tier requires plugin auth/development per harness
- **AND** coverage depends on the specific plugin API capabilities

#### Scenario: Rules-file fallback tier is defined with limitations

- **WHEN** an integrator reads the integration tier matrix
- **THEN** the rules-file fallback tier is defined as manual event-rule authoring
- **AND** the tier explicitly states that it cannot guarantee automatic session demarcation
- **AND** the tier explicitly states that it requires manual rule authoring by the user
- **AND** the tier explicitly states that event coverage is inherently partial
- **AND** the tier states it cannot intercept tool events without harness cooperation

#### Scenario: Only confirmed harnesses are named

- **WHEN** an integrator reads the harness coverage column
- **THEN** only Pi and Claude Code are listed as planned harnesses
- **AND** no unconfirmed harnesses (e.g., Cursor, Windsurf, aider) are named as supported

### Requirement: Reference hook examples link to forthcoming Phase 1 deliverables

The hook contract SHALL include reference links for Pi and Claude Code hook work. Since reference hooks do not exist yet, the doc SHALL explicitly mark them as forthcoming Phase 1 deliverables and SHALL link to the roadmap Phase 1 section rather than implying existing implementations.

#### Scenario: Pi reference hook is marked forthcoming

- **WHEN** an integrator reads the reference examples section
- **THEN** the Pi hook is described as a forthcoming Phase 1 deliverable
- **AND** the document references the roadmap Phase 1 section

#### Scenario: Claude Code reference hook is marked forthcoming

- **WHEN** an integrator reads the reference examples section
- **THEN** the Claude Code hook is described as a forthcoming Phase 1 deliverable
- **AND** the document references the roadmap Phase 1 section

### Requirement: Roadmap.mdx no longer contradicts current product state

The `.devagent/docs/docs/roadmap.mdx` "Current Starting Point" section SHALL be updated to remove the stale claim that `record` does not exist. It SHALL also update the claim that the CLI only exposes placeholder `hotspots` behavior to reflect that `scryrs record` exists for JSONL trace event ingestion.

#### Scenario: Roadmap reflects record existence

- **WHEN** a reader reviews the roadmap "Current Starting Point" section
- **THEN** it no longer states "No `record` command, event store, reference hooks, installer, or route manifests exist yet"
- **AND** it reflects that `scryrs record` exists for trace event ingestion

#### Scenario: Roadmap does not contradict hook contract

- **WHEN** a reader compares the roadmap and the new hook contract document
- **THEN** both agree that `scryrs record` exists and is the ingestion endpoint for trace events

### Requirement: Scope is limited to documentation and roadmap correction

This change SHALL NOT expand beyond project documentation and the roadmap.mdx fix. It SHALL NOT modify any Rust crate, CLI behavior, wire format, or existing OpenSpec capability specs for `scryrs-record-endpoint` or `trace-event-schema`.

#### Scenario: No code changes are made

- **WHEN** this change is implemented
- **THEN** no Rust source files in `crates/` are modified
- **AND** no new Rust crates are added

#### Scenario: No checked-in manifest is created

- **WHEN** this change is implemented
- **THEN** no `scryrs.json` file is created at the repository root or elsewhere
- **AND** no `hooks/` directory is created

#### Scenario: No existing specs are modified

- **WHEN** this change is implemented
- **THEN** the `openspec/specs/scryrs-record-endpoint/spec.md` file is unchanged
- **AND** the `openspec/specs/trace-event-schema/spec.md` file is unchanged

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

