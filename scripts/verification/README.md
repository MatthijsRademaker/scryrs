# scryrs Verification Suites

This directory contains end-to-end verification fixtures that prove the scryrs
reference hooks correctly feed real `scryrs record --stdin` persistence without
changing agent-visible behavior.

## Architecture

```
scripts/verification/
├── README.md                  # This file
├── lib/
│   ├── assert.mjs             # pass/fail/assert helpers + summary
│   └── jsonl.mjs              # readJsonl, assertEventShape
├── claude-code-e2e.mjs        # Claude Code source-hook fixture
├── pi-hook-e2e.mjs            # Pi source-hook fixture
└── installed-hook-e2e.mjs     # Installed-hook e2e fixture
```

### Entrypoint

`scripts/verify-trace-capture` — the single authoritative entrypoint. It:

1. Builds the real `scryrs` binary via `cargo build --release` in a Rust
   Docker container.
2. Runs the Claude Code source-hook fixture against it in a Node.js Docker container.
3. Runs the Pi source-hook fixture against it in the same container.
4. Runs the installed-hook e2e fixture to validate init output.

No host Node.js is required — all execution happens inside Docker containers.

### Fixtures

#### `claude-code-e2e.mjs`

Exercises `hooks/claude-code/scryrs-hook.mjs` against real `scryrs record --stdin`.

**What it proves:**

- **JSON shaping**: All nine whitelisted Claude Code tools produce correctly
  shaped `TraceEvent` JSON with canonical envelope fields (`schema_version`,
  `timestamp`, `session_id`, `event_type`, `tool_name`, `payload` with `type`
  tag, `outcome`).
- **Persistence**: Events are persisted to `.scryrs/scryrs.db` by the real
  scryrs binary.
- **Non-interference**: The hook subprocess writes zero bytes to stdout and
  zero bytes to stderr.
- **Fail-open**: When scryrs is not on PATH, the hook returns
  `{continue: true}` and writes a timestamped warning to
  `.scryrs/hooks/claude-code-warnings.log`.
- **Pass-through**: Tools not in the whitelist (e.g., Task) produce no
  trace events.

#### `pi-hook-e2e.mjs`

Loads `hooks/pi/index.ts` via `tsx` against a fake `ExtensionAPI` and exercises
all six tracked Pi tools.

**What it proves:**

- **SessionStart**: The `session_start` lifecycle event produces a
  `SessionStart` TraceEvent with correct envelope shape.
- **Tool event capture**: All six tracked Pi tools (`read`, `bash`,
  `ast_grep_search`, `edit`, `write`, `lsp_navigation`) produce correctly
  mapped events (`FileOpened`, `CommandExecuted`, `SearchRun`, `EditMade`,
  `SymbolInspected`).
- **Non-interference**: The handler returns `undefined` for every event —
  the original tool result is never modified.
- **Failure propagation**: A failing `lsp_navigation` (`isError: true`)
  produces a `FailedLookup` event with `outcome.result: 'Failure'` while
  the original error-state event input is preserved unchanged.
- **Fail-open**: When scryrs is not on PATH, the handler returns
  `undefined` and `console.error` reports the scryrs failure.
- **Unlisted tools**: Pi tools not in the tracked set (e.g., `web_search`)
  are silently ignored and produce no trace events.

#### `installed-hook-e2e.mjs`

Runs `scryrs init --agent claude-code` and `scryrs init --agent pi` in temporary
consumer project directories, loads the installed hook artifacts from consumer
install paths (NOT from `hooks/` in the repository source tree), and exercises
them against the real `scryrs` binary.

**What it proves:**

- **Init output is functional**: Proves that `scryrs init` produces
  loadable, working hook artifacts — not just that files were created.
  Source-hook fixtures load from `hooks/` in the repo; this fixture loads
  from `.claude/hooks/` and `.pi/extensions/pi-trace/` in consumer directories.
- **Claude Code installed hook**: The installed `scryrs-hook.mjs` is a valid
  Node.js module that returns `{continue: true}`, produces zero stdout/stderr,
  and persists events to `.scryrs/scryrs.db` via the real scryrs binary.
- **Pi installed hook**: The installed `index.ts` is a valid TypeScript module
  (transpiled via `tsx`) that registers a `tool_result` handler, returns
  `undefined` (non-interference), and persists events to `.scryrs/scryrs.db`.
  **Version-gating:** The single-file `index.ts` sufficiency assumption has been
  verified against Pi versions that expect a single `index.ts` extension file
  without additional manifest, `package.json`, or `tsconfig` artifacts. If Pi
  extends its extension contract to require additional consumer artifacts, this
  test must be updated.
- **Next-step text accuracy**: The deterministic stdout output from `scryrs init`
  is checked to ensure users receive correct setup instructions.
- **Error detection**: A corrupt or unloadable hook file causes the fixture to
  fail with a diagnostic, rather than silently skipping the load step.

### Libraries

#### `lib/assert.mjs`

Shared assertion helpers used by both fixtures:

| Function | Description |
|---|---|
| `pass(name)` | Record a passing assertion |
| `fail(name, reason?)` | Record a failing assertion |
| `assert(condition, name)` | Condition-based assertion |
| `assertDeepEqual(actual, expected, name)` | Deep-equal JSON comparison |
| `summary()` | Print pass/fail counts and exit non-zero on failures |
| `counts()` | Return `{ passed, failed }` for aggregation |
| `reset()` | Reset pass/fail counters |

#### `lib/jsonl.mjs`

JSONL helpers for reading and validating scryrs event files:

| Function | Description |
|---|---|
| `readEventsDb(path)` | Read all events from a `.scryrs/scryrs.db` SQLite datastore |
| `assertEventShape(event, type, toolName?)` | Validate canonical `TraceEvent` envelope |

## Usage

### Run all verifications

```bash
scripts/verify-trace-capture
```

### Run a specific harness

```bash
scripts/verify-trace-capture --claude-only
scripts/verify-trace-capture --pi-only
scripts/verify-trace-capture --init-only
```

### Run a single fixture directly (for debugging)

```bash
# Build scryrs first
cargo build --release

# Run the fixture (requires Node.js)
node scripts/verification/claude-code-e2e.mjs
node scripts/verification/pi-hook-e2e.mjs
```

## Docker Image Compatibility

The verification entrypoint uses `node:22` (Debian-based, glibc) for running
fixtures because the `scryrs` binary is compiled on `rust:1.85.0` (also glibc).
Alpine-based Node images (musl libc) cannot run glibc-compiled binaries.

Override the fixture image with:

```bash
FIXTURE_NODE_IMAGE=node:24 scripts/verify-trace-capture
```

## Relationship to Other Tests

- `scripts/hook-test` — fast, fake-scryrs development feedback loop for Claude
  Code JSON shaping and fail-open logic. Runs in seconds, no Rust build
  required. Does NOT prove persistence through real `scryrs record`.
- `scripts/verify-trace-capture` — authoritative end-to-end proof that both
  hooks correctly feed real scryrs persistence. May be wired into
  `scripts/precommit-run` as a dedicated CI step via `--with-trace-verify`.

Both serve different purposes and neither replaces the other.
