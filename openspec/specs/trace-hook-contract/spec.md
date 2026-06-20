# trace-hook-contract Specification

## Purpose
TBD - created by archiving change task-eab8b93c-f563-4925-bd88-48bf90e5fd6c. Update Purpose after archive.
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

The hook contract SHALL state unambiguously that scryrs is trace-collection only. It SHALL document that scryrs never rewrites tool stdout, stderr, exit status, or semantics; scryrs does not proxy business-tool execution; hooks contain no business logic beyond formatting plus subprocess delegation; and scryrs is never registered as an agent-callable business tool or MCP/tool catalog surface. The contract SHALL include fail-open guidance: if hook invocation of `scryrs record` fails for any reason, the harness tool execution SHALL proceed normally without scryrs interference.

#### Scenario: Integrator reads non-interference rule

- **GIVEN** scryrs is trace collection only
- **WHEN** an integrator reads the contract
- **THEN** they see that scryrs must never rewrite tool output, exit status, or semantics
- **AND** scryrs is never registered as an agent-callable business tool

#### Scenario: Hook failure does not block tool execution

- **WHEN** a hook's invocation of `scryrs record` fails (process crash, pipe error, or non-zero exit)
- **THEN** the harness SHALL proceed with the original tool execution normally
- **AND** the original tool's stdout, stderr, and exit status are preserved unmodified

#### Scenario: scryrs.json is not a tool catalog

- **WHEN** an integrator reads the `scryrs.json` manifest documentation
- **THEN** the document explicitly states that `scryrs.json` is a hook-interface and record-invocation manifest only
- **AND** the document states it is not a tool catalog, MCP descriptor, or business-tool surface

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

The hook contract SHALL document `scryrs record --stdin` and `scryrs record --file <PATH>` as the only supported ingestion modes. It SHALL reference `.devagent/docs/docs/cli-v0-contract.md` for the deterministic output shape, exit codes (0/1/2), and rejection diagnostics. The contract SHALL NOT invent any alternate ingestion path, wrapper command, or IPC surface.

#### Scenario: Hook pipes events via stdin

- **WHEN** a hook invokes `scryrs record --stdin` and writes newline-delimited TraceEvent JSON to its stdin
- **THEN** scryrs validates, persists, and reports acceptance/rejection as documented in the CLI v0 contract
- **AND** the hook contract references the CLI v0 contract for the full output specification

#### Scenario: Record reads events from file

- **WHEN** a user or hook invokes `scryrs record --file session.jsonl`
- **THEN** scryrs reads the file using the same ingestion path as stdin mode
- **AND** the mode is documented as mutually exclusive with `--stdin`

#### Scenario: No alternate ingestion paths exist

- **WHEN** an integrator reads the contract
- **THEN** the only documented ingestion modes are `--stdin` and `--file <PATH>`
- **AND** no pipe wrapper, socket, HTTP endpoint, or inter-process mechanism is documented

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

