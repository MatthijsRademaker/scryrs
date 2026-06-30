# Live Server Setup

This guide is the operational companion to [Live Hotspots](./live-hotspots.md). Where that page explains *what* live mode is and *why* you would use it, this page is purely about *running it*: a two-minute quickstart, then an extensive install-and-configure reference covering the single-binary path, the Docker image, the multi-agent Compose stack, connecting agents and the dashboard, verifying the install, and troubleshooting.

For the exact endpoint tables, JSON schemas, and exit codes, see the [CLI Reference](./cli-v0-contract.md). For hook and identity semantics, see the [Trace Hook Contract](./trace-hook-contract.md).

## Quickstart

Get a shared live hotspot server running and an agent feeding it in under two minutes.

### Option A — Workspace-local live bootstrap (recommended for multi-agent)

Run live bootstrap in the **consumer workspace**, not in the scryrs source checkout:

```bash
# 1. Configure live ingest + workspace-local bootstrap.
#    Live mode is the default; repository_id falls back to Git origin when omitted.
scryrs init --agent claude-code \
  --ingest-url http://scryrs:8081 \
  --workspace-id my-workspace \
  --agent-id agent-1 \
  --docker-network my-agent-network

# 2. Start the managed live server from the generated scaffold.
scryrs up
```

That writes the committed live config (`ingest_url`, `workspace_id`,
`docker_network`) into `scryrs.json` `remote` — the single committed source of
truth — and scaffolds `.scryrs/compose.yml` plus an overrides-only `.scryrs/.env`
in the current workspace. The generated Compose service joins the existing
external Docker network named by `remote.docker_network` and is reachable there
as `http://scryrs:8081`.

```bash
# 3. Confirm it is serving (empty repo returns an empty ranking, not an error)
curl -s http://localhost:8081/v1/repositories/demo-repo/hotspots
# => {"schemaVersion":"1.0.0","repositoryId":"demo-repo","entries":[],"generatedAt":"..."}
```

The generated `.scryrs/compose.yml` references the published image
`ghcr.io/matthijsrademaker/scryrs-server:latest`, so `scryrs up` pulls it
automatically — no source checkout or local image build is required. The image
tracks `:latest`; run `docker compose pull` (or recreate the stack) to update.
The repository-root `Dockerfile` and `docker-compose.yml` are packaging /
maintainer assets; ordinary consumer workspaces use the generated `.scryrs/`
scaffold and the published image instead.

### Option B — Single binary (local / single host)

```bash
# 1. Install the CLI (one-shot: downloads the published binary, installs to ~/.local/bin)
curl -fsSL https://raw.githubusercontent.com/matthijsrademaker/scryrs/main/install.sh | sh

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
| One-shot binary (Option B, recommended) | Just `curl` (or `wget`) and `sha256sum`/`shasum`. No Rust toolchain. Downloads a prebuilt macOS arm64 or Linux x86_64 binary. |
| Docker (Option A) | **Docker** with the Compose plugin. `scryrs up` pulls the published `ghcr.io` image; no host Rust toolchain needed. |
| From source (contributors) | **Rust 1.85+** (install via [rustup](https://rustup.rs)), plus a C toolchain for the bundled SQLite. macOS and Linux are supported by `scripts/install`. |

The server has no external service dependencies. State lives entirely in one
server-owned SQLite file; there is no separate database to provision.

## Install paths

### 1. One-shot binary (recommended)

The fastest path needs no Rust toolchain or source checkout. It downloads the
published release binary for your platform (**macOS arm64** or **Linux x86_64**),
verifies it against its `.sha256` checksum, and installs it onto your `PATH`.

```bash
curl -fsSL https://raw.githubusercontent.com/matthijsrademaker/scryrs/main/install.sh | sh

# custom directory:
curl -fsSL https://raw.githubusercontent.com/matthijsrademaker/scryrs/main/install.sh | sh -s -- --bin-dir /usr/local/bin

# pin a release tag (default: latest):
curl -fsSL https://raw.githubusercontent.com/matthijsrademaker/scryrs/main/install.sh | SCRYRS_VERSION=v0.1.0 sh
```

> **macOS note:** if Gatekeeper quarantines the downloaded binary, clear the flag
> with `xattr -d com.apple.quarantine "$HOME/.local/bin/scryrs"`.

### 2. From source with `scripts/install`

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

### 3. Docker image

Released versions are published to GitHub Container Registry as
`ghcr.io/matthijsrademaker/scryrs-server` (`linux/amd64`), tagged with each
version and `latest`. Pull and run it directly — no source checkout or build:

```bash
# Pull and run the published image with a persistent named volume
docker run -d --name scryrs-server \
  -p 8081:8081 \
  -v scryrs-data:/data/scryrs \
  ghcr.io/matthijsrademaker/scryrs-server:latest
