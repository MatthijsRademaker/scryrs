## Why

The repository currently exposes only a scaffold trace shape (`TraceEvent { kind, subject }`) and no stable JSON contract. That is insufficient for the backlog goal of a single harness-agnostic schema shared by future hooks and the eventual record endpoint. Without a real shared contract now, each producer or consumer will drift toward ad hoc payloads, lifecycle handling, and failure reporting.

This foundation change defines the minimum durable schema needed to cover the vision-listed activity targets: files opened, searches run, symbols inspected, commands executed, docs retrieved, edits made, repeated failed lookups, plus session start and end events so downstream aggregation can recognize session boundaries.

## What Changes

1. **Replace the scaffold shared trace contract in `crates/scryrs-types`** with a versioned, serde-serializable event envelope that carries `schema_version`, `timestamp`, `session_id`, `event_type`, `tool_name`, `payload`, and `outcome`.
2. **Define typed payload families** for `SessionStart`, `SessionEnd`, `FileOpened`, `SearchRun`, `SymbolInspected`, `CommandExecuted`, `DocRetrieved`, `EditMade`, and `FailedLookup` using minimal identifier/reference fields only.
3. **Keep the wire contract harness-agnostic and simple**: RFC3339 timestamp strings, `String` session IDs, `Option<String>` tool names for lifecycle events, an explicit `Outcome` enum for success/failure, no harness-specific fields, and no stdout/stderr, document bodies, or edit diffs in payloads.
4. **Preserve existing workspace shape and compatibility**: do not create a new `scryrs-trace` crate, keep `SCHEMA_VERSION` at `0.1.0`, add only the minimal serialization dependencies, and adapt `scryrs-core` hotspot scoring to extract subjects through a helper/accessor instead of a flat `subject` field.
5. **Add verification coverage** with JSON round-trip tests for every event family and updated `scryrs-core` tests so lifecycle events are ignored for hotspot subjects while deterministic repeated-subject scoring remains intact.
6. **Do not expand scope** into `scryrs record`, storage, aggregation behavior, harness integrations, or new CLI commands.

## Impact

- **Code changes** are localized to the shared trace contract, minimal Cargo dependency updates, and the `scryrs-core` scorer/tests that currently depend on `event.subject`.
- **Shared consumers** gain one stable JSON-ready contract that can be reused across core, CLI, graph, curator, and future harness integrations without harness-specific parsing.
- **Hotspot scoring** remains deterministic, but it now depends on a subject extraction helper rather than a flat field on every event.
- **Verification** must cover both serialization behavior and workspace health through Docker-backed `scripts/test` and preferably `scripts/check`.