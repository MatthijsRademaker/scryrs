## 1. Add an explicit invocation-level SQLite batch commit

- [x] 1.1 Extend `scryrs-core` `EventStore` with a batch insert/transaction path that persists all accepted events for one record invocation through a single explicit commit boundary.
- [x] 1.2 Update `scryrs-cli` record composition to use that batch path so success is reported only after the batch commit succeeds and fatal datastore errors still exit through the existing exit-2 path.

## 2. Strengthen record-mode integration coverage

- [x] 2.1 Add a `--stdin` record test that reopens `.scryrs/scryrs.db` and asserts the expected `trace_events` rows were inserted.
- [x] 2.2 Add a `--file <PATH>` record test that reopens the same canonical store and asserts the expected rows were inserted.
- [x] 2.3 Add a mixed valid/invalid record test that queries `trace_events` directly to prove rejected non-empty lines never create rows and `.scryrs/events.jsonl` is not written as canonical output.
- [x] 2.4 Add a fatal datastore failure test that proves exit code `2`, deterministic stderr diagnostics, and no success summary for open/init/write/commit failure.

## 3. Align discovery and documentation surfaces with the SQLite contract

- [x] 3.1 Update plain `scryrs --help` record exit-code text to mention datastore store failure and refresh the corresponding snapshot.
- [x] 3.2 Update `README.md` error-path documentation so exit code `2` includes datastore store failure for `record`.
- [x] 3.3 Update `.devagent/docs/docs/roadmap.mdx` so Phase 1 references `.scryrs/scryrs.db` instead of `.scryrs/events.jsonl` as the canonical persisted store.

## 4. Re-run the targeted verification path

- [x] 4.1 Run the relevant Docker-backed test path for `scryrs-core` and `scryrs-cli` covering the new batch write behavior and record integration assertions.
- [x] 4.2 Confirm the touched help/docs surfaces no longer describe `.scryrs/events.jsonl` as the canonical persistence store.
