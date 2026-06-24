## ADDED Requirements

### Requirement: `scryrs hook <harness>` subcommand exists and is discoverable

The CLI SHALL provide a `hook` subcommand taking a required positional `<harness>` argument (`claude-code` or `pi`). It SHALL be reachable through the pre-clap command whitelist, documented in `write_help()`, and exposed in `--help-json`. Input SHALL be read from stdin by default, with an alternative `--file <path>`; `--stdin` and `--file` SHALL be mutually exclusive.

#### Scenario: hook reaches clap dispatch

- **GIVEN** the pre-clap unknown-command check
- **WHEN** `scryrs hook claude-code` is invoked
- **THEN** `"hook"` is recognized as a known command and arguments reach clap dispatch

#### Scenario: hook appears in help surfaces

- **WHEN** `scryrs --help` and `scryrs --help-json` are invoked
- **THEN** both include a `hook` entry listing `claude-code` and `pi` as supported harnesses
- **AND** the `--help-json` entry documents the `--file` argument and the stdin default

#### Scenario: unknown harness is handled without blocking

- **WHEN** `scryrs hook bogus` is invoked
- **THEN** the command does not persist any event
- **AND** it exits 0 (the fail-open contract applies to all harness routing errors)

### Requirement: hook translates a harness event and persists it via the canonical store

For a recognized harness, the subcommand SHALL parse the harness's native event, delegate translation to the `scryrs-adapter-harness` crate, and append any resulting `TraceEvent` to the canonical `EventStore` used by `scryrs record`. When the adapter returns no event (untracked tool / pass-through), the subcommand SHALL persist nothing.

#### Scenario: tracked tool event is persisted

- **GIVEN** a Claude Code PreToolUse payload for a tracked tool on stdin
- **WHEN** `scryrs hook claude-code` runs
- **THEN** exactly one `TraceEvent` is appended to the `EventStore`
- **AND** its `tool_name` matches the harness tool name

#### Scenario: untracked tool yields no event

- **GIVEN** a harness event for a tool the adapter does not track
- **WHEN** `scryrs hook <harness>` runs
- **THEN** no event is persisted
- **AND** the exit code is 0

### Requirement: hook resolves session identity and store location from the payload

The subcommand SHALL read `session_id` from the harness event payload when present, and SHALL resolve the trace store path relative to the payload `cwd` when present, falling back to the canonical cwd-relative store path otherwise. It SHALL NOT depend on `CLAUDE_SESSION_ID`-style environment variables for identity.

#### Scenario: session_id taken from payload

- **GIVEN** a PreToolUse payload whose `session_id` is `"abc123"`
- **WHEN** `scryrs hook claude-code` persists the event
- **THEN** the persisted `TraceEvent.session_id` is `"abc123"`

#### Scenario: store resolved against payload cwd

- **GIVEN** a payload whose `cwd` is a project directory
- **WHEN** the event is persisted
- **THEN** it is written to the `.scryrs` store under that `cwd`, not under the spawned process's working directory if they differ

### Requirement: hook never blocks harness execution (fail-open)

The subcommand SHALL always exit 0 and SHALL write nothing to stdout, regardless of any internal failure. On any error — malformed input, unknown harness, translation failure, or store error — it SHALL append a timestamped line to `.scryrs/hooks/<harness>-warnings.log` and still exit 0. It SHALL NOT use the `record` 1/2 exit policy.

#### Scenario: malformed input does not block

- **GIVEN** non-JSON bytes on stdin
- **WHEN** `scryrs hook claude-code` runs
- **THEN** the exit code is 0
- **AND** stdout is empty
- **AND** a warning line is appended to `.scryrs/hooks/claude-code-warnings.log`

#### Scenario: store failure does not block

- **GIVEN** a valid payload but an unwritable trace store
- **WHEN** `scryrs hook claude-code` runs
- **THEN** the exit code is 0
- **AND** a warning line records the persistence failure

#### Scenario: exit 0 with empty stdout is the allow signal

- **WHEN** Claude Code invokes the hook command and it exits 0 with no stdout
- **THEN** Claude Code proceeds with the tool (no `{continue:true}` payload is required or emitted)
