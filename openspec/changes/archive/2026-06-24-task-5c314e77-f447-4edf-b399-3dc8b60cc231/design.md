## Context

The current scryrs pipeline is entirely local-only: hooks emit bare `TraceEvent` records, `scryrs record --stdin` persists to `.scryrs/scryrs.db`, and `scryrs hotspots` materializes `HotspotsReport` to stdout and `.scryrs/hotspots.json`. The dashboard reads `.scryrs/hotspots.json` as its live data source. There are no repository, workspace, agent, or producer identity fields anywhere in the pipeline, and no deduplication mechanism beyond SQLite autoincrement.

Phase 4 (Live Hotspot Server and Signals) requires a versioned remote contract that layers stable identity and idempotency rules around the existing `TraceEvent` payload, defines HTTP endpoint shapes, and documents source-of-truth separation between local and remote modes — without implementing the server, auth, or dashboard streaming.

This design resolves the open questions and non-blocking concerns raised in refinement and produces unambiguous contract artifacts for downstream implementation.

## Goals

1. Define a `ServerIngestEnvelope` wrapper carrying `repository_id`, `workspace_id`, `agent_id`, and an array of `EnvelopeEvent` items (each with `producer_event_id`, `client_timestamp`, and inner `TraceEvent`).
2. Define three HTTP endpoint shapes: `POST /v1/trace-events/batch` (batch ingest with idempotency), `GET /v1/repositories/{repository_id}/hotspots` (live hotspot query), and `GET /v1/repositories/{repository_id}/signals` (SSE signal stream, skeleton only).
3. Make deduplication explicit: composite key `(repository_id, workspace_id, agent_id, producer_event_id)` with first-writer-wins semantics.
4. Separate clock domains: `client_timestamp` on each `EnvelopeEvent` preserves producer timing; server-stamped `received_at` is authoritative for server-side ordering.
5. Define `LiveHotspotsResponse` as a separate envelope (not reuse `HotspotsReport`) to avoid leaking local-filesystem fields (`repositoryPath`, `storePath`) into remote mode.
6. Document local-only vs remote-live mode separation: remote mode is exclusive (skips local `.scryrs/scryrs.db`), activated by explicit configuration only.

## Non-Goals

- Implementing a server runtime, persistence layer, auth, or deployment topology.
- Adding dashboard live streaming, mutation APIs, or hosted-service UX.
- Modifying the canonical inner `TraceEvent` schema.
- Silently merging local SQLite state and remote live state behind one implicit code path.
- Changing hotspot scoring behavior or the `HotspotsReport` ranking contract.
- Specifying signal stream payload format beyond transport choice (SSE).

## Decisions

### Decision 1: ServerIngestEnvelope wrapper structure

**Choice:** Define a new top-level `ServerIngestEnvelope` struct in `scryrs-types` with `envelope_version` ("1.0.0"), `repository_id`, `workspace_id`, `agent_id`, and `events: Vec<EnvelopeEvent>`. Each `EnvelopeEvent` carries `producer_event_id`, `client_timestamp`, and `event: TraceEvent`.

**Rationale:** This preserves the inner `TraceEvent` schema untouched, keeping all existing hooks and the local `scryrs record` pipeline working without change. Identity fields describe submission context, not individual event semantics. The envelope is independently versionable from the inner trace schema.

**Evidence:** Unanimous across all three refinement round outputs. `scryrs-types/src/lib.rs` has no identity fields on `TraceEvent`. The roadmap Phase 4 explicitly calls for "Versioned server ingest envelope around existing TraceEvent data."

### Decision 2: Repository ID derivation

**Choice:** `repository_id` is derived from the Git remote origin URL, normalized (lowercased, trailing-slash-stripped, protocol-agnostic). For repositories without a remote, the producer must supply an explicit `repository_id` via `scryrs.json` configuration or environment variable; omitting it is a validation error in remote mode.

**Rationale:** The current `HotspotsReport.repositoryPath` uses absolute filesystem paths that are unstable across containers and clones — they cannot serve as stable identity. Git remote origin is the most stable, widely-available canonical identity for a repository.

**Evidence:** All three reviewers identified this gap. Lead-dev explicitly recommended Git remote URL normalization with a documented fallback for repos without remotes. `scryrs-types/src/lib.rs` lines 108-117 show `HotspotsReport` uses absolute `repositoryPath`.

### Decision 3: Deduplication key scope

**Choice:** Composite key `(repository_id, workspace_id, agent_id, producer_event_id)` with first-writer-wins semantics. Duplicate submissions return `EventAck.status = "idempotent"` with the original `received_at` and do not increment hotspot scores.

**Rationale:** Scoped uniqueness within `(repository_id, workspace_id, agent_id)` allows agents to use simpler IDs (e.g., monotonic counters per agent) without global coordination. First-writer-wins is the simplest idempotency model.

**Evidence:** All three reviewers converged on the 4-tuple composite key. The task scenario explicitly states "Duplicate submissions are harmless" and "hotspot scores do not double count the duplicate." Current `store.rs` has no dedup mechanism.

### Decision 4: Clock semantics

