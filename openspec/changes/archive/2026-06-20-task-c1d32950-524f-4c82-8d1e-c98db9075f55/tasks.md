## 1. Define the shared trace schema

- [x] 1.1 Add minimal serialization dependencies in the workspace and wire `crates/scryrs-types` to use `serde` in production and `serde_json` for tests only.
- [x] 1.2 Replace the scaffold `TraceEvent { kind, subject }` shape in `crates/scryrs-types/src/lib.rs` with a versioned event envelope carrying `schema_version`, `timestamp`, `session_id`, `event_type`, `tool_name`, `payload`, and `outcome`.
- [x] 1.3 Define typed event/payload families for `SessionStart`, `SessionEnd`, `FileOpened`, `SearchRun`, `SymbolInspected`, `CommandExecuted`, `DocRetrieved`, `EditMade`, and `FailedLookup`.
- [x] 1.4 Keep the schema harness-agnostic, use RFC3339 timestamp strings, allow lifecycle events to omit `tool_name`, and preserve `SCHEMA_VERSION` at `0.1.0`.

## 2. Preserve hotspot scorer compatibility

- [x] 2.1 Add `TraceEvent::subject()` or an equivalent shared accessor that returns hotspot subjects for subject-bearing events and no subject for lifecycle events.
- [x] 2.2 Update `crates/scryrs-core/src/lib.rs` to use the subject accessor instead of `event.subject`.
- [x] 2.3 Update the existing `scryrs-core` unit tests to construct the new schema and preserve deterministic repeated-subject ordering.

## 3. Prove the JSON contract

- [x] 3.1 Add serde round-trip tests for every event family, including `SessionStart`, `SessionEnd`, and a failure outcome.
- [x] 3.2 Verify the serialized schema remains self-describing and carries a schema version on every event.
- [x] 3.3 Verify payloads and event types do not introduce harness-specific fields, variants, or identifiers.

## 4. Validate the workspace

- [x] 4.1 Update `Cargo.lock` with only the minimal serialization dependency changes required by the schema.
- [x] 4.2 Run Docker-backed `scripts/test`.
- [x] 4.3 Run Docker-backed `scripts/check`.