```

For maintainer/dev builds the repository also ships a `Dockerfile` that builds
the CLI with `--features server,core` into a minimal Debian runtime image:

```bash
docker build -t scryrs-server .
```

The image's entrypoint is fixed to:

```text
scryrs server --bind 0.0.0.0 --port 8081 --store /data/scryrs/server.db
```

It binds all interfaces (so the container is reachable), exposes port `8081`, and
persists state to the `/data/scryrs` volume.

### 4. Repository packaging / maintainer Compose stack

The repository-root `docker-compose.yml` is still useful for maintainers: it
builds `scryrs-server` from source and smoke-checks the packaged container.
It is **not** the normal consumer bootstrap path.

```bash
docker compose up -d        # build + start from source checkout
docker compose logs -f      # watch startup and ingest activity
docker compose down         # stop (the scryrs-data volume is retained)
```

That stack provides:

- **Service name `scryrs-server`** — convenient for repo-local maintainer workflows.
- **Named volume `scryrs-data`** — mounted at `/data/scryrs`; survives container
  recreation. The server stores events at `/data/scryrs/server.db`.
- **Dedicated network `scryrs-net`** — suitable for repo-root smoke checks only.
- **`restart: unless-stopped`** — the server comes back after host or daemon restarts.

For consumer workspaces, `scryrs init --mode live` instead generates
`.scryrs/compose.yml`, which attaches the service to an **existing external
agent network** and publishes the `http://scryrs:8081` contract there.

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
scryrs source checkout) configures remote ingest **and** scaffolds the workspace-local
server bootstrap used later by `scryrs up`.

```bash
scryrs init --agent claude-code \
  --ingest-url http://scryrs:8081 \
  --workspace-id my-workspace \
  --agent-id agent-1 \
  --docker-network my-agent-network
```

Supported harnesses are `claude-code` and `pi`. Live-mode init:

- Resolves the committed live config (`ingest_url`, `workspace_id`,
  `docker_network`) from the precedence chain (see below).
- **Fails fast before writes** if `ingest_url`, `workspace_id`, or `docker_network`
  is missing, `repository_id` is underivable, or an existing committed
  `scryrs.json` value conflicts.
- Writes only the committed shared constants (`ingest_url`, `workspace_id`,
  `docker_network`) into the `remote` section of project-root `scryrs.json`,
  preserving unrelated keys. It does **not** write `repository_id` or `agent_id`.
- Scaffolds `.scryrs/compose.yml` and an overrides-only gitignored `.scryrs/.env`
  stub — no managed identity or network values are pre-populated there.
- Installs the requested harness hook transport.
- Does **not** scaffold a local `.scryrs/scryrs.db` — every event flows to the server.

`--repository-id` and `--agent-id` are optional and are never committed. When
`repository_id` is omitted, resolution derives a stable identity from the
project's Git remote origin URL (so two clones of the same repository share one
live state). When `agent_id` is omitted, it is autogenerated at runtime from the
container hostname, stable across the per-tool-call hook processes in one
container.

Because `scryrs.json` is committed, a fresh `git clone` of a live-configured
project resolves to live mode without re-running `init`.

### Configuring identity: `scryrs.json` as the committed source of truth

The committed live constants live in project-root **`scryrs.json`** `remote`.
Gitignored **`.scryrs/.env`** is for local, per-machine overrides only. The CLI
resolves remote identity by precedence — highest wins:

1. **CLI flags** (`--ingest-url`, `--workspace-id`, `--agent-id`, `--repository-id`)
2. **Process environment** (`SCRYRS_REMOTE_*`)
3. **`.scryrs/.env`** dotenv file
4. **`scryrs.json` `remote`** section (committed base)

The Docker network used by `scryrs up` resolves by the same shaped chain, with the
committed manifest as the base layer:

1. **CLI flag** `--docker-network`
2. **Process environment** `SCRYRS_DOCKER_NETWORK`
3. **`.scryrs/.env`** dotenv file
4. **`scryrs.json` `remote.docker_network`** (committed base)

`repository_id` falls back to the Git remote origin and `agent_id` to the
container hostname when unresolved by every layer.

```json
// scryrs.json (committed — the single source of truth for live config)
{
  "remote": {
    "ingest_url": "http://scryrs:8081",
    "workspace_id": "my-workspace",
    "docker_network": "my-agent-network"
  }
}
```

