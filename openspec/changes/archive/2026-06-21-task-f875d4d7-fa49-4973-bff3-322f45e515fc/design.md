## Context

`scryrs record` already accepts JSONL TraceEvent input, but accepted events are persisted through a core-owned append-only JSONL store at `.scryrs/events.jsonl`. Phase 2 hotspot analysis needs indexed local evidence instead of full JSONL scans. Existing specs, tests, verification fixtures, and docs still encode JSONL as the canonical store, so this task must deliberately replace that contract rather than layering SQLite on as an optional backend.

## Goals

- Make `.scryrs/scryrs.db` the canonical local trace datastore.
- Keep schema ownership in `scryrs-core` and keep `scryrs-cli` composition-only.
- Define a versioned SQLite schema with raw-event auditability, normalized hotspot query columns, and the required indexes.
- Keep `scryrs record` input modes unchanged while making store failures fatal and removing JSONL fallback.
- Update specs, verification, docs, and tests so every canonical-persistence assertion matches the SQLite contract.

## Non-Goals

- No hotspot scoring, graph building, route explanation, proposal generation, adapters, dashboards, MCP, or LLM behavior.
- No migration from existing `.scryrs/events.jsonl` data.
- No change to the TraceEvent JSON wire contract beyond minimal helper logic needed to extract storage fields.
- No alternate canonical write path beside SQLite.
- No hosted or remote storage.

## Decisions

### D1. Canonical path and ownership

The canonical accepted-event store is `.scryrs/scryrs.db` relative to the current working directory. `scryrs-core` owns the path constant, SQLite open/create behavior, schema creation, compatibility validation, and insert logic. `scryrs-cli` only composes that core API.

### D2. Datastore versioning is independent of TraceEvent schema version

The datastore keeps its own schema version in `schema_meta` or an equivalent version table. The first datastore schema version is integer `1`. This version is independent of `scryrs-types::SCHEMA_VERSION`, which continues to version the TraceEvent wire format.

### D3. Event rows preserve canonical event JSON and normalized query fields

Each accepted event is stored as canonical `serde_json` re-serialization of the validated `TraceEvent`, not byte-for-byte input text. Each stored row also carries normalized values for `schema_version`, `timestamp`, `session_id`, `event_type`, `tool_name`, `subject_kind`, `subject`, `outcome`, and `failure_reason`. `subject_kind` is derived from the concrete subject-bearing event family, `subject` uses the existing `TraceEvent` subject extraction, lifecycle events keep `subject_kind` and `subject` as NULL, and `failure_reason` is populated from `Outcome::Failure.reason` when present.

### D4. Indexes are part of the contract

The schema includes indexes for subject lookup via `subject_kind` and `subject`, event-type filtering, session/timestamp ordering, and failure analysis over outcome/reason fields. The contract is explicitly shaped for hotspot-oriented reads without table scans.

### D5. Unknown datastore versions fail fast

Opening an existing database with an unsupported datastore schema version is a fatal error. `scryrs record` exits `2` on SQLite open, write, or schema-version failures and does not fall back to `.scryrs/events.jsonl`.

### D6. SQLite dependency choice

The core datastore uses `rusqlite` with the bundled SQLite build in `scryrs-core`. This keeps the storage contract local to the core crate and matches the accepted implementation direction for this task.

### D7. Verification and documentation must follow the new canonical store

Every spec, verification fixture, README, hook README, and contract note that currently treats `.scryrs/events.jsonl` as canonical persistence must be updated so JSONL remains input-only and `.scryrs/scryrs.db` becomes the persisted source of truth.

## Risks

- Bundled SQLite adds native compilation cost in Docker-backed builds.
- Verification churn is broad because existing JS fixtures and multiple docs currently read JSONL lines directly.
- Schema-version fail-fast behavior is intentionally strict and will reject unknown on-disk schemas rather than silently adapting them.
- Help, snapshots, docs, and OpenSpec requirements can drift unless they are updated together.

## Conflict Resolution

- The previous `scryrs-record-endpoint` requirement that mandated `.scryrs/events.jsonl` as the canonical store is superseded here with the SQLite contract.
- The previous cross-harness verification requirement that froze crate and spec files as unchanged is superseded for this task because the storage contract intentionally changes both implementation and capability specs.
- Raw payload auditability is resolved as canonical `serde_json` serialization of validated events, not byte-preserving input capture.
- Datastore schema versioning is resolved as an integer contract separate from the TraceEvent wire-schema version.

## Traceability

- Task `f875d4d7-fa49-4973-bff3-322f45e515fc`
- Dossier `2026-06-21T07:06:14.222Z`
- Accepted decisions `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, `1-swarm-reviewer-recommendation`
- Round outputs `round:1:agent:swarm-architect`, `round:1:agent:swarm-lead-dev`, `round:1:agent:swarm-reviewer`
- Artifact base `initial`
