# scryrs

Document what your agents keep rediscovering.

scryrs is a context intelligence suite for AI-assisted codebases. It observes how agents navigate a repository, detects recurring knowledge hotspots, and promotes repeated context into durable docs, routing manifests, and reusable agent memory.

## Workspace shape

```text
crates/scryrs-cli
  Main `scryrs` binary. Cargo features decide which suite parts ship.

crates/scryrs-types
  Shared contracts and small domain primitives.

crates/scryrs-core
  Standalone trace and hotspot foundation.

crates/scryrs-server
  Central trace ingest server with idempotent SQLite persistence.

crates/scryrs-dashboard
  Local dashboard server and browser UI for trace and hotspot visualization.

crates/scryrs-graph
  Knowledge graph and routing manifest foundation.

crates/scryrs-curator
  Reviewable docs, skill, decision, and memory proposal foundation.

crates/scryrs-policy
  Deterministic guardrail policy foundation.

crates/scryrs-llm
  Optional bounded provider-neutral LLM transport foundation.

crates/scryrs-sandbox
  Capability-scoped tool and filesystem policy foundation.

crates/scryrs-telemetry
  Opt-in telemetry and redaction foundation.

crates/scryrs-adapter-markdown
  Generic Markdown publishing surface.

crates/scryrs-adapter-rspress
  Optional Rspress publishing surface.

crates/scryrs-runtime
  Agent-side routing and retrieval helper foundation.

xtask
  Repo automation entry point.
```

## Feature split

The CLI ships seven commands:

```bash
# Hotspot analysis from recorded trace events
cargo run -p scryrs-cli -- hotspots /path/to/repo

# JSONL trace event ingestion
cargo run -p scryrs-cli -- record --stdin < events.jsonl
cargo run -p scryrs-cli -- record --file session.jsonl

# Hook installation for supported harnesses
cargo run -p scryrs-cli -- init --agent claude-code
cargo run -p scryrs-cli -- init --agent pi

# Local dashboard server
cargo run -p scryrs-cli -- dashboard

# Central trace ingest server
cargo run -p scryrs-cli -- server

# Graph build from hotspot evidence and docs structure
cargo run -p scryrs-cli -- graph .

# Route manifest generation
cargo run -p scryrs-cli -- route .

# Proposal generation from hotspot and graph evidence
cargo run -p scryrs-cli -- propose .
```

Default features include the standalone suite, Markdown adapter, runtime, and deterministic guardrail support. `full` adds the optional LLM boundary and Rspress adapter.

## Quickstart

Get from a freshly cloned repo to your first command in under two minutes.

### Install (recommended): one-shot binary

Install the latest released `scryrs` binary with a single command — no Rust toolchain
or source checkout required. Supported platforms: **macOS arm64** and **Linux x86_64**.

```bash
curl -fsSL https://raw.githubusercontent.com/matthijsrademaker/scryrs/main/install.sh | sh
```

The installer detects your OS/arch, downloads the matching release binary, verifies it
against its published `.sha256` checksum, installs it to `$HOME/.local/bin`, and runs
`scryrs --version`. To customise:

```bash
# custom install directory
curl -fsSL https://raw.githubusercontent.com/matthijsrademaker/scryrs/main/install.sh | sh -s -- --bin-dir /usr/local/bin

# pin a specific release tag (default: latest)
curl -fsSL https://raw.githubusercontent.com/matthijsrademaker/scryrs/main/install.sh | SCRYRS_VERSION=v0.1.0 sh
```

> **macOS note:** if Gatekeeper quarantines the downloaded binary, clear the flag with
> `xattr -d com.apple.quarantine "$HOME/.local/bin/scryrs"`.

### Prerequisites (source install)

