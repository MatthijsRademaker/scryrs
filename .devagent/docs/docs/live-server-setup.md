# Live Server Setup

This guide is the operational companion to [Live Hotspots](./live-hotspots.md). Where that page explains *what* live mode is and *why* you would use it, this page is purely about *running it*: a two-minute quickstart, then an extensive install-and-configure reference covering the single-binary path, the Docker image, the multi-agent Compose stack, connecting agents and the dashboard, verifying the install, and troubleshooting.

For the exact endpoint tables, JSON schemas, and exit codes, see the [CLI Reference](./cli-v0-contract.md). For hook and identity semantics, see the [Trace Hook Contract](./trace-hook-contract.md).

## Quickstart

Get a shared live hotspot server running and an agent feeding it in under two minutes.

### Option A — Docker Compose (recommended for multi-agent)

From the scryrs source checkout:

```bash
# 1. Build and start the containerized live server
docker compose up -d
```

This builds the `scryrs-server` image and starts it bound to `0.0.0.0:8081` with a
persistent SQLite store at `/data/scryrs/server.db`, on an attachable Docker
network named `scryrs-net`.

```bash
# 2. Confirm it is serving (empty repo returns an empty ranking, not an error)
curl -s http://localhost:8081/v1/repositories/demo-repo/hotspots
# => {"schemaVersion":"1.0.0","repositoryId":"demo-repo","entries":[],"generatedAt":"..."}
```

```bash
# 3. In each agent workspace (NOT the scryrs source checkout), configure identity.
#    Live mode is the DEFAULT, so init just needs resolvable config. Put it in
#    .scryrs/.env (gitignored) — init scaffolds this file, but you can pre-create it:
mkdir -p .scryrs
cat > .scryrs/.env <<'EOF'
SCRYRS_REMOTE_INGEST_URL=http://scryrs-server:8081
SCRYRS_WORKSPACE_ID=my-workspace
SCRYRS_AGENT_ID=agent-1
EOF

# Then install hooks — no --mode flag needed; live is the default:
scryrs init --agent claude-code
```

`SCRYRS_REPOSITORY_ID` is derived from the Git remote origin when omitted. Agents
now stream trace events to the shared server. Query rankings any time at
`GET /v1/repositories/{id}/hotspots`, or tail threshold-crossing signals at
`GET /v1/repositories/{id}/signals`.

### Option B — Single binary (local / single host)

```bash
# 1. Install the CLI (builds in release mode, installs to ~/.local/bin)
./scripts/install

# 2. Start the server in the foreground (binds 127.0.0.1:8081, store .scryrs/server.db)
scryrs server

# 3. From another shell, configure an agent workspace and install hooks.
#    Live is the default; flags override .scryrs/.env when you prefer them inline:
scryrs init --agent claude-code \
  --ingest-url http://127.0.0.1:8081 \
  --workspace-id my-workspace \
  --agent-id agent-1
```

That is the whole loop. The rest of this page covers each step in depth.

