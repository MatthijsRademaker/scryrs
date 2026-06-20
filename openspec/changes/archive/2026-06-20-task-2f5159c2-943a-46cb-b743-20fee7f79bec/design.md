## Context

The repository already implements the roadmap's Phase 1 deterministic proxy-capture boundary in code: `scryrs record`, local `.scryrs/events.jsonl` persistence, root `scryrs.json`, reference Claude Code and Pi hooks, `scryrs init --agent`, and Docker-backed cross-harness verification all exist. The remaining work is closure rather than new product surface: authoritative docs and metadata still contradict the code, and a small semantic-validation gap plus two trust-boundary bugs prevent an honest declaration that Phase 1 is done.

### Phase 1 closure matrix

| Phase 1 deliverable | Current evidence | Current status | Gap this change closes |
| --- | --- | --- | --- |
| `scryrs record` ingestion | `crates/scryrs-cli/src/lib.rs`, `crates/scryrs-core/src/ingestion.rs` | Implemented | Reject semantically invalid `TraceEvent` lines where `schema_version` or `event_type`/`payload.type` disagree with the documented contract |
| Local append-only event store | `crates/scryrs-core/src/store.rs` | Implemented | Keep ingestion-only boundary; no new store scope |
| Root `scryrs.json` hook manifest | `scryrs.json` | Implemented with stale contract text | Sync `record.outputContract` and keep harness/lifecycle metadata accurate |
| Claude Code reference hook | `hooks/claude-code/` | Implemented with accepted limitations | Keep limitations documented accurately; no lifecycle expansion |
| Pi reference hook | `hooks/pi/` | Implemented with accepted limitations | Log resolved non-zero `scryrs record` exits; keep `SessionEnd` deferred |
| `scryrs init --agent` installer | `crates/scryrs-cli/src/init.rs` | Implemented | Fix misleading `.claude/settings.json` collision wording without changing exit or non-mutation behavior |
| Docker-backed fail-open verification | `scripts/verify-trace-capture`, `scripts/verification/README.md` | Implemented | Re-run and extend coverage for the Pi non-zero exit path |
| Roadmap/docs/README accuracy | `.devagent/docs/docs/roadmap.mdx`, `.devagent/docs/docs/trace-hook-contract.md`, `README.md` | Stale | Update current-state, hook-contract, and CLI examples to match the shipped v0 surface |

## Goals / Non-Goals

### Goals

- Align Phase 1-facing docs and metadata with the code that already exists.
- Enforce the documented `TraceEvent` invariants at ingestion time for `schema_version` equality and `event_type`/`payload.type` coherence.
- Remove misleading or silent trust-boundary behavior in `init` and the Pi hook while preserving fail-open non-interference.
- Verify Phase 1 closure using the existing Docker-backed check/test/trace-capture workflow.

### Non-Goals

- No Phase 2 hotspot materialization; `scryrs hotspots <PATH>` remains the placeholder JSON contract.
- No graph, route, proposal, docs-adapter, runtime retrieval, dashboard, MCP, or LLM work.
- No new harnesses beyond Claude Code and Pi.
- No automatic merge or edit of consumer `.claude/settings.json` or other user config beyond the current installer boundary.
- No upgrade of accepted lifecycle limitations into new Phase 1 scope; Claude Code remains PreToolUse-only and Pi `SessionEnd` remains deferred.

## Decisions

### D1: Treat this as a closure and hardening change, not a new feature phase

**Decision:** Scope the work to four closure workstreams only: docs/metadata sync, semantic ingestion validation, installer/Pi hook trust-boundary cleanup, and Docker-backed verification.

**Rationale:** Refinement found that the Phase 1 deliverables already exist in code. The remaining blockers are contradictions between code and docs plus two bounded behavior gaps. No Phase 2 hotspot work is needed to close Phase 1.

### D2: Sync only the surfaces proven stale by the refinement evidence

**Decision:** Update `.devagent/docs/docs/roadmap.mdx`, `.devagent/docs/docs/trace-hook-contract.md`, `README.md`, and `scryrs.json` so they match the live CLI and hook behavior, including the `init` command and current `surfaceVersion` `0.3.0` surface.