- **Rust 1.85+** (install via [rustup](https://rustup.rs)) — or —
- **Docker** (the workspace ships a dev container with all tooling pre-installed)

### Install from source (contributors)

The `scripts/install` script builds and installs the `scryrs` binary to `$HOME/.local/bin`
by default. It works on macOS and Linux.

```bash
# Clone the repository
git clone <repo-url>  # replace <repo-url> with the actual clone URL
cd scryrs

# Install the scryrs CLI binary
./scripts/install
```

The installer builds `scryrs-cli` in release mode with default features, copies the
`scryrs` binary into the target directory, and verifies the install with `--version`.

To install to a custom directory:

```bash
SCRYRS_INSTALL_DIR=/usr/local/bin ./scripts/install
# or
./scripts/install --bin-dir /usr/local/bin
```

After install, if the target directory is not already on your `PATH`, the installer
prints exact instructions to add it to your shell profile.

### Install the agent hook, then configure transport (two steps)

Once `scryrs` is installed and reachable on `PATH`, the workflow is two distinct
commands: `scryrs init` installs the harness hook (config-free, idempotent), and
`scryrs setup <mode>` configures trace transport.

```bash
# Step 1 — install the hook (no config, never fails on missing config):
scryrs init --agent claude-code
scryrs init --agent pi

# Step 2a — local transport: single-machine SQLite store (.scryrs/scryrs.db):
scryrs setup local

# Step 2b — live transport: remote ingest via a scryrs server.
# Requires only ingest_url + workspace_id (repository_id derives from the Git
# remote origin, agent_id is autogenerated per container — neither is committed):
scryrs setup live \
  --ingest-url http://scryrs-server:8081 \
  --workspace-id my-workspace

# ...or put live config in scryrs.json / .scryrs/.env and let a TTY wizard
# collect only missing required values (--no-interactive keeps it fail-fast):
scryrs setup live
```

> **Breaking change:** `scryrs init` no longer takes `--mode` or any live-config
> flags — it installs only the hook. Replace
> `scryrs init --agent <NAME> --mode live --ingest-url ... --workspace-id ...`
> with `scryrs init --agent <NAME>` followed by
> `scryrs setup live --ingest-url ... --workspace-id ...`, and
> `scryrs init --agent <NAME> --mode local` with `scryrs setup local`.
> `setup live` no longer requires `docker_network`; that (and the `.scryrs/compose.yml`
> stack) is the `--with-compose` opt-in. `record` and `dashboard` still default to
> live mode (`--mode local` for the SQLite store).

**Note:** `scripts/install` only installs the CLI binary. It does not create or modify
`.claude/`, `.pi/`, `.scryrs/`, `scryrs.json`, git hooks, or shell profile files.
Hook installation is a separate step performed by `scryrs init --agent <NAME>`,
and transport configuration by `scryrs setup <mode>`, after the `scryrs` binary is
on your `PATH`.

### Run the live hotspot server (multi-agent Docker setup)

The repository includes Docker packaging for running `scryrs server` as a shared
network service for multiple agent containers.

```bash
# Build and start the containerized live server
docker compose up -d
```

This starts `scryrs-server` bound to `0.0.0.0:8081` with persistent storage at
`/data/scryrs/server.db`. The service is reachable by name from any container on
the `scryrs-net` network.

Once the server is running, install the hook and configure each agent workspace
for live remote ingest:

```bash
# In each agent workspace (not the scryrs source checkout):
scryrs init --agent claude-code
scryrs setup live \
  --ingest-url http://scryrs-server:8081 \
  --workspace-id my-workspace
```

`setup live` creates or merges a `scryrs.json` `remote` section (the committed
source of truth: `ingest_url` + `workspace_id`). It does not scaffold a local
`.scryrs/scryrs.db` — all events flow to the shared server. In interactive
terminals, omitting `ingest_url` or `workspace_id` starts a wizard;
`--no-interactive` disables prompts and preserves fail-fast validation.

`repository_id` derives from the Git remote origin URL when omitted and
`agent_id` is autogenerated per container; neither is written to committed
config. Identity may also be placed in `.scryrs/.env` instead of flags.

To self-host the live server with `scryrs up`, add the compose opt-in, which
scaffolds `.scryrs/compose.yml` and requires a Docker network:

```bash
scryrs setup live --with-compose \
  --ingest-url http://scryrs-server:8081 \
  --workspace-id my-workspace \
  --docker-network scryrs-net
scryrs up
```

**Agent containers on the same network:** attach your agent containers to
`scryrs-net` so they can resolve `http://scryrs-server:8081`:

```bash
docker run --network scryrs-net ...
# or add `networks: [scryrs-net]` to your agent's compose service.
```

For a complete multi-agent setup: run `docker compose up -d` once for the
server, then run `scryrs init` + `scryrs setup live` in each agent workspace
pointing at the same server endpoint. The live server provides `POST /v1/trace-events/batch`
ingest, `GET /v1/repositories/{id}/hotspots` queries, and `GET
/v1/repositories/{id}/signals` SSE streaming.

### Build from source (manual)

```bash
# Clone the repository
git clone <repo-url>  # replace <repo-url> with the actual clone URL
cd scryrs

# Build the CLI crate
cargo build -p scryrs-cli
```

Once built, verify it works:

```bash
cargo run -p scryrs-cli -- --help
```

### Explore the CLI surface

Every flag and command is discoverable from the terminal.

**`--help`** prints the full usage guide:

```bash
$ cargo run -p scryrs-cli -- --help
scryrs — context intelligence for AI-assisted codebases

Discover, analyze, and navigate hotspots in your codebase.

COMMANDS
  scryrs hotspots <PATH>
      Emit a versioned JSON hotspot report from recorded trace events.
  scryrs record --stdin
      Ingest JSONL trace events from stdin.
  scryrs record --file <PATH>
      Ingest JSONL trace events from a file.
  scryrs hook <HARNESS> [--stdin | --file <PATH>]
      Translate a harness's native tool event and record it (fail-open).
      Supported harnesses: claude-code (stdin), pi (--file).
  scryrs init --agent <NAME>
      Install the scryrs trace hook for a supported agent harness (hook only).
      Supported harnesses: claude-code, pi
  scryrs setup <local|live> [--ingest-url <URL>] [--workspace-id <ID>] [--with-compose] [--docker-network <NAME>]
      Configure local or live trace transport (writes scryrs.json remote / .scryrs/).
  scryrs dashboard [--port <PORT>] [--bind <ADDR>] [--no-open] [--dev]
      Start local dashboard server and open the browser dashboard.
  scryrs server [--bind <ADDR>] [--port <PORT>] [--store <PATH>]
      Start the central trace ingest server for POST /v1/trace-events/batch.

RECORD MODES
  Remote mode (default): submits to the configured ingest server.
      Identity resolves by precedence — flags, then environment, then
      .scryrs/.env, then scryrs.json `remote` — using SCRYRS_REMOTE_INGEST_URL,
      SCRYRS_REPOSITORY_ID, SCRYRS_WORKSPACE_ID, SCRYRS_AGENT_ID,
      SCRYRS_REMOTE_TIMEOUT_MS.
      Remote mode skips SQLite entirely (no dual-write, no local fallback).
      Unresolved required config fails fast (exit 2) with guidance.
      Default timeout: 3000 ms.
  Local mode (--mode local): persisted to .scryrs/scryrs.db, no network calls.

RECORD OUTPUT
  A single-line JSON summary on stdout.
  Rejection diagnostics are written as JSON objects to stderr,
  one per rejected non-empty line.

HOTSPOTS OUTPUT
  A single-line JSON envelope on stdout:

EXAMPLES
  scryrs hotspots /path/to/repo
  scryrs hotspots .
  scryrs record --stdin < events.jsonl
  scryrs record --file session.jsonl
  scryrs hook claude-code < pre-tool-use.json
  scryrs hook pi --file event.json
  scryrs init --agent claude-code
  scryrs init --agent pi
  scryrs setup local
  scryrs setup live --ingest-url http://scryrs:8081 --workspace-id my-workspace
  scryrs dashboard
  scryrs dashboard --port 9090 --no-open
  scryrs server
  scryrs server --port 9091

OPTIONS
  -h, --help       Print this help message and exit
  -V, --version    Print version and exit
  -hj, --help-json Print machine-readable CLI surface description and exit

EXIT CODES
  0    Success (hotspots: JSON written; record local: all events accepted; record remote: no rejections or failures; init: hook installed; dashboard: server shut down cleanly; server: server shut down cleanly; hook: always — fail-open, never blocks the harness)
  1    Hotspots: storage error. Record: rejected events or I/O error (local or server rejections). Init: I/O error. Dashboard: port in use or artifact read error. Server: port in use or store error.
  2    Usage error; hotspots: missing/unsupported store; record: also fatal I/O error (unreadable file, store failure, missing remote identity, transport timeout, connection failure, non-2xx response, malformed response); init: unsupported harness, collision, or self-install refusal; dashboard: invalid flags or bind failure; server: invalid flags or bind failure
```

**`--version`** prints the binary version:

```bash
$ cargo run -p scryrs-cli -- --version
scryrs 0.1.0
```

**`--help-json`** prints a machine-readable surface description:

```bash
$ cargo run -p scryrs-cli -- --help-json
{"surfaceVersion":"0.3.0","binary":"scryrs","commands":[{"name":"hotspots",...},{"name":"record",...},{"name":"init",...}]}
```

The JSON document describes every command, argument, flag, output field, and exit code — suitable for parsing by tooling or agents. Use `scryrs --help-json` directly for the full surface document.

### Run the hotspot command

```bash
$ cargo run -p scryrs-cli -- hotspots .
{"schemaVersion":"1.0.0","command":"hotspots","repositoryPath":"/abs/path","storePath":"/abs/path/.scryrs/scryrs.db","runMetadata":{"storeSchemaVersion":1,"analyzedEventCount":0,"analyzedSubjectCount":0,"firstEventId":0,"lastEventId":0},"generatedAt":"2026-06-21T12:00:00Z","entries":[]}
```

This emits a single-line JSON envelope containing the `HotspotsReport` schema.
If the `.scryrs/scryrs.db` datastore exists and contains trace events, `entries`
will contain ranked `HotspotEntry` objects with scores, per-event-type counts,
per-outcome counts, session breadth, time spans, and evidence row IDs.
An empty datastore (or one with only lifecycle events) produces `entries: []`
with exit code 0. Missing or unsupported datastores produce exit code 2
with an error message on stderr.

The report is also written to `.scryrs/hotspots.json` at the repository root.

### Error paths

The CLI follows a three-code exit convention with command-specific semantics:

| Exit code | Meaning     |
|-----------|-------------|
| 0         | Success (hotspots: JSON written, including empty entries; record local: all events accepted; record remote: no rejections or failures; init: hook installed; dashboard: server shut down cleanly; server: server shut down cleanly; hook: always — fail-open, never blocks the harness) |
| 1         | Hotspots: storage error. Record: rejected events or I/O error (local or server). Init: I/O error. Dashboard: port in use or artifact read error. Server: port in use or store error. |
| 2         | Usage error; hotspots: missing/unsupported store; record: also fatal I/O error (unreadable file, store failure, missing remote identity, transport timeout, connection failure, non-2xx response, malformed response); init: unsupported harness, collision, or self-install refusal; dashboard: invalid flags or bind failure; server: invalid flags or bind failure |

**Missing required argument** — exit 2:

```bash
$ cargo run -p scryrs-cli -- hotspots
scryrs hotspots: missing required PATH argument
Usage: scryrs hotspots <PATH>
See `scryrs --help`
# exit code: 2
```

### Current limitations

- **Nine commands:** `hotspots`, `record`, `hook`, `init`, `dashboard`, `server`, `graph`, `route`, and `propose` are the supported commands. Everything else (`trace`, `adapters`, `report`, `suggest-docs`) produces an "unknown command" error.
- **Hotspot analysis:** `hotspots` reads from `.scryrs/scryrs.db` and produces ranked `HotspotEntry` results. Empty or missing stores produce distinct exit codes.
- **Record is ingestion-only:** `scryrs record` validates and persists trace events. It does not trigger hotspot analysis, graph building, or other downstream processing.
- **What's not listed:** No speculative future commands or features appear here. The quickstart documents exactly what exists today.

### Troubleshooting

| Symptom | Likely cause | Fix |
|---------|-------------|-----|
| `unknown command: 'X'` | You ran a command that doesn't exist yet | Run `cargo run -p scryrs-cli -- --help` to see available commands |
| `missing required PATH argument` | You ran `hotspots` without a path | Add a path: `cargo run -p scryrs-cli -- hotspots .` |
| `port in use` | Another process is already using the dashboard or server port | Use `--port` to pick a different port, or stop the other process |
| `error[E0463]: can't find crate` | Missing Rust toolchain or wrong directory | Install Rust 1.85+ via rustup, ensure you're in the repo root |
| Build hangs or runs out of memory | First build compiles many dependencies | It's normal for a fresh `cargo build`. Subsequent builds use cached artifacts and are much faster. |

## Current status

Current CLI surface ships full local evidence loop plus first graph, route, and proposal artifacts. `scryrs record` ingests JSONL trace events via `--stdin` or `--file <PATH>`, validates against the shared `TraceEvent` schema, and submits accepted events to the live server by default (or persists locally with `--mode local`). `scryrs hotspots <PATH>` scores subjects with deterministic weights and writes `.scryrs/hotspots.json`. `scryrs graph <PATH>` builds deterministic `KnowledgeGraphDocument` output from hotspots plus optional docs navigation metadata. `scryrs route <PATH>` projects `.scryrs/graph.json` into deterministic `.scryrs/routes.json`. `scryrs propose <PATH>` writes validated review-only `ProposalDocument` artifacts under `.scryrs/proposals/`. `scryrs dashboard` serves the hotspot UI (live by default, or local with `--mode local`). `scryrs server` starts the central live-ingest server at `POST /v1/trace-events/batch` with live hotspot query and SSE signal streaming. Optional model-assisted curation is present only as library crate `crates/scryrs-curator-llm`; no model-backed CLI path exists.

## Local checks

```bash
scripts/check
scripts/test
scripts/security
scripts/precommit-run
```
