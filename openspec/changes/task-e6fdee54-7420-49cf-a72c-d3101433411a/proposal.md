# Trace Foundation 03 — Build Pi reference trace hook

## Why

scryrs now has both hard dependencies for a Pi trace hook — the canonical `TraceEvent` contract in `scryrs-types` and the deterministic `scryrs record` ingestion path — but lacks the actual Pi-side transport layer that bridges agent tool events into scryrs. Without this hook, Pi users cannot capture trace data for hotspot analysis, graph building, or knowledge proposals.

This task delivers a thin, transport-only reference Pi hook that consumers can install outside this repository. It observes post-execution `tool_result` events for the six named Pi tools (`read`, `bash`, `ast_grep_search`, `lsp_navigation`, `edit`, `write`), maps each onto the canonical TraceEvent schema, and delegates ingestion to `scryrs record --stdin` via a subprocess. The hook never registers scryrs as an agent-callable tool, never modifies agent-visible tool results, and fails open when scryrs is unavailable.

## What Changes

- **New `hooks/pi/` directory** containing the reference TypeScript Pi extension (`index.ts`) and companion documentation (`README.md`).
- **Transport-only hook implementation** that subscribes to Pi `tool_result` events, filters for the six named tools, constructs minimal TraceEvent payloads with the `SCHEMA_VERSION` from `scryrs-types`, serializes them as newline-delimited JSON, and pipes them to `scryrs record --stdin` via `pi.exec()`.
- **Session demarcation**: The hook generates a unique `session_id` on extension load and emits a `SessionStart` TraceEvent. `SessionEnd` is deferred to a follow-up task when Pi `session_shutdown` lifecycle handling is better understood.
- **Fail-open architecture**: The entire `scryrs record` subprocess invocation is wrapped in try-catch; any failure (missing binary, non-zero exit, timeout, I/O error) is logged via `console.error` and the handler returns `undefined`, preserving the original tool result unchanged.
- **No consumer-side configuration**: No `.pi/extensions/` wiring, `scryrs.json` manifest, or any Pi runtime configuration is committed to this repository.

## Impact

- **Affected specs**: New capability spec `pi-reference-hook` under `openspec/changes/task-e6fdee54-7420-49cf-a72c-d3101433411a/specs/pi-reference-hook/spec.md`.
- **Affected code**: `hooks/pi/index.ts` (new), `hooks/pi/README.md` (new). No Rust crate, CLI, wire format, or existing OpenSpec changes required.
- **Downstream consumers**: Pi users who copy `hooks/pi/` into their `~/.pi/agent/extensions/` or `.pi/extensions/` gain automatic trace capture with zero agent-visible behavioral change.
- **Dependencies**: Depends on Trace Foundation 02 (`scryrs record --stdin`) for the ingestion path and Trace Foundation 05 (trace-hook-contract) for the non-interference, fail-open, and session demarcation rules.