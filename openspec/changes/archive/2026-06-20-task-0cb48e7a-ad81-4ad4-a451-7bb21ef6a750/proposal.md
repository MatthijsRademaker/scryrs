## Why

Trace capture foundations are in place — both the Claude Code reference hook and the Pi reference hook ship in the repository, and `scryrs record --stdin` persists events to `.scryrs/events.jsonl`. However, the repository lacks one authoritative, repeatable end-to-end proof that these reference hooks feed real `scryrs record` persistence without changing agent-visible behavior.

The existing Claude Code hook verification (`scripts/hook-test`) uses a fake shell-script scryrs and requires host-installed Node.js — it cannot run in the worker environment and never proves real persistence through the `scryrs record` binary. The Pi hook has no automated verification at all — its README documents manual verification steps only. This task closes that proof gap with a single Docker-backed cross-harness verification entrypoint.

## What Changes

- **New cross-harness verification entrypoint** `scripts/verify-trace-capture` that builds the real `scryrs` binary via Docker (Rust container) and drives both hook fixtures against it (Node container). The entrypoint is runnable in the worker environment without host Node.js.
- **New `run_node` helper** in `scripts/lib/docker-verification.sh` following the existing `run_rust` pattern, using `node:22-alpine` from `scripts/.versions`.
- **Repackaged Claude Code fixture** that reuses existing `scripts/hook-test-runner.mjs` assertions (JSON shaping, happy-path, fail-open, transparency, pass-through) but pipes hook events to the real `scryrs record --stdin` binary and asserts the deterministic JSON summary output and persisted `.scryrs/events.jsonl` contents.
- **New Pi fixture harness** (`scripts/verification/pi-hook-e2e.mjs`) that loads `hooks/pi/index.ts` via `tsx` against a fake `ExtensionAPI`, emits `session_start` and representative `tool_result` events, invokes real `scryrs record --stdin`, verifies `.scryrs/events.jsonl` contains expected `SessionStart` and tool events, and checks that a failing `lsp_navigation` still surfaces the original error payload unchanged while scryrs records `FailedLookup` with `outcome.result: Failure`.

## Impact

- **Affected code**: New files under `scripts/verification/` and one new function in `scripts/lib/docker-verification.sh`. No hook source, CLI behavior, or OpenSpec specs are modified.
- **Affected specs**: New capability spec at `openspec/specs/cross-harness-verification/spec.md` (proposed in this change).
- **Worker/CI compatibility**: The new verification runs entirely in Docker containers (Rust → Node sequential), satisfying the worker runtime constraint of no host Node.js.
- **Developer workflow**: The existing `scripts/hook-test` (fast, fake-scryrs, requires host Node) is preserved for rapid development feedback. The new `scripts/verify-trace-capture` is the authoritative end-to-end proof and may be wired into `scripts/precommit-run` as a follow-up.
