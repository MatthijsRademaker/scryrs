## Context

`scryrs record` currently validates JSONL into `TraceEvent` values and then unconditionally writes accepted events into local `.scryrs/scryrs.db`. In parallel, the live-hotspot server contract and shared Rust types already define `ServerIngestEnvelope`, `EnvelopeEvent`, and `BatchIngestResponse`, plus the `POST /v1/trace-events/batch` endpoint that returns accepted, duplicate, and rejected acknowledgements.

The missing piece is a CLI-owned remote transport boundary. This task adds that boundary without changing the inner `TraceEvent` schema, without adding HTTP logic to hook shims, and without changing the current local default when remote configuration is absent.

## Goals / Non-Goals

**Goals**

- Add an explicit remote ingest mode for `scryrs record`.
- Preserve existing local SQLite behavior when remote mode is not configured.
- Resolve remote endpoint, repository identity, workspace identity, agent identity, and timeout deterministically.
- Reuse existing local validation before any remote submission.
- Report deterministic accepted/duplicate/rejected/failed counts in remote mode.
- Exit loudly on transport or server failures and never pretend success.
- Keep Pi and Claude integrations transport-dumb by leaving remote submission inside the CLI-owned Rust path.
- Cover mode selection, config precedence, rejection handling, duplicate handling, and transport failures with tests and updated discovery surfaces.

**Non-Goals**

- Offline retry spooling, background resend, or queued delivery.
- Dual-write behavior that writes remote-mode events into local `.scryrs/scryrs.db`.
- Authentication, authorization, TLS/certificate management, or hosted deployment work.
- Changes to the inner `TraceEvent` wire schema or existing local SQLite schema.
- Live hotspot query endpoints, SSE signals, dashboard updates, scoring changes, graph/proposal behavior, or other Phase 4 work outside remote ingest transport.
- Transport-specific HTTP code in `hooks/pi/index.ts` or Claude Code hook configuration.

## Decisions

### Decision 1: Keep remote transport in shared Rust CLI code

Remote submission belongs in a shared `scryrs-cli` transport layer rather than in hook shims. `scryrs record` is the primary contract being extended, and the same CLI-owned transport path may be reused wherever the existing hook topology needs it so that Pi and Claude integrations participate without embedding HTTP logic.

### Decision 2: Remote mode is explicit and configuration-driven

Remote mode activates only when a non-empty ingest URL is resolved. The configuration contract uses the nearest ancestor `scryrs.json` `remote` section as the file source and environment variables as the override layer: `SCRYRS_REMOTE_INGEST_URL`, `SCRYRS_REPOSITORY_ID`, `SCRYRS_WORKSPACE_ID`, `SCRYRS_AGENT_ID`, and `SCRYRS_REMOTE_TIMEOUT_MS`. If no ingest URL resolves, the command stays in local mode regardless of other remote fields.

### Decision 3: Repository identity follows the existing server contract

`repository_id` resolves from explicit configuration first and otherwise follows the normalized Git remote-origin rule already defined by the live-hotspot server contract. If remote mode is active and `repository_id`, `workspace_id`, or `agent_id` still cannot be resolved, the CLI fails before any network call.

### Decision 4: Local validation remains the gate before remote submission

Remote mode still parses newline-delimited `TraceEvent` JSON locally, skips blank lines, and emits deterministic line-based rejection diagnostics for malformed or schema-invalid input. Only accepted events are wrapped in `ServerIngestEnvelope` version `1.0.0` and submitted to the server.

### Decision 5: `producer_event_id` must be deterministic and content-addressed

For remote `scryrs record` submissions, each accepted event uses a stable `producer_event_id` derived as the SHA-256 hex digest of the canonical serialized `TraceEvent`, followed by `:` and the 1-based physical line number. This keeps duplicate replay idempotent without changing the inner event payload.

### Decision 6: Local output stays unchanged; remote output is additive and mode-specific

When remote mode is absent, `scryrs record` keeps the current local summary shape and exit semantics. In remote mode, stdout emits one JSON summary with `command`, `schemaVersion`, `transport`, `accepted`, `duplicate`, `rejected`, and `failed`. Remote duplicates are not failures; remote per-item rejections are reported deterministically and cause exit code `1`; transport and server failures are fatal exit-2 errors with no success summary.

### Decision 7: Use a small blocking HTTP client with a bounded timeout

The CLI transport should use a synchronous HTTP client rather than pulling async runtime behavior into the record path. The accepted direction is a `ureq`-backed client with a default timeout of `3000` ms and a testable transport abstraction so remote-mode tests can cover success, duplicate, rejection, and failure paths deterministically.

### Decision 8: No fallback, spool, or silent mixing in remote mode

When remote mode is active, the CLI does not open or write `.scryrs/scryrs.db`, does not queue failed submissions for later retry, and does not silently fall back to local persistence on network or server failure.

## Risks

| Risk | Mitigation |
| --- | --- |
| Config precedence or discovery bugs could activate remote mode unexpectedly. | Resolve transport mode before input/store I/O, treat missing or empty ingest URL as local mode, walk ancestors deterministically for `scryrs.json`, and cover precedence in tests. |
| Unstable `producer_event_id` derivation would break server-side idempotency. | Derive IDs from canonical event JSON plus 1-based physical line number and add replay-stability tests. |
| The hook path and record path have different failure contracts. | Keep hook shims HTTP-free, preserve fail-open hook behavior where the existing hook-facing CLI path reuses remote transport, and keep `scryrs record` itself as the loud-failure surface. |
| Adding HTTP support could bloat the synchronous CLI path or make tests brittle. | Use a small blocking client (`ureq`) and an injectable transport abstraction for deterministic tests. |

## Conflict Resolution

1. **Record-only vs shared hook participation**: refinement evidence conflicted on whether this task should touch the hook path. This synthesis resolves in favor of CLI-owned shared transport reuse where needed by the current hook topology, because the user story explicitly requires existing Pi and Claude integrations to participate without HTTP logic. The normative contract remains centered on `scryrs record`, while hook-facing behavior keeps fail-open semantics and transport stays inside Rust CLI code.
2. **Config flags vs config/env only**: refinement evidence proposed both CLI flags and a narrower config surface. This synthesis resolves to `scryrs.json` remote defaults plus environment overrides only. That satisfies the explicit-configuration requirement without adding a larger per-invocation flag surface that the task never explicitly requested.
3. **Summary shape**: refinement evidence proposed both always-on mode fields and a local-byte-identical contract. This synthesis keeps local-mode stdout unchanged and makes remote-mode fields additive (`transport`, `duplicate`, `failed`) so the current local contract remains intact.
4. **`producer_event_id` inputs**: refinement evidence differed on whether `session_id` needed to be added explicitly. Because `session_id` is already part of the serialized `TraceEvent`, the stable derivation is canonical event JSON plus 1-based physical line number.

## Traceability

- Task: `727d76f0-f4ca-4a0c-ae2f-a26accdd339b`
- Exploration dossier: `2026-06-25T04:17:26.024Z`
- Accepted decisions: `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, `1-swarm-reviewer-recommendation`
- Round outputs: `round:1:agent:swarm-architect`, `round:1:agent:swarm-lead-dev`, `round:1:agent:swarm-reviewer`
- Artifact base: `initial`