```bash
# .scryrs/.env (gitignored — local overrides only; nothing managed is written here)
# SCRYRS_REMOTE_INGEST_URL=http://scryrs:8081
# SCRYRS_REPOSITORY_ID=my-repo          # optional; derived from Git origin if omitted
# SCRYRS_WORKSPACE_ID=my-workspace
# SCRYRS_AGENT_ID=agent-1               # optional; autogenerated per container if omitted
# SCRYRS_REMOTE_TIMEOUT_MS=3000         # optional; defaults to 3000
# SCRYRS_DOCKER_NETWORK=my-agent-network
```

| Field / variable | Purpose |
|----------|---------|
| `remote.ingest_url` / `SCRYRS_REMOTE_INGEST_URL` | Server base URL. For container-attached agents, use `http://scryrs:8081`. Required for live mode. |
| `remote.workspace_id` / `SCRYRS_WORKSPACE_ID` | Workspace identity component of the dedup key. Required (committed). |
| `remote.docker_network` / `SCRYRS_DOCKER_NETWORK` | Existing external Docker network that both agents and the managed scryrs service join. Required (committed). |
| `SCRYRS_REPOSITORY_ID` | Explicit repository identity override (otherwise derived from the Git remote origin; not committed). |
| `SCRYRS_AGENT_ID` | Explicit agent identity override (otherwise autogenerated per container; not committed). |
| `SCRYRS_REMOTE_TIMEOUT_MS` | Per-request transport timeout. Default `3000` ms. |

When the live default is active but the ingest URL, `workspace_id`, or the Docker
network cannot be resolved from any layer (or `repository_id` is underivable), the
command exits `2` and prints guidance. It never silently degrades to local mode.

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
| `scryrs init: live mode is the default but docker_network is not configured` (exit 2) | Workspace bootstrap has no external network name | Set `--docker-network <NAME>`, commit `remote.docker_network` in `scryrs.json`, or set `SCRYRS_DOCKER_NETWORK` / `.scryrs/.env`. |
| `scryrs up: SCRYRS_DOCKER_NETWORK could not be resolved from any layer` (exit 2) | No committed `remote.docker_network` and no override | Commit `remote.docker_network` in `scryrs.json` (rerun `scryrs init` with `--docker-network`), or set `SCRYRS_DOCKER_NETWORK` / `.scryrs/.env`. |
| `scryrs up: missing required scaffold file ...` (exit 2) | Workspace has not been bootstrapped yet | Run `scryrs init --agent <NAME>` in live mode first. |
| `scryrs up: external Docker network '...' does not exist` (exit 2) | Named external network is missing | Create the network first, then rerun `scryrs up`. |
| `scryrs server: ... port ... in use` (exit 1) | Another process holds the port | Pick another `--port`, or stop the other process. |
| `scryrs server: unavailable (server feature not enabled)` (exit 2) | Binary built without the `server` feature | Reinstall with default features (`scripts/install`) or `cargo build -p scryrs-cli --features server,core --release`. |
| `invalid --port value '0'` / `invalid --bind value` (exit 2) | Bad flag value | Port must be 1–65535; bind must be a valid IP address. |
| Agents on other hosts/containers cannot connect | Server bound to `127.0.0.1` or workspace bootstrap not started | Bind `0.0.0.0` for host-managed server, or run `scryrs up` so the managed container publishes port `8081`. |
| Agent container cannot resolve `scryrs` | Not attached to the shared external network | Attach both agent containers and the managed scryrs service to the same external Docker network named by `SCRYRS_DOCKER_NETWORK`. |
| Events submitted but rankings stay empty | Lifecycle-only events, or wrong `repository_id` in the query | Only subject-bearing events score. Confirm the query's `{id}` matches the agents' repository identity. |
| Duplicate submissions don't raise scores | Working as designed | Dedup is first-writer-wins on `(repo, workspace, agent, producer_event_id)`. |
| Live state vanished after `docker compose down -v` | `-v` removed the `scryrs-data` volume | State is unrecoverable; omit `-v` to retain the volume next time. |

## Related pages

- [Live Hotspots](./live-hotspots.md) — what live mode is, the end-to-end pipeline, and field interpretation
- [CLI Reference](./cli-v0-contract.md) — complete endpoint tables, JSON schemas, and exit codes
- [Trace Hook Contract](./trace-hook-contract.md) — hook capture and the `ServerIngestEnvelope` identity semantics
- [Hotspots](./hotspots.md) — local batch hotspots and scoring rationale
- [Architecture](./architecture.mdx) — `scryrs-server` accumulator and signal-streaming design
