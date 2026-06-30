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

### Requirement: Manifest may declare remote ingest defaults

The manifest SHALL allow an optional top-level `remote` object for CLI live-config defaults. When present, the object SHALL use fields `ingest_url`, `workspace_id`, `docker_network`, and MAY also include the optional override fields `repository_id`, `agent_id`, and `timeout_ms`. The `docker_network` field names the external Docker network the live server joins and is the committed source of truth for that name. The `remote` object is configuration only; it does not register a direct HTTP tool surface and does not change the manifest's hook-interface scope.

#### Scenario: Remote defaults are parseable

- **WHEN** a consumer reads a `scryrs.json` file that includes `remote`
- **THEN** the `remote` object parses as ordinary JSON configuration
- **AND** its fields are limited to live-config defaults rather than a callable tool surface

#### Scenario: Committed shared constants are present

- **WHEN** a consumer reads a `remote` object written by `scryrs init` in live mode
- **THEN** `ingest_url`, `workspace_id`, and `docker_network` are present
- **AND** `docker_network` is a string naming an external Docker network
- **AND** `repository_id` and `agent_id` are absent because they are derived or autogenerated at resolution time

#### Scenario: Remote defaults do not imply agent-callable HTTP behavior

- **WHEN** a developer inspects the manifest's `remote` section
- **THEN** they see live-config defaults for the CLI
- **AND** they do not see the manifest described as a direct HTTP integration surface

### Requirement: Manifest remote defaults are subordinate to environment overrides

When a `remote` object is present in `scryrs.json`, its values SHALL act as defaults for CLI transport resolution and MAY be overridden by `SCRYRS_REMOTE_INGEST_URL`, `SCRYRS_REPOSITORY_ID`, `SCRYRS_WORKSPACE_ID`, `SCRYRS_AGENT_ID`, `SCRYRS_REMOTE_TIMEOUT_MS`, and (for the network field) `SCRYRS_DOCKER_NETWORK`. The committed manifest values SHALL be the lowest-precedence layer below CLI flags, the process environment, and `.scryrs/.env`. An empty or omitted `ingest_url` SHALL NOT activate remote mode.

#### Scenario: Environment overrides manifest remote values

- **GIVEN** a manifest `remote` section provides one set of values
- **AND** one or more corresponding override environment variables are set
- **WHEN** the CLI resolves remote configuration
- **THEN** the environment values take precedence for those fields

#### Scenario: Docker network resolves from the manifest as the committed base

- **GIVEN** `scryrs.json` `remote.docker_network` provides a committed network name
- **AND** no `SCRYRS_DOCKER_NETWORK` environment variable, `.scryrs/.env` value, or CLI flag is present
- **WHEN** the external network is resolved
- **THEN** the committed manifest value is used

#### Scenario: Empty ingest URL does not activate remote mode

- **WHEN** the manifest omits `remote.ingest_url` or sets it to an empty value
- **THEN** the manifest alone does not activate remote ingest mode
- **AND** the CLI remains in local mode unless a non-empty ingest URL is provided by a higher-precedence source

### Requirement: Live identity derives repository_id and autogenerates agent_id when uncommitted

Live transport resolution SHALL NOT require `repository_id` or `agent_id` to be present in committed config. When `repository_id` is not provided by any override layer, the system SHALL derive it from the normalized Git remote origin URL. When `agent_id` is not provided by any override layer, the system SHALL autogenerate it from a stable per-container identity (the container hostname) that remains constant across the separate hook processes spawned within one container's lifetime. Explicit override values (CLI flag, environment, `.scryrs/.env`, or manifest) SHALL take precedence over derivation and autogeneration.

#### Scenario: agent_id is autogenerated from a stable container identity

- **GIVEN** no `agent_id` is provided by any override layer and no committed `agent_id` exists
- **WHEN** the hook resolves live config across multiple tool-call invocations within the same container
- **THEN** an `agent_id` is autogenerated from the container hostname
- **AND** the autogenerated value is identical across those invocations
- **AND** live mode is not blocked by the absence of a committed `agent_id`

#### Scenario: repository_id is derived from the Git remote when uncommitted

- **GIVEN** no `repository_id` is provided by any override layer and no committed `repository_id` exists
- **AND** the workspace is a Git checkout with an origin remote
- **WHEN** the hook resolves live config
- **THEN** `repository_id` is derived from the normalized Git remote origin URL

#### Scenario: Explicit overrides win over derivation and autogeneration

- **GIVEN** an explicit `agent_id` or `repository_id` is provided via flag, environment, `.scryrs/.env`, or the manifest
- **WHEN** live config is resolved
- **THEN** the explicit value is used instead of the autogenerated or derived value

#### Scenario: Undeterminable repository_id still fails loudly

- **GIVEN** no `repository_id` override is present and the workspace is not a Git checkout with an origin remote
- **WHEN** live config is resolved
- **THEN** resolution fails with deterministic remediation guidance naming `repository_id`
- **AND** it does not silently substitute an empty or placeholder value

