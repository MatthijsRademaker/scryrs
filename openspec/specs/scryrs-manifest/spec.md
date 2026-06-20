# scryrs-manifest Specification

## Purpose
TBD - created by archiving change task-e32b450a-0631-41a7-a3fb-939b14cc73f0. Update Purpose after archive.
## Requirements
### Requirement: Manifest file exists at repository root

The system SHALL provide a valid JSON file named `scryrs.json` at the repository root. The file SHALL be parseable as JSON and SHALL contain top-level fields for manifest versioning, scope declaration, record invocation, and harness enumeration.

#### Scenario: Manifest is discoverable at repo root

- **WHEN** a harness integrator or tool inspects the repository root
- **THEN** a valid JSON file named `scryrs.json` exists alongside `Cargo.toml` and other root markers
- **AND** the file parses as valid JSON without errors

#### Scenario: Manifest is machine-readable

- **WHEN** a script or installer reads `scryrs.json`
- **THEN** it can parse the file with a standard JSON parser
- **AND** all top-level fields are present with the expected types

### Requirement: Manifest declares its own schema version and the canonical trace schema version

The manifest SHALL include a `manifest_version` field (the version of the manifest schema itself) and a `trace_schema_version` field (the version of the trace event schema). The `trace_schema_version` SHALL match the value of `scryrs_types::SCHEMA_VERSION` defined in `crates/scryrs-types/src/lib.rs` (currently `"0.1.0"`).

#### Scenario: Manifest version is declared

- **WHEN** a consumer reads `scryrs.json`
- **THEN** the `manifest_version` field is present and is a semantic version string
- **AND** the `trace_schema_version` field is present and matches `"0.1.0"`

#### Scenario: Trace schema version matches canonical source

- **GIVEN** `crates/scryrs-types/src/lib.rs` defines `SCHEMA_VERSION = "0.1.0"`
- **WHEN** `scryrs.json` is inspected
- **THEN** the `trace_schema_version` field equals `"0.1.0"`

### Requirement: Manifest declares explicit scope and anti-pattern boundaries

The manifest SHALL include a `scope` field declaring its purpose as a hook-interface manifest. It SHALL include an `_anti_patterns` section (or equivalent boundary declaration) explicitly stating that the manifest is not an MCP descriptor, tool catalog, agent-callable tool surface, or tool-output rewriter.

#### Scenario: Scope is declared

- **WHEN** a developer inspects `scryrs.json`
- **THEN** the `scope` field is present with a value describing it as a hook-interface and record-invocation manifest
- **AND** the `scope` field states the manifest defines hook interfaces and record invocation only

#### Scenario: Anti-pattern boundaries are explicit

- **WHEN** a developer inspects `scryrs.json`
- **THEN** the manifest contains an explicit declaration that it is NOT an MCP descriptor
- **AND** the manifest states it is NOT a tool catalog or agent-callable surface
- **AND** the manifest states it does NOT describe tool-output-rewriting behavior

#### Scenario: Manifest contains no MCP or tool-registration fields

- **WHEN** `scryrs.json` is parsed
- **THEN** it contains no MCP method entries, no `tools` array with schemas, and no callable surface descriptions
- **AND** no field implies scryrs can be called as an agent tool

### Requirement: Manifest defines the `scryrs record` invocation contract

The manifest SHALL include a `record` object describing how hooks invoke `scryrs record`. The `record` object SHALL specify the command (`"scryrs"`) and the canonical args (`["record", "--stdin"]`). The manifest SHALL document `--file <PATH>` as an available alternate ingestion mode.

#### Scenario: Record invocation contract is defined

- **WHEN** a hook author reads the `record` section of the manifest
- **THEN** they see `command` set to `"scryrs"` and `args` set to `["record", "--stdin"]`
- **AND** the section describes `--stdin` as the canonical hook ingestion path

#### Scenario: File mode is documented as alternate

- **WHEN** a hook author reads the `record` section
- **THEN** `--file <PATH>` is documented as an available alternate ingestion mode
- **AND** the documentation notes that hooks currently use `--stdin`

### Requirement: Manifest enumerates supported harnesses with per-harness metadata

The manifest SHALL use a `supported_harnesses` array containing one entry per supported harness. The harnesses SHALL be `claude-code` and `pi`. Each entry SHALL include `referenceSource` (repository path to the hook source file), `interceptedTools` (harness-specific tool names the hook observes), and `capturedEventFamilies` (canonical `TraceEventType` families the hook produces).

#### Scenario: Supported harnesses are enumerated

- **WHEN** a consumer reads `supported_harnesses`
- **THEN** the array contains exactly two entries for `claude-code` and `pi`
- **AND** entries are ordered deterministically

#### Scenario: Claude Code entry has correct reference source

- **WHEN** the `claude-code` entry is inspected
- **THEN** `referenceSource` equals `"hooks/claude-code/scryrs-hook.mjs"`
- **AND** the path matches the `include_str!()` path in `crates/scryrs-cli/src/init.rs`