**Choice:** Each `EnvelopeEvent` carries a `client_timestamp` (RFC 3339, producer's wall clock at submission). The inner `TraceEvent.timestamp` is left unchanged. The server stamps `received_at` independently. `received_at` is authoritative for server-side ordering and audit; `client_timestamp` is diagnostic-only. Producers with client_timestamps far outside a configurable skew window SHOULD be accepted but flagged.

**Rationale:** Adding a separate `client_timestamp` alongside `TraceEvent` avoids reinterpreting the inner event's timestamp field, which is used for local SQLite ordering and `firstSeen`/`lastSeen` computation in hotspots. The server must not trust container-local wall clocks for ordering.

**Evidence:** Lead-dev and architect both recommended adding `client_timestamp` alongside `TraceEvent` rather than reinterpreting the inner field. Task technical notes require "idempotency and clock-field rules (`client_timestamp` vs server `received_at`)".

### Decision 5: Live hotspot query response envelope

**Choice:** Define a new `LiveHotspotsResponse` envelope with `schemaVersion`, `repository_id`, `cursor`, `generatedAt`, and `entries: Vec<HotspotEntry>`. Do not reuse `HotspotsReport`, which carries `repositoryPath` and `storePath` as absolute filesystem paths that are meaningless in remote mode.

**Rationale:** `HotspotsReport` was designed for local batch artifact output and leaks local-filesystem identifiers into its envelope. A separate envelope keeps the remote contract clean and independently versionable. Hotspot scoring and `HotspotEntry` shape are reused.

**Evidence:** Lead-dev explicitly recommended this separation. Architect raised the question of reuse vs new envelope. Task non-goals include "silently merge local SQLite state and remote live state."

### Decision 6: Signal stream scope

**Choice:** Define `GET /v1/repositories/{repository_id}/signals` as an SSE endpoint with a skeletal contract (media type `text/event-stream`, `id:` field for cursor position, `data:` field as JSON). Do not specify the signal payload schema beyond noting it carries hotspot delta events; implementation tasks will define the exact signal shape.

**Rationale:** The signal stream is in scope per the task technical notes and roadmap Phase 4, but the reviewer raised valid scope-creep concerns. A skeleton SSE contract (format choice only, no signal schema) satisfies the "endpoint shapes before implementation" requirement without creating implementation pressure for streaming server infrastructure.

**Evidence:** Reviewer flagged signal stream as scope-creep risk. Task says "Specify HTTP endpoints before implementation" and "Do not implement the server in this task." Roadmap Phase 4 lists SSE signal stream.

### Decision 7: Local-only vs remote coexistence

**Choice:** Remote mode is **exclusive**, not additive. When remote ingest is configured (via `scryrs.json` or environment variable), the CLI skips local `.scryrs/scryrs.db` storage entirely. There is no dual-write mode. Local-only remains the default; remote mode requires explicit opt-in.

**Rationale:** Dual-write creates ambiguous source-of-truth and complexity around state reconciliation. Exclusive modes keep the contract simple and enforce the separation the task scenarios require.

**Evidence:** Reviewer identified coexistence as a non-blocking design gap. Dossier non-goal: "Silently merge local SQLite state and remote live state behind one implicit code path." Task scenario: "server state is the source of truth" when remote is configured.

### Decision 8: Workspace ID semantics

**Choice:** `workspace_id` identifies a logical hook-installation scope — a stable identifier for a particular agent installation on a particular working copy. It persists across harness restarts within the same working copy but is distinct per agent installation (different agent_id + same checkout = different workspace_id). The contract documents the field as required but does not mandate a specific derivation rule in this task; derivation rules are deferred to implementation tasks.

**Rationale:** The dossier identifies workspace semantics as ambiguous. The contract must name and type the field but can defer exact derivation to later implementation work without blocking the contract specification.

**Evidence:** Both architect and reviewer flagged this as non-blocking. Lead-dev suggested "logical agent workspace installation scope" with agent_id incorporation. Dossier open questions raise the ambiguity explicitly.

## Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| Repository ID unavailable for new repos without Git remote | Medium | Contract requires explicit `repository_id` in `scryrs.json` or env var for such repos; validation error if omitted |
| Clock skew between producer and server | Medium | `received_at` is authoritative for ordering; `client_timestamp` is diagnostic-only; far-future/past timestamps flagged in server logs |
| Workspace ID collision if derived solely from filesystem path | Low | Contract documents recommended derivation incorporating agent_id or random token; exact derivation deferred to implementation |
| Signal stream endpoint invites scope creep into server implementation | Medium | Skeleton SSE contract only (format choice, no signal payload schema); implementation of signal schema deferred to follow-up task |
| New types in scryrs-types create dependency weight for all crates | Low | Types are small (4 new structs) and additive; no existing types are modified; feature-gating possible in future |

## Traceability

- **Task:** 5c314e77-f447-4edf-b399-3dc8b60cc231 (Live Hotspot Foundation 01)
- **Dossier:** 2026-06-24T06:43:07.260Z
- **Decisions:** `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, `1-swarm-reviewer-recommendation`
- **Round outputs:** architect (round 1), lead-dev (round 1), reviewer (round 1)
- **Artifact snapshot:** `openspec/changes/task-5c314e77-f447-4edf-b399-3dc8b60cc231` @ initial
- **Consulted sources:** `crates/scryrs-types/src/lib.rs`, `openspec/specs/hotspot-report/spec.md`, `openspec/specs/trace-hook-contract/spec.md`, `openspec/specs/scryrs-record-endpoint/spec.md`, `.devagent/docs/docs/trace-hook-contract.md`, `.devagent/docs/docs/roadmap.mdx`, `.devagent/docs/docs/architecture.mdx`