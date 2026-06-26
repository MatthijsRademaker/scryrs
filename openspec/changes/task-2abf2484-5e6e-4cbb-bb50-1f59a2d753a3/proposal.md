## Why

scryrs already has local JSONL ingestion into `.scryrs/scryrs.db`, shared `ServerIngestEnvelope`/`BatchIngestResponse` types, and an Axum-based local server pattern, but it still lacks the long-lived central ingest process required for multi-agent and multi-container use. Direct SQLite writes from multiple agents are unsafe and prevent first-writer-wins idempotency across retries. This change delivers the narrow server-side foundation: one `scryrs` process accepts versioned trace-event batches, validates them deterministically, persists accepted events into a server-owned SQLite store, and leaves existing local `scryrs record` behavior intact.

## What Changes

1. **Add a central ingest server surface**: introduce `scryrs server` as the long-lived runtime/API surface for `POST /v1/trace-events/batch`, with configurable bind address, port, and server-owned SQLite store path.
2. **Add a dedicated server crate and store**: implement a `scryrs-server` crate with an Axum router and a dedicated SQLite database/table for server ingest. The server store mirrors the normalized trace-event columns needed for future scoring, adds `repository_id`, `workspace_id`, `agent_id`, `producer_event_id`, `client_timestamp`, and `received_at`, and enforces a unique composite idempotency key. The existing local `trace_events` table is not modified.
3. **Extend the ingest response and validation contract additively**: update `scryrs-types` so batch responses report deterministic `accepted_count`, `duplicate_count`, and `rejected_count`, while preserving the existing response envelope. Per-item results must remain deterministic for mixed batches, including malformed items that lack `producer_event_id`, by carrying positional identity alongside rejection diagnostics.
4. **Implement two-layer validation and partial-batch handling**: top-level envelope failures (malformed JSON, unsupported `envelope_version`, missing top-level identity) return `400 Bad Request`; otherwise the server iterates `events` entries individually, reuses existing `TraceEvent` validation semantics, and accepts valid siblings while rejecting invalid ones with indexed diagnostics.
5. **Preserve current local-only flows**: keep `scryrs record --stdin/--file`, local `.scryrs/scryrs.db` writes, existing hook direct-write behavior, and dashboard behavior unchanged. This task does not add auth, hosted deployment, remote hook mode, live hotspot queries, or SSE streaming.
6. **Cover the foundation with tests and discovery updates**: add serialization and contract tests, router/store integration tests, duplicate and idempotency tests, concurrent client tests, and CLI help/help-json updates for the new `server` command.

## Impact

- **New workspace code**: add `crates/scryrs-server/` for the Axum router, request handling, config, and dedicated SQLite store.
- **Shared contract changes**: update `crates/scryrs-types/src/lib.rs` for additive batch-response/result metadata and corresponding tests.
- **CLI integration**: update `crates/scryrs-cli` dispatch, command parsing, help text, and `--help-json` so `scryrs server` is discoverable and wired through the existing crate/dependency pattern.
- **No local schema regression**: do not change the existing local `trace_events` schema or the current `scryrs record` contract; the server owns a separate store/table and returns original `received_at` for duplicate acknowledgments.
- **Spec scope**: add a new `central-trace-ingest-server` capability spec and modify `live-hotspot-server-contract` so the accepted/duplicate/rejected response contract and server-foundation scope match implementation.