> **Local mode is still available** as an explicit opt-in: `scryrs init --agent
> claude-code --mode local` scaffolds the single-machine `.scryrs/scryrs.db` store
> and makes no network calls. See [Live vs local](#live-vs-local-pick-one).

## Prerequisites

| Path | Requirements |
|------|--------------|
| Docker (Option A) | **Docker** with the Compose plugin. No host Rust toolchain needed — the image builds the binary internally. |
| From source (Option B) | **Rust 1.85+** (install via [rustup](https://rustup.rs)), plus a C toolchain for the bundled SQLite. macOS and Linux are supported by `scripts/install`. |

The server has no external service dependencies. State lives entirely in one
server-owned SQLite file; there is no separate database to provision.

## Install paths

### 1. From source with `scripts/install`

`scripts/install` builds `scryrs-cli` in release mode with default features and
copies the `scryrs` binary onto your `PATH`.

```bash
git clone <repo-url>   # replace with the actual clone URL
cd scryrs
./scripts/install
```

By default the binary lands in `$HOME/.local/bin`. Override the target directory:

```bash
SCRYRS_INSTALL_DIR=/usr/local/bin ./scripts/install
# or
./scripts/install --bin-dir /usr/local/bin
```

After install the script verifies the binary with `scryrs --version` and, if the
target directory is not already on your `PATH`, prints the exact line to add to
your shell profile.

> **Note:** `scripts/install` installs **only** the CLI binary. It does not create
> `.scryrs/`, `scryrs.json`, agent hooks, or shell profile entries. Hook setup is a
> separate step (`scryrs init`, below).

The default feature set already includes the `server` and `core` features, so the
installed binary can run `scryrs server` immediately. To build manually instead:

```bash
cargo build -p scryrs-cli --release
./target/release/scryrs server --help
```

### 2. Docker image

The repository ships a `Dockerfile` that builds the CLI with `--features server,core`
and produces a minimal Debian runtime image.

```bash
# Build the image
docker build -t scryrs-server .

# Run it standalone with a persistent named volume
docker run -d --name scryrs-server \
  -p 8081:8081 \
  -v scryrs-data:/data/scryrs \
  scryrs-server
```

The image's entrypoint is fixed to:

```text
scryrs server --bind 0.0.0.0 --port 8081 --store /data/scryrs/server.db
```

It binds all interfaces (so the container is reachable), exposes port `8081`, and
persists state to the `/data/scryrs` volume.

### 3. Multi-agent Compose stack

`docker-compose.yml` wraps the image with persistent storage and a stable,
peer-resolvable service name. This is the intended deployment for several agent
containers sharing one source of truth.

```bash
docker compose up -d        # build + start
docker compose logs -f      # watch startup and ingest activity
docker compose down         # stop (the scryrs-data volume is retained)
```

The stack provides:

- **Service name `scryrs-server`** — reachable as `http://scryrs-server:8081` from
  any container on the `scryrs-net` network.
- **Named volume `scryrs-data`** — mounted at `/data/scryrs`; survives container
  recreation. The server stores events at `/data/scryrs/server.db`.
- **Network `scryrs-net`** — an attachable network for peer agent containers.
- **`restart: unless-stopped`** — the server comes back after host or daemon restarts.

**Attaching agent containers** so they can resolve the server by name:

```bash
docker run --network scryrs-net ...
```

or add to your agent's Compose service:

```yaml
services:
  my-agent:
    # ...
    networks: [scryrs-net]
```

## Running the server

The default invocation needs no flags — it binds `127.0.0.1:8081` and stores at
`.scryrs/server.db`:

```bash
scryrs server
```

All three settings are optional overrides:

```bash
scryrs server [--bind <ADDR>] [--port <PORT>] [--store <PATH>]
```

| Flag | Default | Meaning |
|------|---------|---------|
| `-b, --bind <ADDR>` | `127.0.0.1` | Interface to bind. Use `0.0.0.0` to accept connections from other hosts/containers. |
| `-p, --port <PORT>` | `8081` | TCP port (1–65535; `0` is rejected). |
| `--store <PATH>` | `.scryrs/server.db` | Path to the server-owned SQLite store. Created if missing. |

The startup message prints the listen address and store path to **stderr**. The
process is long-lived and shuts down cleanly on signal (exit `0`).

The server exposes exactly three endpoints (full schemas in the [CLI Reference](./cli-v0-contract.md)):

| Method & path | Purpose |
|---------------|---------|
| `POST /v1/trace-events/batch` | Ingest a `ServerIngestEnvelope`; returns accepted/duplicate/rejected counts with per-item diagnostics. |
| `GET /v1/repositories/{id}/hotspots` | Live rankings. Supports `?window=cumulative` and optional `?session_id=<id>`. |
| `GET /v1/repositories/{id}/signals` | SSE stream of `HotspotSignal` records. Supports `?after=<signal_id>` for replay/resume. |

### Signal threshold

A `HotspotSignal` fires the first time a subject's cumulative score crosses from
below to at-or-above the threshold, which defaults to **`10`**
(`DEFAULT_SIGNAL_THRESHOLD`). The threshold is fixed at this default in the current
CLI; it is not adjustable via flag or environment variable. The SSE stream sends a
**15-second keep-alive heartbeat** so idle connections do not time out.

### Exit codes

| Code | Meaning |
|------|---------|
| `0` | Server shut down cleanly. |
| `1` | Runtime failure — port already in use, or store open/IO error. |
| `2` | Usage error — invalid `--port`, invalid `--bind`, empty `--store`, or the `server` feature not compiled in. |

## Connecting agents (live ingest)

Live mode is the **default**. Running `scryrs init` in an agent workspace (not the
scryrs source checkout) configures remote ingest as long as the required identity
resolves:

```bash
scryrs init --agent claude-code
```

Supported harnesses are `claude-code` and `pi`. Live-mode init:

- Resolves remote identity from the precedence chain (see below) and **fails fast
  with guidance** if the ingest URL or required identity is missing.
- Creates or merges a `remote` section in the project root `scryrs.json`.
- Scaffolds a gitignored `.scryrs/.env` template (without clobbering existing values).
- Installs the same harness hook transport.
- Does **not** scaffold a local `.scryrs/scryrs.db` — every event flows to the server.

`--repository-id` is optional; when omitted, init derives a stable repository
identity from the project's Git remote origin URL. Two clones of the same
repository therefore share one live state on the server.

### Configuring identity: `.scryrs/.env` and precedence

Remote identity lives in a gitignored **`.scryrs/.env`** dotenv file in the project
root. The CLI resolves each field by precedence — highest wins:

1. **CLI flags** (`--ingest-url`, `--workspace-id`, `--agent-id`, `--repository-id`)
2. **Process environment** (`SCRYRS_REMOTE_*`)
3. **`.scryrs/.env`** dotenv file
4. **`scryrs.json` `remote`** section

`repository_id` falls back to the Git remote origin when unresolved by every layer.

```bash
# .scryrs/.env (gitignored — never commit per-agent identity)
SCRYRS_REMOTE_INGEST_URL=http://scryrs-server:8081
SCRYRS_REPOSITORY_ID=my-repo          # optional; derived from Git origin if omitted
SCRYRS_WORKSPACE_ID=my-workspace
SCRYRS_AGENT_ID=agent-1
SCRYRS_REMOTE_TIMEOUT_MS=3000         # optional; defaults to 3000
```

| Variable | Purpose |
|----------|---------|
| `SCRYRS_REMOTE_INGEST_URL` | Server base URL. Required for live mode. |
| `SCRYRS_REPOSITORY_ID` | Explicit repository identity (overrides Git-derived). |
| `SCRYRS_WORKSPACE_ID` | Workspace identity component of the dedup key. Required. |
| `SCRYRS_AGENT_ID` | Agent identity component of the dedup key. Required. |
| `SCRYRS_REMOTE_TIMEOUT_MS` | Per-request transport timeout. Default `3000` ms. |

When the live default is active but the ingest URL or a required identity field
cannot be resolved from any layer, the command exits `2` and prints guidance
naming the missing field and both remediation paths (populate `.scryrs/.env`, or
rerun with `--mode local`). It never silently degrades to local mode.

Remote mode skips SQLite entirely — there is no dual-write and no local fallback.
Deduplication is first-writer-wins on the composite key
`(repository_id, workspace_id, agent_id, producer_event_id)`, so retries and
reconnections never double-count.

### Recording manually

`scryrs record` follows the same default: remote transport unless `--mode local`
is passed. `scryrs record --file session.jsonl` submits to the configured server;
`scryrs record --mode local --file session.jsonl` writes to `.scryrs/scryrs.db`.

## Connecting the dashboard (live read path)

The browser dashboard also defaults to **live mode**, proxying the server contract
over same-origin `/api/*` calls. Targets resolve from the same precedence chain, so
a configured `.scryrs/.env` is enough:

```bash
scryrs dashboard
# or override inline:
scryrs dashboard --server-url http://127.0.0.1:8081 --repository-id my-repo
```

If neither flags nor `.scryrs/.env`/`scryrs.json` resolve a server URL, startup
fails `2` with guidance. Use `scryrs dashboard --mode local` to read local
`.scryrs` artifacts instead. In live mode:

- `GET /api/meta` reports `mode: "live"` and the configured `repositoryId`.
- `GET /api/hotspots` proxies the cumulative live ranking.
- `GET /api/signals?after=<id>` streams replayed-plus-live signals; the browser
  owns reconnect, resuming from the last seen SSE id and ignoring replay duplicates.

Local-only views (Sessions, Events) are hidden in live mode. See
[Live Hotspots → Live Dashboard Mode](./live-hotspots.md#live-dashboard-mode) for
the full local-vs-live behavior matrix.

## Verifying the install

**1. Server health (any repository id returns a well-formed empty response):**

```bash
curl -s http://localhost:8081/v1/repositories/demo-repo/hotspots | head -c 200
# {"schemaVersion":"1.0.0","repositoryId":"demo-repo","entries":[], ...}
```

**2. Signal stream connects (SSE; Ctrl-C to stop):**

```bash
curl -N http://localhost:8081/v1/repositories/demo-repo/signals?after=0
# streams persisted signals from cursor 0, then tails new ones, with periodic
# keep-alive comments every 15s
```

**3. End-to-end fixture (Docker-backed, no host Rust/Node required):**

```bash
scripts/verify-live-hotspots
```

This builds the real `scryrs` binary in Docker and drives the full
capture → submit → ingest → query → signal loop against it, reporting `PASSED`
or `FAILED`.

## Persistence and data

- All state lives in the single `--store` SQLite file. Back up or snapshot that one
  file to preserve cumulative hotspot state and the append-only signal log.
- Under Docker, the file is on the `scryrs-data` named volume at
  `/data/scryrs/server.db`. `docker compose down` keeps the volume; remove it
  explicitly with `docker compose down -v` (this permanently deletes all live state).
- The store is the single writer of record. Do not point two `scryrs server`
  processes at the same store file concurrently.

## Live vs local: pick one

Live mode and local batch mode are **exclusive deployment choices**, not layers.
Server-owned state does not merge with any pre-existing local `.scryrs/scryrs.db`.
`init`, `record`, and `dashboard` now **default to live**; local is the explicit
`--mode local` opt-in.

- **Single developer / single agent on one machine →** `--mode local`
  (`scryrs init --agent claude-code --mode local`, then `scryrs hotspots .`).
  No server, no network.
- **Multiple agents — CI workers, parallel containers, a shared harness workspace →**
  the default live mode. One server, one source of truth, instant signals.

The full dimension-by-dimension comparison lives in
[Live Hotspots → Live Mode vs Local Batch](./live-hotspots.md#live-mode-vs-local-batch-hotspots).

## Troubleshooting

| Symptom | Likely cause | Fix |
|---------|--------------|-----|
| `scryrs init/record/dashboard: live mode is the default but … is not configured` (exit 2) | Live is the default and no ingest URL/identity resolved | Populate `.scryrs/.env` (`SCRYRS_REMOTE_INGEST_URL`, `SCRYRS_WORKSPACE_ID`, `SCRYRS_AGENT_ID`), or rerun with `--mode local`. |
| `scryrs server: ... port ... in use` (exit 1) | Another process holds the port | Pick another `--port`, or stop the other process. |
| `scryrs server: unavailable (server feature not enabled)` (exit 2) | Binary built without the `server` feature | Reinstall with default features (`scripts/install`) or `cargo build -p scryrs-cli --features server,core --release`. |
| `invalid --port value '0'` / `invalid --bind value` (exit 2) | Bad flag value | Port must be 1–65535; bind must be a valid IP address. |
| Agents on other hosts/containers cannot connect | Server bound to `127.0.0.1` | Bind `0.0.0.0` (the Docker image already does). For Compose, ensure agents are on `scryrs-net`. |
| Agent container cannot resolve `scryrs-server` | Not attached to the shared network | Add `networks: [scryrs-net]` or run with `--network scryrs-net`. |
| Events submitted but rankings stay empty | Lifecycle-only events, or wrong `repository_id` in the query | Only subject-bearing events score. Confirm the query's `{id}` matches the agents' repository identity. |
| Duplicate submissions don't raise scores | Working as designed | Dedup is first-writer-wins on `(repo, workspace, agent, producer_event_id)`. |
| Live state vanished after `docker compose down -v` | `-v` removed the `scryrs-data` volume | State is unrecoverable; omit `-v` to retain the volume next time. |

## Related pages

- [Live Hotspots](./live-hotspots.md) — what live mode is, the end-to-end pipeline, and field interpretation
- [CLI Reference](./cli-v0-contract.md) — complete endpoint tables, JSON schemas, and exit codes
- [Trace Hook Contract](./trace-hook-contract.md) — hook capture and the `ServerIngestEnvelope` identity semantics
- [Hotspots](./hotspots.md) — local batch hotspots and scoring rationale
- [Architecture](./architecture.mdx) — `scryrs-server` accumulator and signal-streaming design
