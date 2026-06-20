## 1. Add core ingestion and persistence

- [ ] 1.1 Add the production JSON parsing dependency needed by `crates/scryrs-core` and introduce a shared JSONL ingestion API that reads line-by-line into `TraceEvent`, skips blank lines, accumulates deterministic rejections, and returns accepted/rejected counts.
- [ ] 1.2 Introduce a minimal append-only event-store seam in `crates/scryrs-core` with only the behavior needed to append accepted events and report stored counts, backed by a default local JSONL store at `.scryrs/events.jsonl`.
- [ ] 1.3 Add core tests covering all-valid input, partially-invalid input, blank-line skipping, unreadable-store/input failures, and persisted event count/content without invoking hotspot scoring.

## 2. Expose `scryrs record` in the CLI

- [ ] 2.1 Extend the root-command dispatch guard so `record` reaches parsing, and evolve the existing writer-based runner to accept an injected input reader for deterministic `--stdin` tests while `run()` continues to bind real stdin in production.
- [ ] 2.2 Implement `scryrs record --stdin` and `scryrs record --file <PATH>` as mutually exclusive modes that share the same core ingestion path and map outcomes to exit codes `0`, `1`, and `2`.
- [ ] 2.3 Emit exactly one JSON summary on stdout and deterministic per-rejection JSON diagnostics on stderr, keeping all command output on the existing writer seam so workspace `print_stdout` / `print_stderr` lints remain satisfied.

## 3. Update discovery and contract surfaces

- [ ] 3.1 Update `scryrs --help`, `scryrs --help-json`, and the committed CLI snapshots so `record` is discoverable, its summary fields and input modes are documented, and the machine-readable surface version bumps to `0.2.0`.
- [ ] 3.2 Update `README.md` and `.devagent/docs/docs/cli-v0-contract.md` so they describe the intentional evolution from the one-command placeholder surface to the ingestion-capable `record` endpoint and document the record-specific exit-code contract.

## 4. Verify the workspace

- [ ] 4.1 Add CLI tests covering stdin ingestion, file ingestion, malformed-line continuation, mutually exclusive input-mode errors, unreadable file failures, and deterministic output/exit codes.
- [ ] 4.2 Run Docker-backed `scripts/test`.
- [ ] 4.3 Run Docker-backed `scripts/check`.