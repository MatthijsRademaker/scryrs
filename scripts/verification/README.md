# scryrs Verification Suites

This directory defines the runnable verification lanes for production hardening. The installed-user diagnostic entrypoint is `scryrs doctor`; the authoritative release gate is `scripts/verify-production-suite`.

## Operator entrypoints

| Entrypoint | Purpose | Posture |
| --- | --- | --- |
| `scryrs doctor` | Diagnose the current workspace install and readiness state | Installed-user / operator command |
| `scripts/verify-production-suite` | Run the full headless production-readiness suite | Explicit heavy maintainer lane |
| `scripts/precommit-run` | Default lighter PR-gate wrapper (`scripts/check` + `scripts/test`) | Default development lane |
| `scripts/precommit-run --production` | Explicit wrapper for the full production suite | Heavy maintainer lane |

The production suite is intentionally **not** the default PR gate in this change. Run it explicitly before release hardening decisions.

## What lives here

This directory contains end-to-end fixtures and documentation for four automated verification workflows:

- **Hook capture verification** — prove the native `scryrs hook <harness>` integrations feed real scryrs persistence without changing agent-visible behavior.
- **Live hotspot verification** — prove the shipped `scryrs` binary can drive the real multi-agent live ingest workflow headlessly.
- **Core artifact-loop verification** — prove the deterministic local artifact loop `record -> hotspots -> graph -> route -> propose -> proposals accept` produces the documented artifacts through the real binary.
- **Privacy defaults verification** — prove compiled telemetry/privacy defaults stay in their safe release posture.

For hook capture:

