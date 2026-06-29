## Context

The repository already documents and implements the live hotspot server contract, live accumulators, remote `scryrs record` transport, and Docker-backed verification helpers. What is missing is a high-confidence maintainer workflow that proves those pieces work together through the shipped `scryrs` binary in a headless environment. The refinement evidence converged on a focused verification suite parallel to `scripts/verify-trace-capture`: a shell entrypoint under `scripts/`, a Node fixture under `scripts/verification/`, and README documentation that explains how to run it and what it proves.

## Goals / Non-Goals

### Goals

- Add a standalone Docker-backed verification path that builds or locates the real `scryrs` binary and runs headlessly without host Rust or Node.
- Start a fresh `scryrs server`, drive remote `scryrs record --file` from two configured agent identities, and prove cumulative hotspot state for overlapping subjects.
- Prove duplicate producer replay is acknowledged idempotently and does not change accumulator score, counts, evidence, or signal history.
- Prove SSE replay/resume with persisted signal IDs and real `text/event-stream` parsing.
- Document invocation, prerequisites, scope, and the initial nightly verification recommendation.

### Non-Goals

- Changing `TraceEvent`, `ServerIngestEnvelope`, `BatchIngestResponse`, `LiveHotspotsResponse`, `HotspotSignal`, or adding a new server threshold flag.
- Adding auth, TLS, hosted deployment, Kubernetes, retry spooling, dual-write, or local fallback behavior.
- Making live dashboard smoke a required part of this task.
- Wiring this verification into a PR-gate lane or `scripts/test --full` by default.

## Decisions

### Decision 1: Use the existing Docker-backed verification pattern

Add `scripts/verify-live-hotspots` as the authoritative entrypoint and mirror the structure of `scripts/verify-trace-capture`: build the release binary with `scripts/lib/docker-verification.sh`, copy it into `.docker-fixtures/`, install any fixture dependencies in a `node:22` container, run the fixture, and return a clear pass/fail summary.

### Decision 2: Use a Node fixture that drives the real CLI and server

Add `scripts/verification/live-hotspots-e2e.mjs` as the single live-workflow fixture. It owns port selection, process lifecycle, HTTP assertions, and cleanup. It should spawn `scryrs server` on a pre-allocated nonzero port with a fresh temp store, wait for readiness by polling `GET /v1/repositories/<probe>/hotspots?window=cumulative`, and terminate the child process on every exit path.

### Decision 3: Configure remote producers with environment variables, not `scryrs init`

The fixture should configure each producer with explicit `SCRYRS_REMOTE_INGEST_URL`, `SCRYRS_REPOSITORY_ID`, `SCRYRS_WORKSPACE_ID`, and `SCRYRS_AGENT_ID` environment variables. This keeps setup ephemeral, avoids `scryrs init --mode live` restrictions inside the source checkout, and matches the existing remote-config precedence rules.

### Decision 4: Use deterministic overlapping `EditMade` fixtures and property-based assertions

The fixture should use overlapping subject-bearing `EditMade` success events because they cross the default threshold of `10` quickly without contract changes. The deterministic crossing plan is four `EditMade` events per threshold-crossing subject (weight `3` each, total `12`). To avoid unnecessary coupling to the scoring table beyond the threshold-crossing requirement, assertions should verify cumulative properties that matter to the task: successful remote summaries, multi-agent evidence in one hotspot entry, `sessionCount >= 2`, threshold crossing, and unchanged state after duplicate replay.

### Decision 5: Treat SSE as an infinite stream with an explicit two-phase protocol

The fixture should use Node's native `http`/`https` primitives, not `fetch()`, for `text/event-stream` handling. Phase 1 connects with `after=0`, parses `id:` and `data:` lines until an idle/overall timeout, records the highest signal ID, and disconnects explicitly. Phase 2 reconnects with `after=<last_seen_id>` and asserts that already-seen signals are not replayed and that only newer signal IDs, if any, are delivered.

### Decision 6: Document the suite as standalone with an initial nightly recommendation

Update `scripts/verification/README.md` to add the new entrypoint, explain prerequisites and binary/container expectations, list the behaviors under test, call out loud failure behavior, and state that the initial CI positioning is standalone/nightly rather than PR-gate.

## Risks

| Risk | Mitigation |
| --- | --- |
| Server child process leaks on failure or timeout | Track the spawned process explicitly and kill it on success, assertion failure, uncaught exception, and timeout paths. |
| Port allocation race between probe and server bind | Retry server startup with a newly allocated port on bind failure. |
| SSE verification hangs because the stream never ends | Use explicit idle and overall timeouts, abort the request after enough evidence is collected, and never wait for EOF. |
| Node/container binary mismatch | Run the fixture in Debian/glibc `node:22`, matching the existing verification pattern. |
| False failures from asserting incidental fields | Do not assert exact score tables beyond threshold crossing and do not assert `client_timestamp` semantics. |

## Conflict Resolution

1. **Dashboard smoke**: the task prompt mentioned an optional dashboard smoke path, but the refinement evidence and accepted decisions explicitly scoped it out because the current dashboard reads local artifacts rather than live server APIs.
2. **Exact values vs resilient assertions**: refinement agreed to pin the event count needed to cross the default threshold (`4` `EditMade` events per crossing subject), but not to couple the suite to exact end-state score values beyond threshold/cumulative properties.
3. **Nightly posture**: refinement agreed the suite should start as a standalone/nightly verification path and should not be promoted to a PR gate in this change.

## Traceability

| Source | How it is used |
| --- | --- |
| Task `e5d582d9-8d71-4c4c-baa6-d4ef1593d731` | Defines the multi-agent, duplicate-idempotency, SSE replay/resume, Docker-backed, headless verification scope. |
| Dossier `2026-06-29T05:03:00.404Z` | Supplies goals, non-goals, affected areas, proposal sketch, and acceptance criteria. |
| Accepted decisions `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, `1-swarm-reviewer-recommendation` | Fix the wrapper+fixture structure, env-var config, SSE lifecycle, threshold-crossing plan, dashboard exclusion, and nightly posture. |
| `scripts/verify-trace-capture` and `scripts/lib/docker-verification.sh` | Provide the authoritative Docker-backed verification pattern to mirror. |
| `scripts/verification/README.md` | Defines the existing verification documentation surface to extend. |
| Live server and record specs/docs cited in refinement | Bound the change to verification and documentation rather than runtime contract changes. |