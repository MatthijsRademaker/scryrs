## Context

The backlog task moves `scryrs` from a frozen placeholder CLI toward its first real trace-ingestion endpoint. The repository already has the shared `TraceEvent` schema in `scryrs-types` and deterministic hotspot scoring in `scryrs-core`, but it still lacks a record command, any JSONL ingestion path, and any event-store boundary. The implementation therefore needs to extend the CLI surface deliberately without leaking into hotspot analysis, promotion, or broader future command work.

Refinement converged on three required seams: `scryrs-core` owns JSONL ingestion and storage, `scryrs-cli` exposes `record` with stdin/file modes and deterministic I/O, and the discovery/docs/snapshot surfaces must evolve from the old single-command contract. The main synthesis work is pinning the unresolved contract details — default persistence location, blank-line handling, deterministic output shape, and the record-specific exit-code semantics — so implementation does not rediscover them ad hoc.

## Goals

- Expose `scryrs record --stdin` and `scryrs record --file <PATH>` as the only new public ingestion modes.
- Reuse the existing `scryrs-types::TraceEvent` wire contract instead of inventing a record-specific schema.
- Add a core-owned JSONL ingestion path that validates line-by-line, stores accepted events, accumulates structured rejections, and continues after per-line validation failures.
- Persist accepted events through a minimal append-only store seam with a concrete local default that satisfies the task's storage requirement without locking the product into SQLite.
- Return deterministic results: a single stdout summary, deterministic rejection diagnostics, and command behavior that maps cleanly to exit codes `0`, `1`, and `2`.
- Update help, help-json, docs, and tests so the public CLI surface remains truthful.

## Non-Goals

- No hotspot scoring, hotspot output, promotion logic, report generation, graph building, routing, or LLM-backed behavior in the `record` path.
- No harness-specific event fields, adapters, or IPC beyond JSONL over stdin or file.
- No expansion into the broader future CLI vision beyond the `record` endpoint needed by this task.
- No requirement for SQLite, migrations, hosted services, or any non-local infrastructure.
- No silent coercion of malformed events, synthesized required fields, or placeholder records for invalid input.

## Decisions

### D1. `scryrs-core` owns JSONL ingestion and validates the existing `TraceEvent` contract

The ingestion path lives in `crates/scryrs-core` and reads JSONL line-by-line through a shared implementation used by both CLI modes. Each non-empty line is deserialized as the existing `scryrs-types::TraceEvent`; blank or whitespace-only lines are skipped; malformed JSON and schema-invalid events are rejected without aborting later lines.

### D2. Persistence is append-only with a minimal store surface and a local default

The storage boundary stays intentionally narrow: only append accepted events and report stored counts needed by ingestion results. This task does not add query, delete, filtering, or analysis APIs. To satisfy the requirement that accepted events are actually stored while backend choice remains deferred, the default store is a local JSONL file at `.scryrs/events.jsonl` relative to the current working directory.

### D3. The CLI adds `record` by evolving the existing dispatch and test seams

`crates/scryrs-cli` must let `record` reach parsing instead of rejecting it in the root-command prefilter. The existing writer-based runner evolves to accept an input reader so `--stdin` behavior can be tested deterministically while `run()` still binds real stdin in production. `--stdin` and `--file <PATH>` are mutually exclusive; providing both or neither is a usage error with exit code `2`.

### D4. Deterministic output uses one stdout summary and structured stderr diagnostics

`record` emits exactly one JSON summary object on stdout with `command`, `schemaVersion`, `accepted`, and `rejected`. Rejected non-empty lines emit deterministic JSON diagnostics on stderr containing the 1-based physical line number, the failing field/path when available, and a reason. Accepted events do not produce per-event stdout acknowledgements; the final summary is the machine-readable success path.

### D5. Record owns an explicit 0/1/2 command contract that discovery surfaces must document

For `record`, exit code `0` means every processed non-empty line was accepted, exit code `1` means ingestion completed but one or more lines were rejected, and exit code `2` means fatal usage or I/O failure such as unreadable input. Help text, help-json, README, the CLI contract note, and snapshots must document this intentional contract evolution. The machine-readable CLI surface bumps its minor version from `0.1.0` to `0.2.0` because adding `record` is an additive surface change.

## Risks

- **Contract drift across discovery surfaces**: the repo currently documents a one-command CLI, so help text, help-json, README, and the CLI contract note can easily diverge unless they are updated together and covered by tests/snapshots.
- **Store abstraction scope creep**: adding anything beyond append/count would pull backend and analysis concerns into an ingestion-only task. Keep the store boundary minimal.
- **Diagnostic field-path fidelity**: serde/JSON parse failures may not always expose a clean field path. The contract therefore requires `field` when available, not for every rejection.
- **Exit-code confusion from older docs**: previous docs describe exit code `1` differently. The record command's explicit contract must replace that ambiguity everywhere this command is documented.

## Conflict Resolution

- **Default store location**: resolved to `.scryrs/events.jsonl` in the current working directory. This matches the task requirement that accepted events be stored, satisfies reviewer/lead-dev requests for a concrete persistent default, and is stronger than the architect's risk-only suggestion of an in-memory default that would discard events on exit.
- **Output shape**: resolved to a single JSON summary on stdout plus per-rejection JSON diagnostics on stderr. This follows the accepted architect decision, keeps stdout machine-readable, and avoids interleaving per-event acknowledgements with the summary.
- **Blank-line handling**: resolved to skipping blank or whitespace-only lines rather than rejecting them. This matches JSONL expectations and avoids counting formatting separators as bad events.
- **Exit-code collision**: resolved by giving `record` an explicit command-specific `0/1/2` contract and requiring all discovery/docs surfaces to stop describing exit code `1` as only an I/O failure for this command.

## Traceability

- Task `e0171d56-10a7-4314-b6e6-0115ab025db3`
- Dossier `2026-06-20T13:18:03.715Z`
- Accepted decisions `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, `1-swarm-reviewer-recommendation`
- Round outputs `round:1:agent:swarm-architect`, `round:1:agent:swarm-lead-dev`, `round:1:agent:swarm-reviewer`
- Artifact base `initial` and proposal-synthesis input snapshot