- **Claude Code** pipes the `PreToolUse` event JSON to `scryrs hook claude-code` on stdin. There is no `.mjs` hook file and no node hook process.
- **Pi** loads a thin in-process `index.ts` shim that forwards the raw event to `scryrs hook pi --file <PATH>` (Pi's `exec()` opens stdin as `/dev/null`).

All tool→`TraceEvent` translation lives once, in the Rust `scryrs-adapter-harness` crate. The fixtures exercise the shipped `scryrs` binary, never repository hook-translation source.

## Architecture

```text
scripts/verification/
├── README.md                    # This file
├── lib/
│   ├── assert.mjs               # pass/fail/assert helpers + summary
│   ├── db.mjs                   # readEventsDb, assertEventShape (SQLite via python3)
│   └── pi-shim-driver.mjs       # loads hooks/pi/index.ts via tsx with a mock Pi
├── claude-code-e2e.mjs          # native `scryrs hook claude-code`
├── pi-hook-e2e.mjs              # native `scryrs hook pi` + transport shim
├── installed-hook-e2e.mjs       # `scryrs init` create/merge + native commands
├── live-hotspots-e2e.mjs        # real `scryrs server` + remote ingest + SSE replay
└── core-artifact-loop-e2e.sh    # real `scryrs` binary through the deterministic local loop
```

## Automated lanes

### `scripts/verify-production-suite`

Authoritative headless production-readiness gate. It runs these required lanes, in order, and stops on the first failure:

1. `scripts/check`
2. `scripts/test --full`
3. `scripts/security`
4. `scripts/verify-install`
5. `scripts/verify-trace-capture`
6. `scripts/verify-live-hotspots`
7. `scripts/verify-core-artifact-loop`
8. `scripts/verify-privacy-defaults`
9. `scripts/verify-docs-publish`

**Expected posture:** explicit maintainer invocation, Docker or DinD available, headless execution, non-zero exit on the first failing required lane.

**Failure interpretation:** the lane header printed immediately before the failure is the proving path that broke. Re-run that lane directly to debug instead of re-running the full suite blindly.

### `scripts/verify-trace-capture`

Authoritative hook-capture verification entrypoint. It:

1. Builds the real `scryrs` binary via `cargo build --release` in a Rust Docker container.
2. Installs fixture deps (`tsx`) in a Node.js Docker container.
3. Runs the Claude Code fixture against the binary.
4. Runs the Pi fixture (native command + transport shim) against the binary.
5. Runs the installed-hook fixture to validate `scryrs init` output.

### `scripts/verify-live-hotspots`

Authoritative live-workflow verification entrypoint. It:

1. Builds the real `scryrs` binary via `cargo build --release` in a Rust Docker container.
2. Copies the binary into `.docker-fixtures/scryrs`.
3. Runs `live-hotspots-e2e.mjs` in a Debian/glibc Node container.
4. Fails non-zero on server startup failure, transport failure, malformed JSON, assertion failure, or timeout.

### `scripts/verify-core-artifact-loop`

Authoritative deterministic local artifact-loop entrypoint. It:

1. Builds the real `scryrs` binary via `cargo build --release` in a Rust Docker container.
2. Copies the binary into `.docker-fixtures/scryrs`.
3. Runs `core-artifact-loop-e2e.sh` against a temporary fixture repository.
4. Executes `record -> hotspots -> graph -> route -> propose -> proposals accept` through the shipped binary.
5. Asserts `.scryrs/scryrs.db`, `.scryrs/hotspots.json`, `.scryrs/graph.json`, `.scryrs/routes.json`, `.scryrs/proposals/`, and `.scryrs/accepted/` are produced in their documented locations.

### `scripts/verify-privacy-defaults`

Runnable compiled privacy assertion lane. It runs `cargo test -p scryrs-telemetry --locked` in the Rust Docker container and fails if telemetry opt-in defaults, redaction defaults, or remote prompt-storage defaults drift.

## Fixtures

### `claude-code-e2e.mjs`

Pipes real `PreToolUse` payloads to `scryrs hook claude-code` on stdin.

**What it proves:**

- **Mapping**: Each tracked PascalCase tool (`Read`, `Grep`, `Glob`, `WebSearch`, `Edit`, `Write`, `NotebookEdit`, `WebFetch`) maps to the correct canonical `TraceEvent` family, with `outcome = Success` (PreToolUse is pre-execution) and `session_id` taken from the payload.
- **Store location**: Events persist under the payload `cwd`, not the spawned process's working directory.
- **Pass-through**: Untracked tools (e.g. `TodoWrite`) produce no event.
- **Bash debug-gating**: `Bash` is dropped unless `SCRYRS_DEBUG` is non-empty, in which case it maps to `CommandExecuted`.
- **Fail-open**: Malformed stdin and an unwritable store each exit 0 with empty stdout and append a line to `.scryrs/hooks/claude-code-warnings.log`.

### `pi-hook-e2e.mjs`

Two layers: (A) drives `scryrs hook pi --file <tmp>` with crafted raw Pi events, and (B) loads `hooks/pi/index.ts` via `tsx` with a mock Pi runtime.

**What it proves:**

- **Mapping**: `read`→`FileOpened`, `ast_grep_search`→`SearchRun`, `edit`/`write`→`EditMade`, `lsp_navigation`→`SymbolInspected` (success) or `FailedLookup` (`isError`). `outcome` reflects `isError` (post-execution).
- **Pass-through** and **Bash debug-gating**, as for Claude Code.
- **Shim delegation**: The slimmed `index.ts` forwards the raw event (with an injected `session_id`) to `scryrs hook pi --file <tmp>` and persists.
- **Fail-open**: A non-zero `scryrs hook pi` invocation does not throw or alter the agent-visible tool result.
- **No translation in TypeScript**: `index.ts` contains no `scryrs record` call, no `TRACKED_TOOLS` whitelist, and no event-type mapping.

### `installed-hook-e2e.mjs`

Runs `scryrs init --agent claude-code --mode local` and `scryrs init --agent pi --mode local` in temporary consumer directories and proves the installed artifacts capture events.

### `live-hotspots-e2e.mjs`

Starts a fresh `scryrs server`, waits for readiness through the live hotspot query API, submits deterministic remote `scryrs record --file` batches from two agent identities, then verifies cumulative hotspot state, duplicate replay, and signal replay/resume through the shipped binary.

### `core-artifact-loop-e2e.sh`

Creates a temporary repository, records deterministic local trace events via `scryrs record --mode local`, materializes hotspots/graph/routes/proposals, accepts one proposal, and asserts the expected artifact directories and files exist.

## Usage

### Run the authoritative production suite

```bash
scripts/verify-production-suite
```

### Run the heavy wrapper from the precommit entrypoint

```bash
scripts/precommit-run --production
```

### Run individual lanes

```bash
scripts/verify-trace-capture
scripts/verify-live-hotspots
scripts/verify-core-artifact-loop
scripts/verify-privacy-defaults
scripts/verify-docs-publish
scripts/security
scripts/verify-install
```

### Run a specific hook harness

```bash
scripts/verify-trace-capture --claude-only
scripts/verify-trace-capture --pi-only
scripts/verify-trace-capture --init-only
```

### Run the diagnostic command directly

```bash
scryrs doctor
scryrs doctor --json
```

## Live dashboard manual smoke boundary

Automated production gating intentionally stops at the live server contract. **Live dashboard browser verification is still manual in this change.** After `scripts/verify-live-hotspots` passes, smoke-test the dashboard separately:

1. Start `scryrs server` and ingest the deterministic fixture used by `live-hotspots-e2e.mjs`.
2. Start the dashboard in live mode:

```bash
scryrs dashboard --server-url http://127.0.0.1:8081 --repository-id repo-a --no-open
```

1. Verify rankings proxying:

```bash
curl http://127.0.0.1:8080/api/meta
curl http://127.0.0.1:8080/api/hotspots
```

1. Verify signal replay and resume through the dashboard-owned SSE proxy:

```bash
curl -N http://127.0.0.1:8080/api/signals?after=0
curl -N http://127.0.0.1:8080/api/signals?after=<last_seen_signal_id>
```

`GET /api/meta` must report `"mode":"live"` and the configured `repositoryId`; `GET /api/hotspots` must return the live server ranking payload; reconnecting to `/api/signals` with `after=<last_seen_signal_id>` must skip already-seen ids.

## Privacy proving map

| Boundary | Proving lane |
| --- | --- |
| Telemetry opt-in default | `scripts/verify-privacy-defaults` (`scryrs-telemetry` compiled tests) |
| Prompt/source/path redaction defaults | `scripts/verify-privacy-defaults` |
| Remote prompt-storage default off | `scripts/verify-privacy-defaults` |
| Debug-gated Bash capture | `scripts/test --full` and `scripts/verify-trace-capture` |
| Fail-open hook behavior | `scripts/verify-trace-capture` |
| Remote mode no dual-write / no local fallback | `scripts/test --full` plus `scripts/verify-live-hotspots` |
| Dependency policy | `scripts/security` |

## Runtime prerequisites and posture

- **Docker or DinD is required** for every automated lane here.
- **No host Rust or Node.js is required** for the automated wrappers; they run through Docker-backed scripts.
- **The production suite is intentionally slower than the default PR gate.** Use it explicitly for release hardening, not every edit loop.
- **Live verification is automated only at the server/API layer.** Browser/dashboard verification remains manual.

## Linux vs macOS verification posture

### Automated posture

Current automated verification runs in Linux Docker or DinD environments. That proves the Linux build, Linux install path, and Linux containerized verification behavior only.

### Exact macOS manual maintainer commands

Run these on a real macOS machine when you need **native Darwin** confidence for the installed binary:

```bash
scripts/install --bin-dir "$HOME/.local/bin"
SCRYRS_BIN="$HOME/.local/bin/scryrs"
MACOS_FIXTURE_ROOT="$(mktemp -d)"
REPO_ROOT="$MACOS_FIXTURE_ROOT/repo"
mkdir -p "$REPO_ROOT"
cd "$REPO_ROOT"

"$SCRYRS_BIN" --version
"$SCRYRS_BIN" init --agent claude-code --mode local
"$SCRYRS_BIN" doctor --json

cat > events.jsonl <<'JSONL'
{"schema_version":"0.1.0","timestamp":"2026-06-29T12:00:00Z","session_id":"macos-manual-1","event_type":"FileOpened","tool_name":"read","payload":{"type":"FileOpened","path":"src/auth.ts"},"outcome":{"result":"Success"}}
{"schema_version":"0.1.0","timestamp":"2026-06-29T12:00:01Z","session_id":"macos-manual-1","event_type":"SearchRun","tool_name":"grep","payload":{"type":"SearchRun","query":"auth"},"outcome":{"result":"Success"}}
JSONL

"$SCRYRS_BIN" record --mode local --file events.jsonl
"$SCRYRS_BIN" hotspots .
"$SCRYRS_BIN" graph .
"$SCRYRS_BIN" route .
"$SCRYRS_BIN" propose .
proposal_path="$(printf '%s\n' .scryrs/proposals/*.json | sort | head -n 1)"
proposal_id="$(basename "$proposal_path" .json)"
"$SCRYRS_BIN" proposals accept . "$proposal_id" --reviewer macos-manual --rationale "manual verification" --decided-at "2026-06-29T12:00:00Z"
test -f ".scryrs/accepted/$proposal_id.json"
```

If you also want the existing **Docker-backed** server-contract checks while on macOS, run them separately:

```bash
scripts/verify-trace-capture --claude-only
scripts/verify-live-hotspots
```

Those commands still prove the Linux-container verification lanes, not the native Darwin packaging path. For the manual live dashboard smoke on macOS, use the commands from the "Live dashboard manual smoke boundary" section above with the installed binary.

**Limitation:** the current Linux Docker automation does **not** prove native Darwin behavior. Do not claim automated macOS coverage unless a real macOS runner is added.

## Debug mode notes

`SCRYRS_DEBUG` is opt-in and intended for development only. When set to a non-empty value it enables `Bash`/`bash` capture and bounded `[scryrs-hook]` and `[scryrs]` breadcrumbs on stderr. Keep it off in normal runs.

## Docker image compatibility

- The hook and live-workflow fixtures use `node:22` (Debian/glibc) because the `scryrs` binary is compiled on `rust:1.85.0` (also glibc).
- Alpine-based Node images (musl libc) cannot run the glibc-compiled binary.
- Override the fixture image when needed with `FIXTURE_NODE_IMAGE=node:24 scripts/verify-trace-capture` or `FIXTURE_NODE_IMAGE=node:24 scripts/verify-live-hotspots`.

## Relationship to other tests

- `cargo test` (via `scripts/test`) — fast unit and integration coverage.
- `scripts/check` — format, frontend check, workspace check, clippy, and docs publish verification.
- `scripts/test --full` — Rust tests plus the lighter Claude-only `scripts/hook-test` wrapper lane.
- `scripts/verify-production-suite` — authoritative composed release gate.
