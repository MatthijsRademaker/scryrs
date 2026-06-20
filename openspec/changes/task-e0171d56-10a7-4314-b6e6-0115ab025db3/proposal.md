## Why

`scryrs` currently has the shared `TraceEvent` schema and a placeholder hotspot command, but it still lacks the first real ingestion endpoint that harness hooks need. Hook authors can produce newline-delimited trace events today, yet they have no supported way to pipe those events into scryrs without inventing custom IPC or storage plumbing.

This change publishes the narrowest stable contract needed for Trace Foundation 02: a `scryrs record` endpoint that accepts JSONL trace events from stdin or a file, validates them against the existing shared schema, persists accepted events, reports deterministic counts and rejections, and stops short of any hotspot analysis or promotion behavior.

## What Changes

1. **Add `scryrs record` as the trace-ingestion endpoint** with exactly two mutually exclusive input modes: `--stdin` for streamed JSONL and `--file <PATH>` for JSONL on disk.
2. **Reuse the existing `scryrs-types::TraceEvent` JSON contract** for validation and add a `scryrs-core` ingestion path that reads input line-by-line, skips blank lines, accepts valid events, rejects malformed non-empty lines with diagnostics, and continues after per-line validation failures.
3. **Persist accepted events through a minimal core-owned append-only store seam** using a default local JSONL store at `.scryrs/events.jsonl` relative to the current working directory, while explicitly deferring SQLite and broader backend choices to later work.
4. **Pin deterministic command behavior**: a single JSON summary on stdout with `command`, `schemaVersion`, `accepted`, and `rejected`; deterministic per-rejected-line JSON diagnostics on stderr with `line`, `field` when available, and `reason`; exit code `0` when all processed events are accepted, `1` when ingestion completes with one or more rejected events, and `2` for fatal usage or I/O failures.
5. **Update CLI discovery and verification surfaces** so `record` is reachable and truthful everywhere: extend the root-command dispatch guard, evolve the CLI test seam to inject an input reader, update `--help`, `--help-json`, README, the CLI contract note, and committed snapshots, and bump the machine-readable CLI surface minor version for this additive command.
6. **Keep the task ingestion-only**: no hotspot scoring, promotion logic, graph building, routing, LLM behavior, or harness-specific IPC/fields beyond JSONL over stdin or file.

## Impact

- **Code changes** are localized to `crates/scryrs-core`, `crates/scryrs-cli`, the shared trace contract usage in `crates/scryrs-types`, and the related docs/snapshots/tests.
- **Hook developers** get a supported CLI ingestion path that does not require custom IPC and behaves the same for stdin and file inputs.
- **The CLI contract intentionally evolves** from the frozen single-command placeholder surface, so help text, help-json output, README language, and CLI contract docs must all change together.
- **Verification** must cover all-valid, partially-invalid, blank-line, mutually-exclusive-flag, unreadable-file, and persistence cases through deterministic tests plus Docker-backed `scripts/test` and `scripts/check`.