#### Scenario: Pi entry has correct reference source

- **WHEN** the `pi` entry is inspected
- **THEN** `referenceSource` equals `"hooks/pi/index.ts"`
- **AND** the path matches the `include_str!()` path in `crates/scryrs-cli/src/init.rs`

#### Scenario: Claude Code entry lists intercepted tools

- **WHEN** the `claude-code` entry is inspected
- **THEN** `interceptedTools` contains exactly: `read`, `bash`, `grep`, `glob`, `edit`, `write`, `notebookedit`, `web_search`, `web_fetch`
- **AND** the field clearly describes what the hook observes, not a callable surface

#### Scenario: Pi entry lists intercepted tools

- **WHEN** the `pi` entry is inspected
- **THEN** `interceptedTools` contains exactly: `read`, `bash`, `ast_grep_search`, `lsp_navigation`, `edit`, `write`
- **AND** the field clearly describes what the hook observes, not a callable surface

#### Scenario: Claude Code entry lists captured event families

- **WHEN** the `claude-code` entry is inspected
- **THEN** `capturedEventFamilies` contains exactly: `FileOpened`, `CommandExecuted`, `SearchRun`, `EditMade`, `DocRetrieved`
- **AND** the families match the canonical `TraceEventType` variants in `scryrs-types`

#### Scenario: Pi entry lists captured event families

- **WHEN** the `pi` entry is inspected
- **THEN** `capturedEventFamilies` contains exactly: `FileOpened`, `CommandExecuted`, `SearchRun`, `EditMade`, `SymbolInspected`, `FailedLookup`
- **AND** the families match the canonical `TraceEventType` variants in `scryrs-types`

### Requirement: Per-harness lifecycle capabilities are accurately represented

Each harness entry SHALL include a `lifecycle` field that accurately describes the harness's session lifecycle event capture. The `claude-code` entry SHALL indicate no lifecycle events are captured. The `pi` entry SHALL indicate `SessionStart` is captured but `SessionEnd` is not. Limitations SHALL be documented per harness.

#### Scenario: Claude Code lifecycle is accurately declared

- **WHEN** the `claude-code` entry is inspected
- **THEN** the `lifecycle` field or equivalent capability declaration indicates no lifecycle events
- **AND** the entry does NOT claim `SessionStart` or `SessionEnd` is captured
- **AND** a `limitations` field notes that the hook is PreToolUse-only with unconditional `Success` outcome

#### Scenario: Pi lifecycle is accurately declared

- **WHEN** the `pi` entry is inspected
- **THEN** `capturedLifecycleEvents` or equivalent field includes `SessionStart`
- **AND** the entry does NOT claim `SessionEnd` is captured
- **AND** a `limitations` field notes that `SessionEnd` is deferred due to Pi's `session_shutdown` timing

#### Scenario: Manifest does not claim unsupported behavior

- **WHEN** `scryrs.json` is inspected in its entirety
- **THEN** no field claims that Claude Code emits lifecycle events
- **AND** no field claims that Pi emits `SessionEnd`
- **AND** no field implies full lifecycle coverage across all harnesses

### Requirement: Manifest does not include consumer-side install targets

The manifest SHALL NOT include consumer-side installation target paths (such as `.claude/hooks/scryrs-hook.mjs` or `.pi/extensions/pi-trace/index.ts`). Repository reference paths SHALL describe source locations only. The installer's hardcoded registry in `crates/scryrs-cli/src/init.rs` remains the sole source of truth for install targets.

#### Scenario: No consumer install paths in manifest

- **WHEN** `scryrs.json` is parsed
- **THEN** no field contains `.claude/` or `.pi/extensions/` paths
- **AND** all paths are repository-relative source paths in `hooks/`

### Requirement: Manifest is repository metadata, not consumed by current installer

The manifest SHALL be repository metadata only. The current `scryrs init` installer is not required to read or depend on the manifest. This is consistent with the accepted init-installer specification.

#### Scenario: Manifest is present but installer ignores it

- **GIVEN** `scryrs.json` exists at the repository root
- **WHEN** a consumer runs `scryrs init --agent claude-code` in their project
- **THEN** the installer does not read or depend on the manifest for installation decisions
- **AND** installation behavior is identical regardless of the manifest's presence

### Requirement: Manifest is limited to repository metadata and does not change code

This change SHALL NOT modify any Rust crate, CLI behavior, hook source file, or existing OpenSpec capability spec. The manifest SHALL be a new artifact only.

#### Scenario: No code changes

- **WHEN** this change is implemented
- **THEN** no files in `crates/` are modified
- **AND** no files in `hooks/` are modified
- **AND** no existing `openspec/specs/` files are modified

#### Scenario: Existing specs are unchanged

- **WHEN** this change is implemented
- **THEN** `openspec/specs/init-installer/spec.md` is unchanged
- **AND** `openspec/specs/trace-hook-contract/spec.md` is unchanged
- **AND** `openspec/specs/scryrs-record-endpoint/spec.md` is unchanged

