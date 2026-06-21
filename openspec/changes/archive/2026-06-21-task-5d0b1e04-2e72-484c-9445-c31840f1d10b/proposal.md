## Why

The repository already contains the substantive SQLite migration for `scryrs record`, but Task `5d0b1e04-2e72-484c-9445-c31840f1d10b` still has narrow, verifiable gaps against its acceptance criteria. Accepted events are written to `.scryrs/scryrs.db`, yet the current record path still inserts one row per autocommit, CLI integration tests do not directly prove row-level outcomes for `--stdin` and `--file`, the fatal datastore path is not covered at the CLI level, and user-facing discovery surfaces still drift on store-failure and canonical-store wording.

## What Changes

1. Tighten the SQLite writer contract so one `scryrs record` invocation persists accepted events through a single explicit transaction or equivalent batch boundary owned by `scryrs-core`, and reports success only after that commit succeeds.
2. Strengthen record-mode verification in `scryrs-cli` by reopening `.scryrs/scryrs.db` in tests to prove accepted rows are present, rejected lines never create rows, `--stdin` and `--file <PATH>` share the same canonical store, and fatal datastore failures still exit through the deterministic record error path.
3. Align discovery/documentation surfaces with the live SQLite contract by updating plain help, its snapshot, the README error-path table, and roadmap references so JSONL remains input-only and `.scryrs/scryrs.db` remains the canonical persisted store.

## Impact

- Scope stays narrow: this is a follow-up hardening pass on top of the already-landed SQLite migration, not a restart of that migration.
- `scryrs-core` remains the single owner of datastore behavior; `scryrs-cli` continues to compose the core API rather than managing raw SQLite state itself.
- `scryrs record` keeps the existing JSONL ingestion wire format, deterministic rejection behavior, and fatal/no-fallback semantics while gaining a clear invocation-level commit boundary.
- No hotspot analysis, query APIs, legacy `.scryrs/events.jsonl` compatibility, or alternate persistence paths are added by this change.
