## 1. Replace the core JSONL store with the SQLite datastore contract

- [x] 1.1 Add the SQLite dependency in `crates/scryrs-core` and replace the JSONL store implementation with a core-owned datastore that opens `.scryrs/scryrs.db`.
- [x] 1.2 Define datastore initialization with a version table, a `trace_events` table, and the required indexes for subject lookup, event-type filtering, session/timestamp ordering, and failure analysis.
- [x] 1.3 Store each accepted event as canonical event JSON plus the normalized query columns for `schema_version`, `timestamp`, `session_id`, `event_type`, `tool_name`, `subject_kind`, `subject`, `outcome`, and `failure_reason`.
- [x] 1.4 Add the minimal extraction helpers needed in the core/types layer, including `subject_kind` derivation and failure-reason mapping, without changing the TraceEvent wire format.
- [x] 1.5 Add core tests for schema creation, canonical path creation, row insertion, normalized field extraction, index presence, and unknown-schema-version failure.

## 2. Rewire `scryrs record` to the core-owned SQLite store

- [x] 2.1 Update CLI record composition to use the core datastore API and the core-owned canonical path instead of embedding `.scryrs/events.jsonl`.
- [x] 2.2 Migrate the CLI test-only store override and related record tests from JSONL-path semantics to database-path semantics.
- [x] 2.3 Ensure SQLite open, write, and schema-validation errors are fatal record failures with exit code `2`, and remove any canonical JSONL fallback behavior.
- [x] 2.4 Add CLI tests proving accepted events create and write `.scryrs/scryrs.db` and that canonical persistence no longer appends to `.scryrs/events.jsonl`.

## 3. Update specs, verification, and documentation to the SQLite contract

- [x] 3.1 Amend the OpenSpec change and live capability expectations so `scryrs-record-endpoint` and `cross-harness-verification` describe SQLite as the canonical store.
- [x] 3.2 Update verification fixtures and test assertions that currently read `.scryrs/events.jsonl` so they validate persisted data from `.scryrs/scryrs.db` instead.
- [x] 3.3 Update `README.md`, `.devagent/docs/docs/cli-v0-contract.md`, `.devagent/docs/docs/trace-hook-contract.md`, `hooks/claude-code/README.md`, `hooks/pi/README.md`, and any related snapshots/help surfaces so JSONL is documented only as input and SQLite is documented as canonical persistence.

## 4. Validate the change in the Docker-backed workspace flow

- [x] 4.1 Run the relevant Docker-backed test path that covers `scryrs-core`, `scryrs-cli`, and verification fixtures.
- [x] 4.2 Run the Docker-backed check/lint path and confirm the new SQLite-backed storage contract does not leave stale JSONL assertions behind.
