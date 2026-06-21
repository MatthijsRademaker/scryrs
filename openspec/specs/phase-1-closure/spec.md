# phase-1-closure Specification

## Purpose
TBD - created by archiving change task-2f5159c2-943a-46cb-b743-20fee7f79bec. Update Purpose after archive.

## RECONCILIATION — Phase 2 Hotspot Materialization (2026-06-21)

The Phase 2 hotspot materialization has been fully implemented and delivered. The following requirement is **superseded** by the Phase 2 contractual specs:

- **Superseded**: Requirement "Phase 2 behavior remains out of scope" and its scenario "Closure work does not add Phase 2 behavior" — these asserted that `scryrs hotspots <PATH>` must remain a placeholder. The live implementation has replaced the placeholder with real SQLite analysis, deterministic scoring, and a versioned `HotspotsReport` (schemaVersion 1.0.0).

**Canonical Phase 2 contract** (supersedes the above):
- [`openspec/specs/hotspot-report/spec.md`](../hotspot-report/spec.md) — `HotspotsReport` schema, scoring dimensions, exit codes, artifact file.
- [`openspec/specs/hotspot-verification/spec.md`](../hotspot-verification/spec.md) — E2E verification, edge-case coverage, snapshot assertions.

**Closure change traceability**: `openspec/changes/task-56573ced-fdeb-49b2-aea6-41b30f19d2bf/specs/phase-2-closure/spec.md` documents the full evidence matrix mapping code/test artifacts to Phase 2 deliverables.

All other requirements in this spec remain valid for Phase 1 closure and are not affected by this reconciliation.
## Requirements
### Requirement: Phase 1 closure evidence is published and bounded

The change SHALL publish a Phase 1 closure matrix that maps each roadmap Phase 1 deliverable to its current repository implementation status and any accepted limitation or deferred behavior. The matrix SHALL cover `scryrs record`, local append-only persistence, `scryrs.json`, Claude Code and Pi reference hooks, `scryrs init --agent`, and Docker-backed fail-open verification.

#### Scenario: Closure matrix cites the implemented Phase 1 boundary

- **WHEN** a reviewer reads the closure artifact for this change
- **THEN** each roadmap Phase 1 deliverable is marked as implemented, limited, or deferred
- **AND** each row cites concrete repository paths or artifacts
- **AND** no Phase 2 deliverable is represented as required for Phase 1 completion

### Requirement: Phase 1-facing docs and metadata reflect the shipped v0 surface

The roadmap, hook-contract docs, README, and `scryrs.json` SHALL describe the current Phase 1 boundary as already implemented in code, not forthcoming. They SHALL match the live CLI and hook behavior, including the `init` command, `surfaceVersion` `0.3.0`, the `record` summary fields `command`, `schemaVersion`, `accepted`, and `rejected`, and the accepted lifecycle limitations for Claude Code and Pi.

#### Scenario: Roadmap and hook contract stop understating current progress

- **WHEN** a reader reviews the Phase 1 status in the roadmap or trace-hook-contract docs
- **THEN** those docs no longer claim that ingestion, persistence, checked-in hooks, `scryrs init --agent`, or the root manifest are still forthcoming
- **AND** they still document that Claude Code is PreToolUse-only and that Pi captures `SessionStart` but not `SessionEnd`

#### Scenario: README and manifest match the live CLI contract

- **WHEN** a reader reviews `README.md` or `scryrs.json`
- **THEN** the README help/help-json examples show all three commands (`hotspots`, `record`, `init`) and `surfaceVersion` `0.3.0`
- **AND** the manifest describes `record` stdout as the JSON summary fields `command`, `schemaVersion`, `accepted`, and `rejected`
- **AND** neither artifact claims warning summaries or `total_event_count` fields that the CLI does not emit

### Requirement: Record ingestion enforces TraceEvent semantic invariants

