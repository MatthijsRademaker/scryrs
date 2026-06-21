## Why

`scryrs record` currently treats `.scryrs/events.jsonl` as the canonical accepted-event store. Phase 2 hotspot work needs indexed, durable evidence instead of repeated JSONL scans. This task is the intentional storage-contract break that moves canonical persistence to SQLite while preserving raw trace truth for auditability.

## What Changes

1. Replace JSONL as the canonical accepted-event store with a core-owned SQLite datastore at `.scryrs/scryrs.db`.
2. Define the datastore contract in `scryrs-core`, including schema ownership, a datastore schema version table, a `trace_events` table, and indexes for subject lookup, event-type filtering, session/timestamp ordering, and failure analysis.
3. Store each accepted `TraceEvent` as canonical event JSON plus normalized query columns for `schema_version`, `timestamp`, `session_id`, `event_type`, `tool_name`, `subject_kind`, `subject`, `outcome`, and `failure_reason`.
4. Keep `scryrs record` ingestion unchanged at the CLI surface (`--stdin` and `--file <PATH>` JSONL input), but route accepted events through the SQLite store and fail fast on open, write, or schema-version errors with no fallback to `.scryrs/events.jsonl`.
5. Update specs, tests, verification fixtures, and user/developer docs that currently describe `.scryrs/events.jsonl` as canonical persistence so they instead validate `.scryrs/scryrs.db`, while keeping JSONL documented only as an input transport format.
6. Keep scope limited to the storage foundation only: no hotspot scoring, graph/proposal behavior, query APIs, hosted storage, or legacy JSONL migration work.

## Impact

- `scryrs-core` becomes the single owner of the canonical datastore path, SQLite schema creation, version validation, and event-row extraction.
- `scryrs-cli` stays composition-only, but its record path and test override seam move from JSONL persistence semantics to SQLite persistence semantics.
- Cross-harness verification and hook documentation must stop treating `.scryrs/events.jsonl` as the canonical source of truth.
- The change is deliberately breaking for canonical storage. `.scryrs/events.jsonl` is not preserved as an alternate canonical write path, and no migration compatibility is required in this task.
