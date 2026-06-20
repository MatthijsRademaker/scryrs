## Context

The scryrs project has two reference harness hooks that demonstrate trace capture integration for different agent runtimes:

1. **Claude Code hook** (`hooks/claude-code/scryrs-hook.mjs`) — a shell-oriented PreToolUse hook that intercepts nine Claude Code tools, maps them to canonical `TraceEvent` JSON, and pipes to `scryrs record --stdin`. It unconditionally emits `outcome: Success` (pre-execution — real outcome unknown) and fails open with timestamped warnings to `.scryrs/hooks/claude-code-warnings.log`.

2. **Pi hook** (`hooks/pi/index.ts`) — a plugin-oriented `tool_result` (post-execution) hook that intercepts six Pi tools, maps them to `TraceEvent` JSON, and delegates to `scryrs record --stdin` via `pi.exec()`. It is the only hook that can produce failure metadata: failing `lsp_navigation` (`event.isError === true`) emits `FailedLookup` with `outcome: { result: 'Failure', reason: 'Tool execution error' }`. It emits `SessionStart` on Pi's `session_start` lifecycle event.

Both hooks report being non-interfering, but this claim has never been proven end-to-end against real `scryrs record --stdin` persistence. The Claude Code hook tests (`scripts/hook-test-runner.mjs`) use a fake shell-script scryrs that merely cats stdin to a file — they prove JSON shaping and fail-open but never exercise the real binary. The Pi hook has no automated tests.

The worker runtime (`.pi/rules/runtime-environment.md`) has no host Node.js — all verification must run through Docker-backed scripts. The existing Docker infrastructure (`scripts/lib/docker-verification.sh`) supports only Rust containers via `run_rust()`; no Node.js container helper exists, even though `scripts/.versions` lists `NODE_IMAGE=node:22-alpine`.

## Goals / Non-Goals

### Goals

1. Provide a repeatable Docker-backed verification entrypoint (`scripts/verify-trace-capture`) runnable in the worker environment that exercises both reference hooks against real `scryrs record --stdin`.
2. Prove that successful trace capture persists events to `.scryrs/events.jsonl` with canonical `TraceEvent` envelope shape (all required fields present, correct payload `type` tag, correct event families for tool mappings).
3. Prove that agent-visible outputs are unchanged: Claude Code path → no stdout/stderr/exit code alteration; Pi path → original `tool_result` event payload passed through unchanged, handler returns `undefined`.
4. Prove failure propagation on the Pi path: a failing `lsp_navigation` produces a `FailedLookup` event with `outcome.result: 'Failure'` while the original error-state payload is preserved unmodified.
5. Prove fail-open behavior on both harnesses: when `scryrs` is missing or cannot execute, tools complete normally (Claude Code returns `{continue: true}`, Pi returns `undefined`) and no tool-output corruption occurs.

### Non-Goals

1. Do not add new hook capabilities, new event families, or new harness integrations beyond Claude Code and Pi.
2. Do not redesign `scryrs record`, the append-only store, or the canonical `TraceEvent` schema.
3. Do not turn scryrs into a tool proxy, wrapper, or business-tool surface.
4. Do not edit canonical OpenSpec specs as part of this verification task (the new spec is proposed, not committed to the canonical registry).
5. Do not delete or replace `scripts/hook-test` — it remains as a fast, fake-scryrs development feedback loop.

## Decisions

### Decision 1: Sequential Rust → Node Docker containers orchestrated by one bash entrypoint

**Choice**: Build scryrs via `cargo build --release` in a Rust container (existing `run_rust`), then run hook-driver verification in a Node container (new `run_node`). Do NOT build a combined Rust+Node image.

**Rationale**: The existing `run_rust` pattern (uid/gid mapping, volume mounts, cargo caching) is proven. Adding a `run_node` helper following the same pattern reuses infrastructure without introducing a new multi-stage Dockerfile. Sequential container runs keep each step simple and cacheable independently.

**Evidence**: `scripts/lib/docker-verification.sh` already supports `run_rust`, `ensure_volume`, and `_pull_image_if_missing`. `scripts/.versions` already lists `NODE_IMAGE=node:22-alpine`. Lead-dev recommendation explicitly endorsed sequential containers.

### Decision 2: Pi fixture loads `hooks/pi/index.ts` via `tsx` against a fake `ExtensionAPI`

**Choice**: The Pi verification fixture installs `tsx` transiently in a temp directory inside the Node container and loads the actual TypeScript hook source (`hooks/pi/index.ts`) against a fake `ExtensionAPI` object that implements `on()`, `exec()`, and event handler signatures matching the ambient declarations in `hooks/pi/ambient.d.ts`.

**Rationale**: This proves the published hook code works end-to-end without requiring a real Pi runtime. `tsx` is a lightweight ESM-compatible TypeScript loader that requires no build step. The fake `ExtensionAPI` is deterministic and controllable — it can simulate both success and failure tool results. The alternative (extracting event logic to a plain JS module) would be refactoring, which the non-goals prohibit.

