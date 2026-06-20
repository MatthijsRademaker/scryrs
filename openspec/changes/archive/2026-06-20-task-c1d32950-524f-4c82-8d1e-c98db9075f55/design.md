## Context

The backlog task requires a stable event schema for agent activity traces so harness hooks and the future record endpoint share one contract. The current repository does not have that contract: `scryrs-types` only defines a scaffold `TraceEvent { kind, subject }`, while `scryrs-core` reads the flat `subject` field directly when scoring hotspots.

Refinement resolved the placement and shape questions. The architecture guidance says shared contracts belong in `crates/scryrs-types/src/lib.rs`, the existing scaffold already lives there, and all workspace consumers already depend on that crate. The task therefore needs a real shared schema in `scryrs-types`, not a new crate and not a move into `scryrs-core`.

## Goals

- Define a versioned, serde-serializable Rust trace event contract for agent activity.
- Cover all required activity families: session start, session end, file opened, search run, symbol inspected, command executed, doc retrieved, edit made, and failed lookup.
- Keep the schema harness-agnostic and limited to JSON as the current wire format.
- Preserve deterministic hotspot scoring by giving `scryrs-core` a stable subject-extraction path for subject-bearing events.
- Prove the contract with JSON round-trip tests for every event family and workspace verification.

## Non-Goals

- No `scryrs record` endpoint, storage, aggregation, or hotspot-analysis expansion beyond compile-required scorer adaptation.
- No harness hook implementations for read/bash/search/edit.
- No new public CLI commands or CLI surface changes.
- No new `scryrs-trace` crate.
- No privacy/redaction subsystem beyond keeping payloads minimal.

## Decisions

### D1. Put the schema in `scryrs-types`

The stable trace event schema lives in `crates/scryrs-types/src/lib.rs`. This follows the documented shared-contract boundary, keeps the existing dependency direction intact, and avoids premature workspace expansion.

### D2. Use a versioned event envelope with explicit common fields

`TraceEvent` carries `schema_version`, `timestamp`, `session_id`, `event_type`, `tool_name`, `payload`, and `outcome` on every serialized event. `timestamp` is an RFC3339 `String`, `session_id` is a `String`, and `tool_name` is `Option<String>` so lifecycle events do not need a fake tool identity.

### D3. Cover all activity families with typed payloads

The schema defines explicit event type variants for `SessionStart`, `SessionEnd`, `FileOpened`, `SearchRun`, `SymbolInspected`, `CommandExecuted`, `DocRetrieved`, `EditMade`, and `FailedLookup`. Payloads are typed per family, including a dedicated session lifecycle payload, so downstream consumers can tell which data shape they are reading.

### D4. Keep payloads minimal and harness-agnostic

Payload fields are limited to identifiers, paths, names, queries, commands, document references, edit targets, and similar lightweight references needed for hotspot subject extraction. The schema excludes harness-specific fields and does not require stdout/stderr, document contents, or edit diffs.

### D5. Keep outcome and versioning simple

Outcome is explicit on every event as `Success` or `Failure` with a generic reason/message. `SCHEMA_VERSION` remains `0.1.0`, and the event envelope carries that version so the wire contract is forward-compatible without changing the frozen CLI version constant.

### D6. Decouple `scryrs-core` from payload internals

`TraceEvent` exposes a subject-extraction helper (`subject()` or equivalent) so `scryrs-core::score_events` does not pattern-match every payload variant itself. Lifecycle events return no subject and are skipped by hotspot scoring.

### D7. Use an explicit serde tagging strategy and full round-trip tests

The wire format uses a self-describing payload encoding with explicit type information alongside payload data. Each event family, including lifecycle events and failure outcomes, must round-trip through JSON in tests.

## Risks

- **Workspace breakage from public contract changes**: `scryrs-types` is shared broadly, so all direct consumers and tests must be updated in the same change.
- **Payload scope creep into sensitive telemetry**: widening payloads would create privacy and review pressure. Keep payloads minimal and reference-only in this task.
- **Lifecycle events in hotspot scoring**: session events have no hotspot subject. The subject helper must return no subject for them so scorer behavior stays deterministic.
- **Wire-format drift**: without explicit tagging and per-family round-trip tests, future producers/consumers could serialize incompatible JSON.

## Conflict Resolution

- **Schema location**: resolved in favor of `scryrs-types`, not `scryrs-core` and not a new crate, because refinement and architecture evidence agree that shared contracts belong there.
- **Timestamp representation**: resolved as RFC3339 `String` values to avoid adding `chrono`/`time` dependencies to the shared types crate.
- **Lifecycle tool identity**: resolved by allowing `tool_name` to be optional and using dedicated lifecycle payloads rather than inventing a fake tool name.

## Traceability

- Task `c1d32950-524f-4c82-8d1e-c98db9075f55`
- Dossier `2026-06-20T12:46:51.100Z`
- Accepted decisions `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, `1-swarm-reviewer-recommendation`
- Round outputs `round:1:agent:swarm-architect`, `round:1:agent:swarm-lead-dev`, `round:1:agent:swarm-reviewer`
- Artifact base `initial`