**Rationale:** These are the surfaces explicitly identified as stale by the dossier and validated round outputs. The work should fix known contradictions, not broaden into unrelated doc cleanup.

### D3: Semantic `TraceEvent` validation stays inside ingestion and follows the existing `record` rejection contract

**Decision:** Add a small post-deserialization validation step in `crates/scryrs-core/src/ingestion.rs` that rejects events when `schema_version != SCHEMA_VERSION` or `event_type` does not match `payload.type`.

**Rationale:** The gap is real, but it is narrow. Keeping validation local to ingestion avoids new crates or validation frameworks and preserves the established `record` behavior: deterministic per-line rejection diagnostics, later-line continuation, accepted-event persistence, and exit code 1 for partial rejection.

### D4: The installer collision fix is wording-only

**Decision:** Correct the `.claude/settings.json` collision message so it no longer claims the hook source has already been installed before the write path runs, while preserving exit code 2 and the no-mutation behavior.

**Rationale:** The trust issue is the false statement, not the installer boundary itself. The change must remain surgical.

### D5: Pi fail-open logging must cover both thrown and resolved non-zero exec failures

**Decision:** Inspect the resolved `ExecResult.code` from `pi.exec('scryrs', ['record', '--stdin'], ...)` and log a trace failure when it is non-zero, in addition to the existing thrown-error path.

**Rationale:** A resolved non-zero exit is still a trace failure under the documented fail-open contract. Logging it closes the silent-gap path without changing tool behavior.

### D6: Existing verification entrypoints remain the source of truth

**Decision:** Validate the change with `scripts/check`, `scripts/test`, and `scripts/verify-trace-capture`, extending existing fixtures where needed instead of adding new verification harnesses or changing default precommit/CI wiring.

**Rationale:** The repository already has Docker-backed verification that fits the worker environment. Refinement raised the precommit/CI question but did not accept any scope expansion there.

### D7: Accepted lifecycle limitations stay documented, not widened

**Decision:** Keep the current limitations explicit in docs and manifest surfaces: Claude Code remains PreToolUse-only with no lifecycle events, and Pi captures `SessionStart` but not `SessionEnd`.

**Rationale:** Refinement treated these as accepted Phase 1 limitations, not blockers. The closure change must avoid overstating support while also avoiding unplanned lifecycle work.

## Conflict Resolution

- **Lifecycle completeness:** Not a blocker for this change. Preserve current limitations in docs instead of expanding hook scope.
- **`scripts/verify-trace-capture` default wiring:** Out of scope. The command remains required validation for this change, but no new default precommit/CI requirement is introduced.
- **Claude settings management:** `scryrs init --agent claude-code` continues to refuse `settings.json` collisions rather than merging them.

## Risks

| Risk | Why it matters | Mitigation |
| --- | --- | --- |
| New semantic validation can break existing tests or fixtures that currently deserialize but violate invariants. | Partial rejection behavior could change unexpectedly if fixtures are not audited. | Audit all record tests and verification inputs; add explicit rejection coverage for version and type mismatches. |
| Docs sync could overstate support by removing real harness limits. | The repository would claim a more complete Phase 1 than the code actually provides. | Preserve Claude PreToolUse-only and Pi `SessionEnd` limitations in roadmap, hook-contract, README, and manifest text. |
| Pi non-zero exit handling depends on the actual `ExecResult` shape. | Logging the wrong branch could miss failures or throw inside the hook. | Confirm the resolved result shape already modeled by the verification fake API and keep the new handling local to `hooks/pi/index.ts`. |
| Installer message changes can accidentally alter exit semantics or file-write order. | That would expand the change beyond the intended trust-boundary fix. | Limit the change to the collision message and its tests; keep exit code 2 and no-write behavior unchanged. |

## Traceability

- **Task:** `2f5159c2-943a-46cb-b743-20fee7f79bec`
- **Dossier:** `2026-06-20T19:44:23.871Z`
- **Accepted decisions:** `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, `1-swarm-reviewer-recommendation`
- **Validated round outputs:** `round:1:agent:swarm-architect`, `round:1:agent:swarm-lead-dev`, `round:1:agent:swarm-reviewer`
- **Artifact base:** `openspec/changes/task-2f5159c2-943a-46cb-b743-20fee7f79bec` @ `initial`