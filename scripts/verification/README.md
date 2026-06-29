# scryrs Verification Suites

This directory contains end-to-end verification fixtures for two independent
operator workflows:

- **Hook capture verification** proves the native `scryrs hook <harness>`
  integration feeds real scryrs persistence without changing agent-visible
  behavior.
- **Live hotspot verification** proves the shipped `scryrs` binary can run the
  real multi-agent live workflow headlessly: `scryrs server`, remote
  `scryrs record --file`, cumulative hotspot queries, duplicate replay, and SSE
  replay/resume.

For hook capture:

- **Claude Code** pipes the `PreToolUse` event JSON to `scryrs hook claude-code`
  on stdin. There is no `.mjs` hook file and no node hook process.
- **Pi** loads a thin in-process `index.ts` shim that forwards the raw event to
  `scryrs hook pi --file <PATH>` (Pi's `exec()` opens stdin as `/dev/null`).

All tool→`TraceEvent` translation lives once, in the Rust
`scryrs-adapter-harness` crate. The fixtures exercise the shipped `scryrs`
binary, never repository hook-translation source.

## Architecture

```
scripts/verification/
├── README.md                  # This file
├── lib/
│   ├── assert.mjs             # pass/fail/assert helpers + summary
│   ├── db.mjs                 # readEventsDb, assertEventShape (SQLite)
│   └── pi-shim-driver.mjs     # loads hooks/pi/index.ts via tsx with a mock Pi
├── claude-code-e2e.mjs        # native `scryrs hook claude-code` (cross-harness)
├── pi-hook-e2e.mjs            # native `scryrs hook pi` + transport shim
├── installed-hook-e2e.mjs     # `scryrs init` create/merge + native commands
└── live-hotspots-e2e.mjs      # real `scryrs server` + remote ingest + SSE replay
```

### Entrypoints

#### `scripts/verify-trace-capture`

Authoritative hook-capture verification entrypoint. It:

1. Builds the real `scryrs` binary via `cargo build --release` in a Rust
   Docker container.
2. Installs fixture deps (`better-sqlite3`, `tsx`) in a Node.js Docker container.
3. Runs the Claude Code fixture against the binary.
4. Runs the Pi fixture (native command + transport shim) against the binary.
5. Runs the installed-hook fixture to validate `scryrs init` output.

#### `scripts/verify-live-hotspots`

Authoritative live-workflow verification entrypoint. It:

1. Builds the real `scryrs` binary via `cargo build --release` in a Rust
   Docker container.
2. Copies the binary into `.docker-fixtures/scryrs`.
3. Runs `live-hotspots-e2e.mjs` in a Debian/glibc `node:22` container.
4. Fails non-zero on server startup failure, transport failure, malformed JSON,
   assertion failure, or timeout.

No host Node.js is required — all execution happens inside Docker containers.

### Fixtures

#### `claude-code-e2e.mjs`

Pipes real `PreToolUse` payloads to `scryrs hook claude-code` on stdin.

**What it proves:**

- **Mapping**: Each tracked PascalCase tool (`Read`, `Grep`, `Glob`,
  `WebSearch`, `Edit`, `Write`, `NotebookEdit`, `WebFetch`) maps to the correct
  canonical `TraceEvent` family, with `outcome = Success` (PreToolUse is
  pre-execution) and `session_id` taken from the payload.
- **Store location**: Events persist under the payload `cwd`, not the spawned
  process's working directory.
- **Pass-through**: Untracked tools (e.g. `TodoWrite`) produce no event.
- **Bash debug-gating**: `Bash` is dropped unless `SCRYRS_DEBUG` is non-empty,
  in which case it maps to `CommandExecuted`.
- **Fail-open**: Malformed stdin and an unwritable store each exit 0 with empty
  stdout and append a line to `.scryrs/hooks/claude-code-warnings.log`.

#### `pi-hook-e2e.mjs`

Two layers: (A) drives `scryrs hook pi --file <tmp>` with crafted raw Pi events,
and (B) loads `hooks/pi/index.ts` via `tsx` with a mock Pi runtime.

**What it proves:**

- **Mapping**: `read`→`FileOpened`, `ast_grep_search`→`SearchRun`,
  `edit`/`write`→`EditMade`, `lsp_navigation`→`SymbolInspected` (success) or
  `FailedLookup` (`isError`). `outcome` reflects `isError` (post-execution).
- **Pass-through** and **Bash debug-gating**, as for Claude Code.
- **Shim delegation**: The slimmed `index.ts` forwards the raw event (with an
  injected `session_id`) to `scryrs hook pi --file <tmp>` and persists.
- **Fail-open**: A non-zero `scryrs hook pi` invocation does not throw or alter
  the agent-visible tool result.
- **No translation in TypeScript**: `index.ts` contains no `scryrs record`
  call, no `TRACKED_TOOLS` whitelist, and no event-type mapping.

#### `installed-hook-e2e.mjs`

Runs `scryrs init --agent claude-code` and `scryrs init --agent pi` in temporary
consumer directories and proves the installed artifacts capture events.

**What it proves:**

- **Claude Code**: `init` create-or-merges `.claude/settings.json` with the
  native `scryrs hook claude-code` command hook (no `.mjs`, no `.claude/hooks`).
  Piping a `PreToolUse` payload to the native command persists an event,
  confirmed via `scryrs hotspots .` (`analyzedEventCount >= 1`). Next-step text
  references the native command and never a `.mjs` file.
- **Pi**: `init` installs the slimmed `index.ts` at
  `.pi/extensions/pi-trace/index.ts`. The fixture transpiles it via `tsx`,
  exercises it with a simulated `tool_result`, proves it invokes
  `scryrs hook pi --file`, and confirms persistence via `scryrs hotspots .`.

#### `live-hotspots-e2e.mjs`

Starts a fresh `scryrs server`, waits for readiness through the live hotspot
query API, submits deterministic remote `scryrs record --file` batches from two
agent identities, then verifies cumulative hotspot state, duplicate replay, and
signal replay/resume through the shipped binary.

**What it proves:**

- **Multi-agent cumulative ingest**: two explicit agents contribute to one
  overlapping `EditMade` hotspot on the shared server-owned SQLite store.
- **Threshold crossing**: the shared hotspot crosses the default threshold of
  `10` without changing server flags.
- **Duplicate idempotency**: re-submitting the same producer file is
  acknowledged as duplicate/idempotent and does not change cumulative hotspot
  score, counts, session count, evidence row IDs, or persisted signal history.
- **SSE replay/resume**: `GET /v1/repositories/{repository_id}/signals?after=0`
  replays persisted signals in ascending ID order, and reconnecting with
  `after=<last_seen_id>` does not replay already-seen signal IDs.
- **Loud failure behavior**: startup, transport, parsing, assertion, and timeout
  failures all surface as non-zero verification failures.

### Libraries

#### `lib/assert.mjs`

| Function | Description |
|---|---|
| `pass(name)` | Record a passing assertion |
| `fail(name, reason?)` | Record a failing assertion |
| `assert(condition, name)` | Condition-based assertion |
| `assertDeepEqual(actual, expected, name)` | Deep-equal JSON comparison |
| `summary()` | Print pass/fail counts and exit non-zero on failures |
| `counts()` | Return `{ passed, failed }` for aggregation |
| `reset()` | Reset pass/fail counters |

#### `lib/db.mjs`

| Function | Description |
|---|---|
| `readEventsDb(path)` | Read all events from a `.scryrs/scryrs.db` SQLite datastore |
| `assertEventShape(event, type, toolName?)` | Validate canonical `TraceEvent` envelope |

#### `lib/pi-shim-driver.mjs`

Run under `tsx`. Loads a Pi `index.ts` shim, exercises it with a mock Pi runtime
(`on` + `exec`), and prints captured `exec` calls as JSON. Used by the Pi
fixtures to verify delegation and fail-open without a real Pi runtime.

## Usage

### Run hook-capture verification

```bash
scripts/verify-trace-capture
```

### Run a specific hook harness

```bash
scripts/verify-trace-capture --claude-only   # Claude Code fixture only
scripts/verify-trace-capture --pi-only       # Pi fixture + installed-hook (pi)
scripts/verify-trace-capture --init-only     # Installed-hook fixture only
```

### Run live hotspot verification

```bash
scripts/verify-live-hotspots
```

This suite is intended to start as a standalone/nightly verification path, not
as a default PR-gate lane.

### Live dashboard smoke path

After `scripts/verify-live-hotspots` passes, you can smoke-test the live
dashboard against the same server contract:

1. Start `scryrs server` and ingest a deterministic fixture (the
   `live-hotspots-e2e.mjs` script is the reference fixture for producing replayable
   hotspot signals).
2. Start the dashboard in live mode:

```bash
scryrs dashboard --server-url http://127.0.0.1:8081 --repository-id repo-a --no-open
```

1. Verify rankings proxying:

```bash
curl http://127.0.0.1:8080/api/meta
curl http://127.0.0.1:8080/api/hotspots
```

   `GET /api/meta` must report `"mode":"live"` and the configured
   `repositoryId`; `GET /api/hotspots` must return the live server ranking
   payload rather than local `.scryrs/hotspots.json` wording.

1. Verify signal replay and resume through the dashboard-owned SSE proxy:

```bash
curl -N http://127.0.0.1:8080/api/signals?after=0
curl -N http://127.0.0.1:8080/api/signals?after=<last_seen_signal_id>
```

   The first connection should replay persisted signals in ascending `id` order.
   Reconnecting with `after=<last_seen_signal_id>` must skip already-seen ids and
   continue with only newer signals. In the browser UI, the Signals route should
   surface connecting / connected / reconnecting / error state explicitly.

### Run a single fixture directly (for debugging)

```bash
# Build scryrs first, then install hook-fixture deps when needed.
cargo build --release
npm install better-sqlite3 tsx

SCRYRS_BIN="$PWD/target/release/scryrs" node scripts/verification/claude-code-e2e.mjs
SCRYRS_BIN="$PWD/target/release/scryrs" node scripts/verification/pi-hook-e2e.mjs
SCRYRS_BIN="$PWD/target/release/scryrs" node scripts/verification/installed-hook-e2e.mjs
SCRYRS_BIN="$PWD/target/release/scryrs" node scripts/verification/live-hotspots-e2e.mjs
```

## Debug mode notes

`SCRYRS_DEBUG` is opt-in and intended for development only. When set to a
non-empty value it enables `Bash`/`bash` capture and bounded `[scryrs-hook]`
and `[scryrs]` breadcrumbs on stderr. Keep it off in normal runs.

## Docker Image Compatibility

The verification entrypoint uses `node:22` (Debian-based, glibc) for running
fixtures because the `scryrs` binary is compiled on `rust:1.85.0` (also glibc).
Alpine-based Node images (musl libc) cannot run glibc-compiled binaries.

Override the fixture image with:

```bash
FIXTURE_NODE_IMAGE=node:24 scripts/verify-trace-capture
FIXTURE_NODE_IMAGE=node:24 scripts/verify-live-hotspots
```

## Relationship to Other Tests

- `cargo test` (unit + `hook_tests.rs`) — fast, in-process coverage of the
  adapters and the `scryrs hook` command, including fail-open, plus live-server
  unit/integration coverage for ingest, accumulators, idempotency, and SSE.
  No Docker needed.
- `scripts/verify-trace-capture` — authoritative end-to-end proof that the
  native `scryrs hook` integration feeds real scryrs persistence. May be wired
  into `scripts/precommit-run` via `--with-trace-verify`.
- `scripts/verify-live-hotspots` — authoritative end-to-end proof of the real
  multi-agent live workflow through the shipped binary. It proves live ingest,
  duplicate replay, and SSE replay/resume; pair it with the documented live
  dashboard smoke path above when you need to validate `/api/hotspots` and
  `/api/signals` through `scryrs dashboard --server-url ... --repository-id ...`.
  Initial CI posture is standalone/nightly rather than PR-gated by default.