**Evidence**: `hooks/pi/ambient.d.ts` defines the exact API surface needed. Lead-dev recommendation endorsed `tsx`-based loading.

### Decision 3: Pi failure assertion checks `outcome.result: 'Failure'` but not exact `reason` string

**Choice**: The Pi failure-assertion test verifies `outcome.result === 'Failure'` and `event_type === 'FailedLookup'` with correct payload contents. It does NOT require the exact reason string `'Tool execution error'` to match.

**Rationale**: The canonical `Outcome::Failure { reason: Option<String> }` type makes reason optional — it's not a required field. Locking to the hardcoded string `'Tool execution error'` would create a brittle test that breaks on any message refinement in the hook. The dossier's open question and the reviewer's Q4 both lean toward lenient assertion.

**Evidence**: `crates/scryrs-types/src/lib.rs` defines `Outcome::Failure { reason: Option<String> }`. The dossier open question: "Should the Pi failure assertion lock the exact failure reason string... or only require outcome.result: Failure?". Reviewer Q4: "The former is brittle against message changes; the latter is more robust."

### Decision 4: Keep existing `scripts/hook-test` as a separate fast-path entrypoint

**Choice**: The new `scripts/verify-trace-capture` is a separate entrypoint from `scripts/hook-test`. The existing hook-test (fake scryrs, requires host Node) is preserved for rapid development feedback on JSON shaping and fail-open logic without the overhead of building the Rust binary.

**Rationale**: `scripts/hook-test` runs in seconds and covers JSON shaping for all nine Claude Code tools without needing Rust. Developers iterating on hook logic benefit from this fast feedback loop. The new verify-trace-capture is the authoritative end-to-end proof that runs in CI/worker environments. Both serve different purposes.

**Evidence**: The lead-dev explicitly recommended keeping both. The architect's question "Should the existing hook-test be upgraded or should a separate entrypoint be added?" was resolved by lead-dev: "I recommend keeping both."

### Decision 5: Verification targets repository hook sources directly, not installer-generated copies

**Choice**: Both fixtures load hook source files from `hooks/claude-code/` and `hooks/pi/` directly, not from installer-generated copies produced by `scryrs init --agent ...`.

**Rationale**: The goal is to verify the reference hooks as published in the repository. Installer copies may diverge or lag behind. Testing repository sources directly gives the most immediate feedback on changes and is simpler to implement without requiring the installer binary.

**Evidence**: Architect's question: "Should the verification target repository hook sources directly... or installer-generated copies?" — lead-dev's approach implicitly chose repository sources by recommending `hooks/pi/index.ts` via tsx.

## Risks

| Risk | Mitigation |
|---|---|
| Cold-cache `cargo build --release` can take 5+ minutes, slowing CI/precommit cycles. | The `scripts/verify-trace-capture` entrypoint is NOT initially wired into `scripts/precommit-run`. It runs on-demand or via dedicated CI job. Docker volume caching of `target/` reduces rebuild time to near-zero for hook-only changes. |
| The fake `ExtensionAPI` for Pi may diverge from real Pi runtime behavior (timing, error propagation, event ordering). | The fake API implements the minimum surface documented in `hooks/pi/ambient.d.ts` and tests only the hook's event-shaping and non-interference logic — not Pi's runtime behavior. This is a known trade-off documented in the spec. |
| `tsx` adds a transient npm dependency that must be installed in the Node container during each test run. | Installation is scoped to a temp directory and discarded. The `node:22-alpine` image is small and stable. Future optimization could cache `tsx` in a Docker volume. |
| The verification does not cover multi-event batching or edge cases in `scryrs record` (blank lines, schema-invalid events). | Those cases are already covered by `crates/scryrs-cli` unit tests. This verification focuses on hook-to-record integration, not exhaustive CLI testing. |

## Traceability

| Source | Artifact |
|---|---|
| Task 0cb48e7a-ad81-4ad4-a451-7bb21ef6a750 prompt | Acceptance criteria, scenarios, technical notes |
| Exploration Dossier (`2026-06-20T18:29:42.319Z`) | Problem framing, goals, non-goals, assumptions, likely affected areas |
| Decision `1-swarm-architect-recommendation` | Cross-harness entrypoint, two fixtures, Docker-backed, no hook/spec changes |
| Decision `1-swarm-lead-dev-recommendation` | Real binary via run_rust, run_node helper, tsx Pi loading, sequential containers |
| `openspec/specs/scryrs-record-endpoint/spec.md` | Record ingestion, persistence, deterministic summary contract |
| `openspec/specs/claude-code-reference-hook/spec.md` | Claude Code hook behavior, tool mapping, fail-open, existing verification spec |
| `openspec/specs/pi-reference-hook/spec.md` | Pi hook behavior, failure mapping, pass-through requirement |
| `crates/scryrs-types/src/lib.rs` | Canonical TraceEvent envelope, payload families, Outcome type |
| `.pi/rules/runtime-environment.md` | Worker runtime constraint (no host Node.js) |
| `scripts/lib/docker-verification.sh` | Existing Docker infrastructure pattern |
| `scripts/.versions` | Pinned `NODE_IMAGE=node:22-alpine`