The `scryrs record` ingestion path SHALL reject structurally deserializable `TraceEvent` lines when `schema_version` does not equal `scryrs-types::SCHEMA_VERSION` or when `event_type` does not match the concrete `payload.type` discriminator. These rejections SHALL follow the existing ingestion contract: deterministic diagnostics per rejected non-empty line, continued processing of later lines, accepted events still persisted, and exit code 1 when any line is rejected.

#### Scenario: Schema version mismatch is rejected

- **WHEN** a non-empty JSONL line deserializes as a `TraceEvent` but `schema_version` is not the current `SCHEMA_VERSION`
- **THEN** that line is rejected with a deterministic diagnostic naming the line and version mismatch
- **AND** later lines are still processed
- **AND** the command does not escalate the mismatch to a fatal exit 2 unless a separate usage or I/O failure occurs

#### Scenario: event_type and payload.type mismatch is rejected

- **WHEN** a non-empty JSONL line deserializes but `event_type` and `payload.type` describe different event families
- **THEN** that line is rejected with a deterministic diagnostic naming both values
- **AND** later valid lines are still accepted and persisted

### Requirement: Installer and Pi hook failures remain fail-open and honest

The Phase 1 closure work SHALL remove misleading or silent trust-boundary behavior without changing the existing fail-open boundary. The Claude Code installer collision branch SHALL not claim that a hook source was installed before the write step occurs. The Pi hook SHALL log trace-failure diagnostics when `pi.exec('scryrs', ['record', '--stdin'], ...)` throws or resolves with a non-zero exit code, while still returning `undefined` and leaving the original tool result untouched.

#### Scenario: Existing Claude settings file is refused without false installation claims

- **GIVEN** `.claude/settings.json` already exists in the target project
- **WHEN** `scryrs init --agent claude-code` is invoked
- **THEN** the command exits 2 without writing the hook file
- **AND** stderr tells the user how to configure the hook manually
- **AND** stderr does not claim that the hook source has already been installed

#### Scenario: Pi hook reports resolved non-zero record exits

- **GIVEN** the Pi hook invokes `scryrs record --stdin` and `pi.exec` resolves with a non-zero `code`
- **WHEN** the hook handles the tool result event
- **THEN** it logs a tracing failure diagnostic
- **AND** it does not throw
- **AND** it returns `undefined`
- **AND** the original Pi tool result remains unchanged

### Requirement: Phase 1 closure is verified with the existing Docker-backed workflow

The implementation SHALL be validated with the repository's existing Docker-backed verification entrypoints: `scripts/check`, `scripts/test`, and `scripts/verify-trace-capture`. The trace-capture verification SHALL continue to exercise both reference hooks and SHALL cover the Pi fail-open non-zero exit path alongside the existing missing-binary fail-open path.

#### Scenario: Closure validation passes end-to-end

- **WHEN** Phase 1 closure work is complete
- **THEN** `scripts/check`, `scripts/test`, and `scripts/verify-trace-capture` all pass in the worker environment
- **AND** the trace-capture verification still proves hook non-interference and accepted event persistence for both harnesses

#### Scenario: Pi fail-open verification covers non-zero exit

- **WHEN** the Pi verification fixture simulates a `scryrs record` subprocess that resolves with a non-zero exit code
- **THEN** the fixture observes a tracing failure diagnostic
- **AND** the hook handler still returns `undefined`
- **AND** the original tool-result payload is unchanged

### Requirement: Phase 2 behavior remains out of scope

This change SHALL NOT introduce real hotspot materialization or any later-phase surface. `scryrs hotspots <PATH>` SHALL remain the documented placeholder JSON contract, and `record` SHALL remain ingestion-only.

#### Scenario: Closure work does not add Phase 2 behavior

- **WHEN** the change is implemented
- **THEN** `scryrs hotspots <PATH>` still emits the existing placeholder JSON envelope
- **AND** no graph, route, proposal, adapter, runtime retrieval, dashboard, MCP, or LLM behavior is added
- **AND** `scryrs record` still performs validation, persistence, and diagnostics only

