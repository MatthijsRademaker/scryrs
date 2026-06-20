## ADDED Requirements

### Requirement: Record command accepts JSONL trace events from stdin and file
The system SHALL expose `scryrs record --stdin` and `scryrs record --file <PATH>` as the only public ingestion modes for this change. Both modes MUST read newline-delimited JSON using the existing `scryrs-types::TraceEvent` wire contract, use the same ingestion path, and reject invocations that provide both or neither input mode.

#### Scenario: Hook pipes events via stdin
- **WHEN** a hook runs `scryrs record --stdin` and writes newline-delimited `TraceEvent` JSON to stdin
- **THEN** the system ingests each non-empty line through the shared record path
- **THEN** accepted events are persisted

#### Scenario: Record reads events from file
- **WHEN** a user runs `scryrs record --file session.jsonl`
- **THEN** the system reads the file as newline-delimited `TraceEvent` JSON
- **THEN** the system uses the same validation, storage, summary, and exit-code behavior as `--stdin`

#### Scenario: Invalid input mode fails fast
- **WHEN** `scryrs record` is invoked with both `--stdin` and `--file`, or with neither
- **THEN** the system writes a usage error to stderr
- **THEN** the system exits with code 2

### Requirement: Accepted events are persisted through a minimal local event store
The system SHALL persist each accepted `TraceEvent` through a core-owned append-only event store. This change MUST keep the store surface minimal and MUST use a default local JSONL store at `.scryrs/events.jsonl` relative to the current working directory when recording events.

#### Scenario: Valid event is appended to the default store
- **WHEN** a valid record input is accepted and this task's default store is used
- **THEN** the event is appended to `.scryrs/events.jsonl`
- **THEN** the persisted record remains JSONL-compatible for later ingestion

#### Scenario: Storage remains ingestion-only
- **WHEN** this change is implemented
- **THEN** the store surface only appends accepted events and reports stored counts needed by record ingestion
- **THEN** the change does not add hotspot analysis, promotion logic, query APIs, or SQLite-specific requirements

### Requirement: Validation rejects malformed non-empty lines without aborting ingestion
The system SHALL validate each non-empty physical line as a `TraceEvent`. Malformed JSON or schema-invalid events MUST be rejected with deterministic diagnostics containing the 1-based physical line number, the failing field/path when available, and a reason, while ingestion continues with later lines.

#### Scenario: Malformed JSON line is rejected
- **WHEN** a non-empty line contains invalid JSON
- **THEN** the system records a rejection for that 1-based physical line number
- **THEN** the rejection diagnostic includes the parse reason
- **THEN** the system continues processing subsequent lines

#### Scenario: Schema-invalid event is rejected
- **WHEN** a non-empty line parses as JSON but fails `TraceEvent` validation
- **THEN** the system records a rejection for that 1-based physical line number
- **THEN** the rejection diagnostic includes the failing field/path when available and a reason
- **THEN** the system continues processing subsequent lines

#### Scenario: Blank line is ignored
- **WHEN** a physical input line is empty or whitespace-only
- **THEN** the system skips that line
- **THEN** the line does not increment accepted or rejected counts

### Requirement: Record output and exit codes are deterministic
The `record` command SHALL emit exactly one JSON summary object to stdout with fields `command`, `schemaVersion`, `accepted`, and `rejected`. Rejection diagnostics MUST be emitted deterministically to stderr as one JSON object per rejected non-empty line containing `line`, `field` when available, and `reason`. Exit code 0 MUST mean all processed non-empty lines were accepted, exit code 1 MUST mean ingestion completed with one or more rejected events, and exit code 2 MUST mean fatal usage or I/O failure.

#### Scenario: All events are valid
- **WHEN** every processed non-empty line is accepted
- **THEN** stdout contains a single JSON object with `command: "record"` and matching `accepted` / `rejected` counts
- **THEN** stderr contains no rejection diagnostics
- **THEN** the system exits with code 0

#### Scenario: Some events are rejected
- **WHEN** at least one processed non-empty line is rejected and later lines still complete
- **THEN** stdout still contains one summary JSON object with final accepted and rejected counts
- **THEN** stderr contains one rejection diagnostic object per rejected line
- **THEN** the system exits with code 1

#### Scenario: Fatal file or stream setup error
- **WHEN** the input file cannot be opened or another fatal record setup error occurs
- **THEN** the system writes the fatal error to stderr
- **THEN** the system does not emit a success summary for partially unread input
- **THEN** the system exits with code 2

### Requirement: Discovery surfaces describe the record endpoint accurately
The CLI discovery surfaces SHALL document `record` as a first-class command. `scryrs --help`, `scryrs --help-json`, `README.md`, and the CLI contract note MUST describe `--stdin` / `--file <PATH>`, the deterministic summary contract, and the command-specific `0/1/2` exit semantics. The machine-readable CLI surface MUST bump its `surfaceVersion` minor version for this additive command.

#### Scenario: Help text lists record
- **WHEN** `scryrs --help` is invoked after this change
- **THEN** the output lists `scryrs record --stdin` and `scryrs record --file <PATH>` alongside the existing `hotspots` placeholder
- **THEN** the output describes the record command's output and exit codes

#### Scenario: Help-json includes record metadata
- **WHEN** `scryrs --help-json` is invoked after this change
- **THEN** the surface document includes a `record` command entry with mutually exclusive `--stdin` and `--file` modes
- **THEN** the document describes the summary JSON fields and the record exit-code contract
- **THEN** the `surfaceVersion` reflects an additive minor bump from the prior surface

#### Scenario: Project docs no longer hide record
- **WHEN** a reader reviews `README.md` or `.devagent/docs/docs/cli-v0-contract.md`
- **THEN** those docs describe `record` as the supported ingestion endpoint for trace events
- **THEN** they no longer describe the public surface as a one-command-only contract

### Requirement: Record remains ingestion-only
The `record` endpoint SHALL be limited to ingestion. This change MUST NOT trigger hotspot scoring, promotion logic, graph building, routing, LLM calls, or harness-specific IPC beyond JSONL over stdin or file.

#### Scenario: Record does not invoke analysis behavior
- **WHEN** `scryrs record` accepts or rejects events
- **THEN** it only validates, stores, summarizes, and reports diagnostics
- **THEN** no hotspot report or promotion output is produced