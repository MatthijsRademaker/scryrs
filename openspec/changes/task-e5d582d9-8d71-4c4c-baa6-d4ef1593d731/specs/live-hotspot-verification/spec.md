## ADDED Requirements

### Requirement: Docker-backed live verification runs headlessly against the real `scryrs` binary

The repository SHALL provide a standalone `scripts/verify-live-hotspots` entrypoint that follows the existing Docker-backed verification pattern: build the release `scryrs` binary with `scripts/lib/docker-verification.sh`, copy it into `.docker-fixtures/`, and run the live-workflow fixture in a Debian/glibc Node container. The verification SHALL require Docker or DinD, SHALL NOT require host Rust or Node, and SHALL fail loudly when the fixture detects a startup, transport, parsing, assertion, or timeout error.

#### Scenario: Maintainer runs live verification in a headless Docker environment

- **GIVEN** Docker or DinD is available
- **WHEN** a maintainer runs `scripts/verify-live-hotspots`
- **THEN** the script builds or locates the real `scryrs` binary through the Docker-backed helper flow
- **AND** the fixture runs in a glibc-compatible Node container
- **AND** the command exits `0` only when all live verification assertions pass

#### Scenario: Fixture failure is surfaced as a non-zero verification result

- **GIVEN** the live verification fixture encounters a server startup failure, malformed response, failed assertion, or timeout
- **WHEN** `scripts/verify-live-hotspots` finishes
- **THEN** the script exits non-zero
- **AND** the output identifies the failing phase clearly enough to debug the live workflow

### Requirement: Verification proves cumulative multi-agent live ingest against one shared server store

The live verification fixture SHALL start `scryrs server` with a fresh temp SQLite store on a nonzero port, wait for readiness through the live hotspot query API, and drive remote `scryrs record --file` submissions from two explicit agent identities to one shared repository on that server. Remote configuration SHALL be supplied through `SCRYRS_REMOTE_INGEST_URL`, `SCRYRS_REPOSITORY_ID`, `SCRYRS_WORKSPACE_ID`, and `SCRYRS_AGENT_ID` environment variables rather than `scryrs init --mode live` or committed manifests. The deterministic event set SHALL include overlapping subject-bearing `EditMade` success events sufficient to cross the default threshold of `10` without adding a new server flag.

#### Scenario: Two agents contribute to one overlapping hotspot

- **GIVEN** two configured agent identities submit a deterministic overlapping event set totaling four accepted `EditMade` success events for one file subject
- **WHEN** the fixture queries `GET /v1/repositories/{repository_id}/hotspots?window=cumulative`
- **THEN** the matching hotspot entry reflects cumulative server-owned state from both agents
- **AND** the entry has crossed the default signal threshold
- **AND** the entry's evidence row IDs include rows accepted from both submissions
- **AND** `sessionCount` proves the hotspot includes more than one contributing session

#### Scenario: Server readiness is proved before submissions begin

- **GIVEN** the fixture has just spawned `scryrs server`
- **WHEN** it polls the live hotspot query endpoint for a probe repository
- **THEN** submissions do not begin until the endpoint returns a successful response
- **AND** a readiness timeout fails the fixture loudly instead of racing the server startup

### Requirement: Duplicate remote replay is acknowledged idempotently and does not change cumulative hotspot state

The live verification fixture SHALL re-submit one producer's identical JSONL through `scryrs record --file` after an initial accepted submission. Because remote mode derives deterministic `producer_event_id` values from accepted input lines, the replay SHALL be acknowledged as duplicate/idempotent and SHALL NOT change cumulative hotspot score, counts, session count, evidence row IDs, or threshold-signal history.

#### Scenario: Replaying the same producer file returns duplicate acknowledgments only

- **GIVEN** one producer already submitted an accepted JSONL file in remote mode
- **WHEN** the fixture submits the exact same file again with the same repository, workspace, and agent identity
- **THEN** the remote summary reports duplicate/idempotent items for the previously accepted lines
- **AND** the follow-up cumulative hotspot query is unchanged from the pre-replay state

### Requirement: Verification proves SSE replay and cursor resume on the live signal stream

The live verification fixture SHALL verify `GET /v1/repositories/{repository_id}/signals` with a time-gated two-phase protocol. Phase 1 SHALL connect with `after=0`, parse `text/event-stream` `id:` and `data:` fields until idle or overall timeout, record the highest signal ID seen, and disconnect explicitly. Phase 2 SHALL reconnect with `after=<last_seen_id>` and assert that already-seen signal IDs are not replayed and that only newer signal IDs, if any, are delivered.

#### Scenario: `after=0` replays persisted threshold-crossing signals in order

- **GIVEN** the server already persisted one or more hotspot threshold-crossing signals for the repository
- **WHEN** the fixture connects to `GET /v1/repositories/{repository_id}/signals?after=0`
- **THEN** it receives `HotspotSignal` payloads as `text/event-stream` messages
- **AND** replayed messages are ordered by ascending persisted signal ID
- **AND** the fixture can record the highest replayed signal ID before disconnecting

#### Scenario: Cursor resume does not replay already-seen signal IDs

- **GIVEN** the fixture previously recorded `last_seen_id` from an `after=0` replay connection
- **WHEN** it reconnects to `GET /v1/repositories/{repository_id}/signals?after=<last_seen_id>`
- **THEN** no previously seen signal ID is replayed
- **AND** any delivered message has a signal ID greater than `last_seen_id`

### Requirement: Verification documentation defines invocation, proof scope, and initial CI posture

`scripts/verification/README.md` SHALL document the live verification entrypoint, its Docker prerequisites, and the behaviors it proves. The documentation SHALL describe the multi-agent cumulative-ingest proof, duplicate idempotency proof, SSE replay/resume proof, and loud-failure behavior. It SHALL also state that live dashboard smoke is out of scope for this verification and that the initial CI posture is standalone/nightly rather than PR-gate.

#### Scenario: README tells maintainers how to run and interpret live verification

- **GIVEN** a maintainer opens `scripts/verification/README.md`
- **WHEN** they read the live verification section
- **THEN** they can see the `scripts/verify-live-hotspots` entrypoint and prerequisites
- **AND** they can see which live-server behaviors the suite proves
- **AND** they can see that the suite is intended for standalone/nightly use first

#### Scenario: README does not claim live dashboard coverage

- **GIVEN** the current dashboard reads local artifacts instead of live server APIs
- **WHEN** the live verification documentation describes scope boundaries
- **THEN** it does not claim that `scripts/verify-live-hotspots` verifies a live dashboard mode
- **AND** it identifies dashboard smoke as out of scope for this task
