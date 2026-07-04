## 1. Shared contract and type updates

- [x] 1.1 Update `crates/scryrs-types/src/lib.rs` so `BatchIngestResponse` reports deterministic `accepted_count`, `duplicate_count`, and `rejected_count` while preserving the existing response envelope. <!-- completed outside OpenSpec: crates/scryrs-types/src/lib.rs -->
- [x] 1.2 Extend per-item response metadata so malformed request items can be identified deterministically even when `producer_event_id` is missing. <!-- completed outside OpenSpec: crates/scryrs-types/src/lib.rs -->
- [x] 1.3 Add or update serialization tests in `crates/scryrs-types` for successful, duplicate, and rejected batch responses. <!-- completed outside OpenSpec: crates/scryrs-types/src/lib.rs -->

## 2. New server crate and central SQLite store

- [x] 2.1 Create `crates/scryrs-server/` with server config, Axum router, request handlers, and SQLite store modules. <!-- completed outside OpenSpec: crates/scryrs-server/ -->
- [x] 2.2 Register `crates/scryrs-server` in the workspace and wire the dependency through `crates/scryrs-cli/Cargo.toml` using the existing crate/dependency pattern. <!-- completed outside OpenSpec: crates/scryrs-server/ -->
- [x] 2.3 Implement server store initialization for a dedicated SQLite database/table that mirrors normalized trace-event columns and adds `repository_id`, `workspace_id`, `agent_id`, `producer_event_id`, `client_timestamp`, and `received_at`. <!-- completed outside OpenSpec: crates/scryrs-server/ -->
- [x] 2.4 Enforce a unique composite idempotency constraint on `(repository_id, workspace_id, agent_id, producer_event_id)` and return the original stored `received_at` on duplicate inserts. <!-- completed outside OpenSpec: crates/scryrs-server/ -->
- [x] 2.5 Keep the existing local `trace_events` schema and `EventStore` behavior unchanged. <!-- completed outside OpenSpec: crates/scryrs-server/ -->

## 3. Batch ingest handling

- [x] 3.1 Implement `POST /v1/trace-events/batch` in the new server crate. <!-- completed outside OpenSpec: crates/scryrs-server/ -->
- [x] 3.2 Reject malformed top-level JSON, unsupported `envelope_version`, and missing top-level identity with deterministic `400 Bad Request` diagnostics. <!-- completed outside OpenSpec: crates/scryrs-server/ -->
- [x] 3.3 Process `events` entries individually so valid siblings can be accepted when other items are malformed or schema-invalid. <!-- completed outside OpenSpec: crates/scryrs-server/ -->
- [x] 3.4 Reuse existing `TraceEvent` validation semantics and validate `client_timestamp` syntax for each item. <!-- completed outside OpenSpec: crates/scryrs-server/ -->
- [x] 3.5 Return deterministic per-item results and batch counts for accepted, idempotent, and rejected items. <!-- completed outside OpenSpec: crates/scryrs-server/ -->

## 4. CLI and discovery integration

- [x] 4.1 Add `scryrs server` to CLI dispatch and command parsing. <!-- completed outside OpenSpec: crates/scryrs-cli/src/dispatch.rs -->
- [x] 4.2 Support `--bind`, `--port`, and `--store` flags for the new server command. <!-- completed outside OpenSpec: crates/scryrs-cli/src/dispatch.rs -->
- [x] 4.3 Update help text and `--help-json` so the `server` command and its flags are discoverable. <!-- completed outside OpenSpec: crates/scryrs-cli/src/dispatch.rs -->

## 5. Verification and regressions

- [x] 5.1 Add router/integration tests for valid batches, mixed valid/invalid batches, malformed envelopes, and duplicate replay. <!-- completed outside OpenSpec: crates/scryrs-server/ -->
- [x] 5.2 Add concurrency tests proving overlapping HTTP submissions yield one stored row per composite key and deterministic accepted/idempotent outcomes. <!-- completed outside OpenSpec: crates/scryrs-server/ -->
- [x] 5.3 Add CLI tests for `scryrs server --help` and `scryrs --help-json` discovery. <!-- completed outside OpenSpec: crates/scryrs-server/ -->
- [x] 5.4 Re-run or extend local-ingest tests so `scryrs record --stdin/--file`, local `.scryrs/scryrs.db` persistence, and current hook behavior remain unchanged. <!-- completed outside OpenSpec: crates/scryrs-server/ -->
- [x] 5.5 Update targeted user-facing docs (`README.md` and any CLI surface docs touched by the command list) to describe the new server command without changing local-record guidance. <!-- completed outside OpenSpec: crates/scryrs-server/ -->
