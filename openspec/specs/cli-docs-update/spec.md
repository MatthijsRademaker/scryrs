# cli-docs-update Specification

## Purpose
TBD - created by archiving change task-38ad5a05-c745-4bb5-835a-aa8153014618. Update Purpose after archive.
## Requirements
### Requirement: Domain-Driven Opening

The CLI documentation page SHALL open with a domain-driven overview that explains what scryrs CLI helps users accomplish, which problems it solves, and the two main workflow paths.

#### Scenario: Reader discovers the CLI page

**Given** a developer lands on the CLI documentation page at `/cli-v0-contract`
**When** they read the opening section
**Then** they see an explanation of how scryrs CLI helps teams observe agent activity, detect knowledge hotspots, inspect them locally, and optionally centralize ingest
**And** they see a description of the two workflow paths: local observe→detect loop (`hook`/`record` → `hotspots` → `dashboard`) and central live-ingest flow (`record`/`hook` with remote config → `server`)
**And** the opening includes cross-links to `hotspots.md` for hotspot interpretation and `trace-hook-contract.md` for harness integration rules

---

### Requirement: Six Command Surface

The CLI documentation page SHALL document exactly six implemented root commands matching `crates/scryrs-cli/src/dispatch.rs` and `crates/scryrs-cli/src/help_text.rs`.

#### Scenario: Agent integrator counts documented commands

**Given** the CLI documentation page
**When** a reader counts the documented commands
**Then** there are exactly six: `hotspots`, `record`, `hook`, `init`, `dashboard`, and `server`
**And** the page does not claim "five implemented commands"

#### Scenario: Root flags remain documented

**Given** the CLI documentation page
**When** a reader checks global behavior
**Then** root help (`-h`/`--help`), version (`-V`/`--version`), and `--help-json` are documented
**And** bare `scryrs` invocation printing help and exiting 0 is documented

---

### Requirement: Hook Command Section

The CLI documentation page SHALL include a dedicated `scryrs hook <HARNESS>` section documenting fail-open semantics matching `crates/scryrs-cli/src/hook.rs`.

#### Scenario: Developer reads the hook command contract

**Given** the CLI documentation page
**When** a reader navigates to the `scryrs hook` section
**Then** they see the fail-open contract: the command always exits 0 with empty stdout and never blocks the harness
**And** they see supported harnesses: `claude-code` (reads PreToolUse event from stdin) and `pi` (reads from `--file <PATH>`)
**And** they see transport asymmetry clearly described (stdin for claude-code, --file for pi)
**And** they see the warning-log side channel at `.scryrs/hooks/<harness>-warnings.log`

#### Scenario: Agent integrator uses hook agent-facing contract

**Given** the CLI documentation page
**When** a reader navigates to the hook agent-facing contract subsection
**Then** they see "When to call," input modes (`--stdin`, `--file <PATH>`, mutually exclusive), output contract (always empty stdout, exit 0), and exit codes
**And** the subsection follows the same pattern as the existing `record`, `hotspots`, `dashboard`, and `server` agent-facing subsections

---

### Requirement: Remote Record Transport Coverage

The `scryrs record` section SHALL document both local and remote transport modes matching `crates/scryrs-cli/src/remote_config.rs` and `scryrs.json`.

#### Scenario: Operator configures remote ingest

**Given** the CLI documentation page
**When** a reader consults the `record` section for remote transport
**Then** they see that remote mode activates when `scryrs.json` `remote.ingest_url` or `SCRYRS_REMOTE_INGEST_URL` is non-empty
**And** they see config precedence: 1. environment variables (`SCRYRS_REMOTE_*`), 2. `scryrs.json` `remote` section, 3. git remote origin URL (repository_id fallback only)
**And** they see no-dual-write semantics: "remote mode skips `.scryrs/scryrs.db` entirely — no dual-write, no local fallback, no retry spool"

#### Scenario: Operator reads remote output contract

**Given** the CLI documentation page
**When** a reader checks the remote record output envelope
**Then** they see the JSON fields: `command`, `schemaVersion`, `transport` (always `"remote"`), `accepted`, `duplicate`, `rejected`, and `failed`
**And** they see exit code implications: 0 for all accepted (duplicates non-fatal), 1 for rejections or I/O error, 2 for missing remote identity or transport failures (timeout, connection, non-2xx, malformed response)
**And** they see default timeout is 3000 ms, overridable via `SCRYRS_REMOTE_TIMEOUT_MS`

---

### Requirement: Dashboard Endpoint Accuracy

The dashboard REST API table SHALL include all five routes implemented in `crates/scryrs-dashboard/src/server.rs` `router()`.

#### Scenario: Developer checks dashboard endpoints

**Given** the CLI documentation page
**When** a reader consults the dashboard REST API table
**Then** they see exactly five endpoints: `GET /api/meta`, `GET /api/hotspots`, `GET /api/sessions`, `GET /api/sessions/:sessionId`, and `GET /api/events`
**And** each endpoint has a description of its response and error states
**And** no endpoint is documented that does not exist in the server router

