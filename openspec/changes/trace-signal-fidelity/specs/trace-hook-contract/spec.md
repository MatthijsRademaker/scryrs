## ADDED Requirements

### Requirement: Path subjects are normalized to a canonical repository-relative form

Every trace event whose subject is a filesystem path SHALL carry that subject in canonical repository-relative POSIX form, normalized once during harness translation before the event is persisted. Normalization SHALL apply to `FileOpened`, `EditMade`, `FailedLookup` path subjects, and `DocRetrieved`. Subjects that are not paths — `search`, `symbol`, and `command` subjects — SHALL NOT be normalized.

Normalization SHALL make `(subject_kind, subject)` a stable grouping key: a single file addressed by an agent in more than one way SHALL produce exactly one subject string.

#### Scenario: Absolute and relative addressing of the same file collapse to one subject

- **GIVEN** a repository root at `/repo`
- **WHEN** one event addresses `/repo/crates/scryrs-cli/src/init.rs` and another addresses `crates/scryrs-cli/src/init.rs`
- **THEN** both events are persisted with subject `crates/scryrs-cli/src/init.rs`
- **AND** hotspot scoring groups them into a single entry

#### Scenario: Redundant path segments are normalized away

- **WHEN** an event addresses `crates/scryrs-cli/../scryrs-cli/src/init.rs` or `./crates//scryrs-cli/src/init.rs`
- **THEN** the persisted subject is `crates/scryrs-cli/src/init.rs`

#### Scenario: Non-path subjects are left untouched

- **WHEN** a `SearchRun` event carries the query `../src` or an `lsp_navigation` event carries the symbol name `Self::new`
- **THEN** the subject is persisted verbatim without path normalization

#### Scenario: Externally supplied pre-built events are recorded as given

- **WHEN** a pre-built `TraceEvent` JSON is ingested through `scryrs record --stdin` rather than produced by a harness adapter
- **THEN** its subject is recorded as supplied and is not rewritten by the store

### Requirement: Repository-external path subjects are marked, not rewritten or dropped

A path subject resolving outside the repository root SHALL be recorded verbatim and marked as external. It SHALL NOT be rewritten into a traversal-relative path, and SHALL NOT be silently dropped. External subjects SHALL be grouped separately from repository-relative subjects during scoring.

#### Scenario: A path outside the repository is preserved and marked

- **GIVEN** a repository root at `/repo`
- **WHEN** an agent reads `/home/user/.claude/CLAUDE.md`
- **THEN** the event is persisted with the subject recorded verbatim and marked external
- **AND** it is not rewritten to a `../` relative form

#### Scenario: External and internal subjects do not merge

- **WHEN** an external subject and a repository-relative subject share the same trailing path segments
- **THEN** hotspot scoring produces two separate entries

### Requirement: A missing key input field drops the event with a diagnostic and never records a placeholder

When a harness adapter recognizes a supported tool but the tool's key input field is absent, the adapter SHALL drop the event and SHALL write a diagnostic to stderr naming the harness, the tool, and the expected field. It SHALL NOT record a placeholder subject such as `"unknown"` or the empty string.

The diagnostic SHALL be unconditional and SHALL NOT be gated on `SCRYRS_DEBUG`, so that a harness contract violation is always visible.

Fail-open toward the agent SHALL be preserved on this path: the hook command SHALL exit 0, SHALL write nothing to stdout, and SHALL NOT modify the agent-visible tool result.

#### Scenario: Missing key field drops the event

- **WHEN** a supported tool event arrives whose key input field is absent
- **THEN** no trace event is persisted
- **AND** a diagnostic naming the harness, tool, and expected field is written to stderr
- **AND** the command exits 0 with empty stdout

#### Scenario: No placeholder subject is ever persisted

- **WHEN** any harness adapter translates any supported tool event
- **THEN** no persisted event carries the subject `"unknown"` or an empty-string subject

#### Scenario: All harness adapters share one extraction contract

- **GIVEN** more than one harness adapter exists
- **WHEN** each adapter extracts a key input field
- **THEN** all adapters use the same shared extraction helper
- **AND** no adapter defines its own missing-field default
