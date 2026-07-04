## MODIFIED Requirements

### Requirement: Manifest enumerates supported harnesses with per-harness metadata

The manifest SHALL use a `supported_harnesses` array with one entry per harness (`claude-code` and `pi`), ordered deterministically. Each entry SHALL include `integration` (how the harness reaches scryrs), `interceptedTools` (harness-specific tool names the integration observes), and `capturedEventFamilies` (canonical `TraceEventType` families produced). The `claude-code` entry's `integration` SHALL describe the native command `scryrs hook claude-code` and SHALL NOT reference any `referenceSource` hook file (none exists). The `pi` entry SHALL include `referenceSource` `"hooks/pi/index.ts"` (the thin shim) and SHALL describe delegation to `scryrs hook pi`.

#### Scenario: Supported harnesses are enumerated

- **WHEN** a consumer reads `supported_harnesses`
- **THEN** the array contains exactly two entries for `claude-code` and `pi`
- **AND** entries are ordered deterministically

#### Scenario: Claude Code entry describes the native command, not a file

- **WHEN** the `claude-code` entry is inspected
- **THEN** `integration` identifies the native `scryrs hook claude-code` command
- **AND** the entry contains no `referenceSource` path to a `.mjs` file
- **AND** no field references `hooks/claude-code/scryrs-hook.mjs`

#### Scenario: Pi entry references the thin shim and delegation

- **WHEN** the `pi` entry is inspected
- **THEN** `referenceSource` equals `"hooks/pi/index.ts"`
- **AND** `integration` states the shim delegates translation to `scryrs hook pi`

#### Scenario: Claude Code entry lists intercepted tools in PascalCase

- **WHEN** the `claude-code` entry is inspected
- **THEN** `interceptedTools` contains the PascalCase names: `Read`, `Grep`, `Glob`, `Edit`, `Write`, `NotebookEdit`, `WebSearch`, `WebFetch` (with `Bash` documented as debug-gated)
- **AND** the field clearly describes what the integration observes, not a callable surface

#### Scenario: Pi entry lists intercepted tools

- **WHEN** the `pi` entry is inspected
- **THEN** `interceptedTools` contains: `read`, `ast_grep_search`, `lsp_navigation`, `edit`, `write` (with `bash` documented as debug-gated)

#### Scenario: Claude Code entry lists captured event families

- **WHEN** the `claude-code` entry is inspected
- **THEN** `capturedEventFamilies` contains: `FileOpened`, `SearchRun`, `EditMade`, `DocRetrieved` (with `CommandExecuted` debug-gated)
- **AND** the families match canonical `TraceEventType` variants in `scryrs-types`

#### Scenario: Pi entry lists captured event families

- **WHEN** the `pi` entry is inspected
- **THEN** `capturedEventFamilies` contains: `FileOpened`, `SearchRun`, `EditMade`, `SymbolInspected`, `FailedLookup` (with `CommandExecuted` debug-gated)
- **AND** the families match canonical `TraceEventType` variants in `scryrs-types`

### Requirement: Manifest defines the harness integration and record invocation contract

The manifest SHALL include a `record` object describing the low-level ingestion primitive: `command` `"scryrs"`, canonical args `["record", "--stdin"]`, with `--file <PATH>` documented as the alternate mode. The manifest SHALL additionally document the harness-facing `scryrs hook <harness>` command as the integration entry point, noting Claude Code uses stdin and Pi uses `--file`.

#### Scenario: Record invocation contract is defined

- **WHEN** a hook author reads the `record` section
- **THEN** they see `command` `"scryrs"` and `args` `["record", "--stdin"]`
- **AND** `--file <PATH>` is documented as the alternate ingestion mode

#### Scenario: Harness command is documented as the integration entry point

- **WHEN** a hook author reads the manifest
- **THEN** `scryrs hook <harness>` is documented as the harness integration command
- **AND** Claude Code is noted as stdin-based and Pi as `--file`-based