---

### Requirement: Out-of-Scope Commands Accuracy

The "Out of scope for v0" section SHALL not list `hook` as unknown and SHALL use a general statement for future commands.

#### Scenario: Reader checks which commands are out of scope

**Given** the CLI documentation page
**When** a reader consults the out-of-scope section
**Then** `hook` is not listed as an unknown or out-of-scope command
**And** future command names are replaced with a general statement that any command other than the six implemented commands exits 2 with a usage error

---

### Requirement: Exit-Code Policy Completeness

The global exit-code table SHALL include hook's always-exit-0 behavior and remote record failure codes.

#### Scenario: Developer checks hook exit code

**Given** the CLI documentation page
**When** a reader consults the exit-code table
**Then** exit 0 includes: "Hook: always — fail-open, never blocks the harness"
**And** exit 2 includes remote record failures: missing remote identity, transport timeout, connection failure, non-2xx response, malformed response

---

### Requirement: Nav Label Update

The navigation label in `.devagent/docs/docs/_nav.json` SHALL be renamed from `CLI v0 Contract` to `CLI Reference` while preserving the file slug.

#### Scenario: Developer browses the documentation nav

**Given** the documentation site navigation
**When** a reader looks under the "Technical" section
**Then** the CLI page is listed as "CLI Reference" (not "CLI v0 Contract")
**And** clicking it navigates to `/cli-v0-contract` (the slug is unchanged)

---

### Requirement: Cross-Links to Domain Docs

The CLI documentation page SHALL include accurate cross-links to `hotspots.md` and `trace-hook-contract.md` without duplicating their content.

#### Scenario: Reader follows cross-links from CLI page

**Given** the CLI documentation page
**When** a reader clicks a link to Hotspots
**Then** they navigate to `hotspots.md` for domain concept depth and hotspot interpretation
**When** a reader clicks a link to Trace Hook Contract
**Then** they navigate to `trace-hook-contract.md` for harness integration rules and fail-open guarantees
**And** the CLI page does not duplicate detailed hotspot interpretation or harness integration rules that already exist in those pages

---

### Requirement: Command Count Updated

The command count claim in the page opening SHALL be six.

#### Scenario: Page opening correctly states six commands

**Given** the CLI documentation page
**When** a reader reads the opening paragraph
**Then** it does not state "provides five implemented commands"
**And** it accurately reflects the six-command surface

---

### Requirement: Dashboard Endpoint Table Updated

The dashboard REST API table SHALL include all five endpoints.

#### Scenario: Full endpoint list is present

**Given** the CLI documentation page
**When** a reader consults the dashboard section
**Then** the REST API table includes `/api/meta` and `/api/sessions/:sessionId`
**And** the table documents `/api/hotspots`, `/api/sessions`, `/api/events`, `/api/meta`, and `/api/sessions/:sessionId`

---

### Requirement: Server Endpoint Table Preserved

The server REST API table SHALL document the three endpoints matching `crates/scryrs-cli/src/server.rs` `write_server_help()`.

#### Scenario: Server endpoints are present

**Given** the CLI documentation page
**When** a reader consults the server section
**Then** the REST API table documents `POST /v1/trace-events/batch`, `GET /v1/repositories/{repository_id}/hotspots`, and `GET /v1/repositories/{repository_id}/signals`
**And** the endpoint descriptions match the current server help text

---

### Requirement: Hook Not in Out-of-Scope List

The CLI documentation page SHALL not list `hook` in the out-of-scope section.

#### Scenario: Hook is not listed as unknown

**Given** the CLI documentation page
**When** a reader checks which commands are out of scope
**Then** `hook` is not present in that list
**And** `hook` has its own documented command section

---

### Requirement: No Explicit Future Command Names

The out-of-scope section SHALL use a general statement instead of enumerating future command names.

#### Scenario: Future commands are not enumerated by name

**Given** the CLI documentation page
**When** a reader checks the out-of-scope section
**Then** the section does not list specific future command names by name
**And** the section instead states that any command other than the six implemented commands exits 2 with a usage error

### Requirement: CLI reference distinguishes workspace bootstrap from repository packaging

The CLI reference docs SHALL describe consumer live setup in terms of workspace-local bootstrap artifacts and SHALL explicitly distinguish those artifacts from the scryrs repository's own packaging and maintainer-oriented Docker files.

#### Scenario: CLI reference points users to `.scryrs/` bootstrap artifacts

- **WHEN** a reader follows the live setup guidance in the CLI reference docs
- **THEN** they are directed to `.scryrs/.env` and `.scryrs/compose.yml` as the consumer-facing live bootstrap artifacts
- **AND** they are not told that checking out the scryrs source repository is a prerequisite for ordinary consumer live setup

#### Scenario: CLI reference names the external network endpoint contract

- **WHEN** a reader inspects the documented live ingest URL and networking guidance
- **THEN** the docs describe the live server joining an existing external agent network
- **AND** the documented in-network endpoint is `http://scryrs:8081`
- **AND** the docs explain that this contract is for container-attached agents on that shared network

