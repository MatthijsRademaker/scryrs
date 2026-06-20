## 1. Docker Infrastructure: `run_node` helper

- [x] 1.1 Add `run_node()` function to `scripts/lib/docker-verification.sh` following the `run_rust` pattern: uid/gid mapping, workspace volume mount (`$ROOT:/workspace`), image pull-on-missing via `_pull_image_if_missing`, using `NODE_IMAGE` sourced from `scripts/.versions`.
- [x] 1.2 Source `scripts/.versions` in `docker-verification.sh` (the file already uses `RUST_IMAGE` and `SECURITY_RUST_IMAGE` from the script body; add `NODE_IMAGE` sourcing from `.versions` if not already present).
- [x] 1.3 Verify `run_node` works by running `node --version` in a smoke test.

## 2. Cross-Harness Verification Entrypoint

- [x] 2.1 Create `scripts/verify-trace-capture` bash entrypoint that sources `docker-verification.sh`, builds the real `scryrs` binary via `run_rust cargo build --release`, then runs both Claude Code and Pi fixture scripts via `run_node`.
- [x] 2.2 The entrypoint SHALL accept `--claude-only` and `--pi-only` flags for targeted testing.
- [x] 2.3 The entrypoint SHALL print a summary of passed/failed assertions and exit non-zero on any failure.

## 3. Claude Code Fixture: Real scryrs persistence

- [x] 3.1 Create `scripts/verification/claude-code-e2e.mjs` that reuses the JSON-shaping, happy-path, fail-open, transparency, and pass-through test logic from `scripts/hook-test-runner.mjs`.
- [x] 3.2 Instead of a fake shell-script scryrs, pipe hook events to the real `scryrs record --stdin` binary (built in task 2.1) and assert:
  - The deterministic JSON summary on stdout contains `command: "record"`, correct `accepted`/`rejected` counts, and `schemaVersion`.
  - `.scryrs/events.jsonl` contains the expected number of persisted events with canonical `TraceEvent` envelope fields (`schema_version`, `timestamp`, `session_id`, `event_type`, `tool_name`, `payload` with `type` tag, `outcome`).
- [x] 3.3 Assert non-interference: hook process produces zero stdout and zero stderr.
- [x] 3.4 Assert fail-open: when scryrs binary is temporarily moved off PATH in a temp workdir, the hook still returns `{continue: true}` and no tool-output corruption occurs.

## 4. Pi Fixture: Fake ExtensionAPI harness

- [x] 4.1 Create `scripts/verification/pi-hook-e2e.mjs` that installs `tsx` transiently in a temp directory and dynamically imports `hooks/pi/index.ts`.
- [x] 4.2 Implement a fake `ExtensionAPI` object with:
  - `on(event, handler)`: stores handlers by event name.
  - `exec(command, args, options)`: wraps `child_process.spawn` for `scryrs record --stdin` with the same stdin-pipe + timeout semantics as the real `pi.exec()`. Returns `{ stdout, stderr, code, killed }`.
  - Manual event emission helpers: `emitSessionStart(reason)` and `emitToolResult(event)` where `event` matches the `ToolResultEvent` interface from `hooks/pi/ambient.d.ts`.
- [x] 4.3 Test successful capture:
  - Emit `session_start`, then emit representative `tool_result` events for each of the six tracked Pi tools (`read`, `bash`, `ast_grep_search`, `edit`, `write`, `lsp_navigation` success).
  - Assert `.scryrs/events.jsonl` contains a `SessionStart` event followed by the correct tool events with canonical envelope fields.
  - Assert the handler returns `undefined` for every event (non-interference on the Pi path).
- [x] 4.4 Test failure propagation:
  - Emit a failing `lsp_navigation` tool_result (`isError: true`, `input: { symbol: 'nonexistent_fn' }`).
  - Snapshot the original event input before passing to the hook handler.
  - Assert `.scryrs/events.jsonl` contains a `FailedLookup` event with `outcome.result: 'Failure'` and `payload.subject: 'nonexistent_fn'`.
  - Assert the handler still returns `undefined` and the original error-state event input is unchanged (deep equality check on the snapshot).
- [x] 4.5 Test fail-open:
  - Run the Pi fixture with scryrs off PATH and assert the handler returns `undefined` for all events, no crash occurs, and `console.error` is called with the expected scryrs-failure message.

## 5. Assertion library and shared helpers

- [x] 5.1 Create `scripts/verification/lib/assert.mjs` with `pass(name)`, `fail(name, reason)`, `assert(condition, message)`, `assertDeepEqual(actual, expected, name)` helpers, plus a `summary()` that prints pass/fail counts and exits non-zero on failures.
- [x] 5.2 Create `scripts/verification/lib/jsonl.mjs` with `readJsonl(path)` that returns parsed JSON objects from a JSONL file, and `assertEventShape(event, expectedType, expectedToolName)` that validates the canonical TraceEvent envelope.

## 6. Integration and documentation

- [x] 6.1 Document the `scripts/verify-trace-capture` entrypoint in both hook READMEs (`hooks/claude-code/README.md` and `hooks/pi/README.md`), replacing or supplementing the current manual verification steps with a reference to the automated entrypoint.
- [x] 6.2 Add a `scripts/verification/README.md` explaining the verification architecture, how to run it, and what each fixture proves.
- [x] 6.3 (Optional) Register `scripts/verify-trace-capture` in `scripts/precommit-run` as a dedicated CI step, gated behind a `--with-trace-verify` flag or equivalent to avoid slowing the default precommit cycle.
