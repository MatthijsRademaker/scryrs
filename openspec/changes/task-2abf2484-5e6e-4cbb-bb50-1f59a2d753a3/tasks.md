## 1. Shared contract and type updates

- [ ] 1.1 Update `crates/scryrs-types/src/lib.rs` so `BatchIngestResponse` reports deterministic `accepted_count`, `duplicate_count`, and `rejected_count` while preserving the existing response envelope.
- [ ] 1.2 Extend per-item response metadata so malformed request items can be identified deterministically even when `producer_event_id` is missing.
- [ ] 1.3 Add or update serialization tests in `crates/scryrs-types` for successful, duplicate, and rejected batch responses.

## 2. New server crate and central SQLite store

- [ ] 2.1 Create `crates/scryrs-server/` with server config, Axum router, request handlers, and SQLite store modules.
- [ ] 2.2 Register `crates/scryrs-server` in the workspace and wire the dependency through `crates/scryrs-cli/Cargo.toml` using the existing crate/dependency pattern.
- [ ] 2.3 Implement server store initialization for a dedicated SQLite database/table that mirrors normalized trace-event columns and adds `repository_id`, `workspace_id`, `agent_id`, `producer_event_id`, `client_timestamp`, and `received_at`.
- [ ] 2.4 Enforce a unique composite idempotency constraint on `(repository_id, workspace_id, agent_id, producer_event_id)` and return the original stored `received_at` on duplicate inserts.
- [ ] 2.5 Keep the existing local `trace_events` schema and `EventStore` behavior unchanged.

## 3. Batch ingest handling

- [ ] 3.1 Implement `POST /v1/trace-events/batch` in the new server crate.
- [ ] 3.2 Reject malformed top-level JSON, unsupported `envelope_version`, and missing top-level identity with deterministic `400 Bad Request` diagnostics.
- [ ] 3.3 Process `events` entries individually so valid siblings can be accepted when other items are malformed or schema-invalid.
- [ ] 3.4 Reuse existing `TraceEvent` validation semantics and validate `client_timestamp` syntax for each item.
- [ ] 3.5 Return deterministic per-item results and batch counts for accepted, idempotent, and rejected items.

## 4. CLI and discovery integration

- [ ] 4.1 Add `scryrs server` to CLI dispatch and command parsing.
- [ ] 4.2 Support `--bind`, `--port`, and `--store` flags for the new server command.
- [ ] 4.3 Update help text and `--help-json` so the `server` command and its flags are discoverable.

## 5. Verification and regressions

- [ ] 5.1 Add router/integration tests for valid batches, mixed valid/invalid batches, malformed envelopes, and duplicate replay.
- [ ] 5.2 Add concurrency tests proving overlapping HTTP submissions yield one stored row per composite key and deterministic accepted/idempotent outcomes.
- [ ] 5.3 Add CLI tests for `scryrs server --help` and `scryrs --help-json` discovery.
- [ ] 5.4 Re-run or extend local-ingest tests so `scryrs record --stdin/--file`, local `.scryrs/scryrs.db` persistence, and current hook behavior remain unchanged.
- [ ] 5.5 Update targeted user-facing docs (`README.md` and any CLI surface docs touched by the command list) to describe the new server command without changing local-record guidance.
