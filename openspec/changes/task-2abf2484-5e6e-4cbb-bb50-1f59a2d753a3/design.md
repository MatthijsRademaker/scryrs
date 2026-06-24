## Context

scryrs already supports local JSONL ingestion into `.scryrs/scryrs.db` through `scryrs record`, shared server-envelope/ack types in `scryrs-types`, deterministic validation patterns in `scryrs-core`, and an Axum/Tokio server pattern in `scryrs-dashboard`. What it does not have is a long-lived central ingest runtime that owns SQLite writes for multi-agent use. This task fills only that gap: central HTTP ingest, deterministic validation, server-owned persistence, and idempotency, while leaving local-only flows untouched.

## Goals / Non-Goals

**Goals**

- Ship `scryrs server` as the runnable server surface for `POST /v1/trace-events/batch`.
- Persist accepted inner `TraceEvent` values into a server-owned SQLite datastore using existing validation and normalized storage semantics where practical.
- Enforce first-writer-wins idempotency on `(repository_id, workspace_id, agent_id, producer_event_id)`.
- Return deterministic accepted, duplicate, and rejected counts plus per-item diagnostics for mixed batches.
- Ensure concurrent clients submit through HTTP while one server process owns SQLite writes.
- Keep `scryrs record --stdin/--file` and local `.scryrs/scryrs.db` behavior unchanged.

**Non-Goals**

- Authentication, authorization, TLS, hosted deployment, or multi-tenant hardening.
- Live hotspot query endpoints, dashboard streaming, or SSE `/signals`.
- Replacing local-only `scryrs record` or removing the current local SQLite workflow.
- Migrating installed hooks to remote mode or changing default hook transport behavior.
- Changing the inner `TraceEvent` schema or the local `HotspotsReport` schema.

## Decisions

### Decision 1: Add a dedicated `scryrs-server` crate and wire it in as `scryrs server`

The server foundation lives in a new `crates/scryrs-server/` crate with its own Axum router, config, and store implementation. `crates/scryrs-cli` exposes it as `scryrs server` following the existing crate/dependency wiring pattern already used for optional command surfaces. The command should expose bind, port, and store-path configuration, with defaults of `127.0.0.1`, `8081`, and `.scryrs/server.db` so it can coexist with the dashboard default port.

### Decision 2: Use a separate server-owned SQLite store/table and do not touch local `trace_events`

The server store is intentionally separate from the existing local `trace_events` schema. It mirrors the normalized columns already extracted by `EventStore`, adds `repository_id`, `workspace_id`, `agent_id`, `producer_event_id`, `client_timestamp`, and row-level `received_at`, and enforces a unique composite key on `(repository_id, workspace_id, agent_id, producer_event_id)`. This preserves zero-regression local behavior and avoids forcing `TraceQuery` or local readers to absorb identity/idempotency columns they do not need.

### Decision 3: Validate in two layers so malformed siblings do not poison the whole batch

Top-level request parsing remains strict: malformed JSON bodies, unsupported `envelope_version`, or missing top-level identity fields fail with deterministic `400 Bad Request` diagnostics. Once the envelope is structurally valid, the server processes `events` entries one by one from raw JSON values, applies deterministic field/path diagnostics during per-item decoding, validates `client_timestamp` syntax, reuses `TraceEvent::validate()` for the inner event, and accepts valid siblings while rejecting invalid ones.

### Decision 4: Extend the response contract additively for deterministic counts and malformed-item identity

`BatchIngestResponse` gains additive `accepted_count` and `rejected_count` fields while preserving `duplicate_count` and the existing response envelope. `received_count` remains the count of accepted plus idempotent items for compatibility with the prior contract. Per-item results carry request-order identity via an `index` field; rejected items may omit `producer_event_id` only when the request item could not supply one. Duplicate acknowledgments must return the original stored `received_at`, which requires persisting `received_at` at row level and reading it back on conflict.

### Decision 5: Preserve all local-only ingestion behavior

This change does not alter `scryrs record`, local `.scryrs/scryrs.db` writes, current hook direct-write behavior, or dashboard read behavior. Central ingest is additive. Clients use HTTP batches against `scryrs server`; they are not given a shared SQLite path and do not open the server-owned database file directly.

### Decision 6: Prove the foundation with contract, integration, and concurrency tests

Implementation must add tests for shared type serialization, top-level `400` failures, mixed valid/invalid batches, duplicate replay returning the original `received_at`, concurrent duplicate submissions yielding one stored row, CLI help/help-json discovery, and regression coverage proving local `scryrs record` behavior is unchanged.

## Risks

| Risk | Mitigation |
|------|------------|
| Server store schema diverges from the local query schema. | Keep the server schema explicitly separate and mirror the normalized columns needed for future scoring so later live-query work has a clean base. |
| Mixed-batch validation can become nondeterministic if full-envelope typed deserialization is used. | Keep the two-layer validation split: strict top-level decode, raw-value per-item iteration, and deterministic field/path diagnostics. |
| Duplicate acknowledgments could return the current request time instead of the original receipt time. | Persist `received_at` per stored row and read it back on unique-key conflicts. |
| Concurrent tests can give false confidence if they only exercise sequential inserts. | Add integration tests that issue overlapping HTTP submissions and assert exactly one accepted row per composite key. |
| The new command can drift from discovery surfaces. | Update dispatch, help text, and `--help-json` together and cover them with tests. |

## Conflict Resolution

1. **Response counts**: accepted reviewer concern that the existing response lacked the required counts. Resolve by adding `accepted_count` and `rejected_count` additively while keeping `duplicate_count` and the existing envelope.
2. **Server schema strategy**: resolve the separate-vs-extended schema question in favor of a dedicated server store/table. This preserves existing local `trace_events` readers and satisfies the zero-regression requirement for `scryrs record`.
3. **Duplicate `received_at` semantics**: resolve by storing `received_at` at row level and returning the original persisted timestamp for idempotent duplicates.
4. **Rejected items missing `producer_event_id`**: resolve by adding deterministic positional identity in per-item results so malformed request items can still be rejected without aborting valid siblings.

## Traceability

- Task: `2abf2484-5e6e-4cbb-bb50-1f59a2d753a3`
- Exploration dossier: `2026-06-24T19:55:15.364Z`
- Accepted decisions: `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, `1-swarm-reviewer-recommendation`
- Round outputs: `round:1:agent:swarm-architect`, `round:1:agent:swarm-lead-dev`, `round:1:agent:swarm-reviewer`
- Artifact base: `initial`
