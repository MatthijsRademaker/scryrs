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

The CLI ships three commands:

```bash
# Hotspot analysis placeholder (v0)
cargo run -p scryrs-cli -- hotspots /path/to/repo

# JSONL trace event ingestion
cargo run -p scryrs-cli -- record --stdin < events.jsonl
cargo run -p scryrs-cli -- record --file session.jsonl

# Hook installation for supported harnesses
cargo run -p scryrs-cli -- init --agent claude-code
cargo run -p scryrs-cli -- init --agent pi
```

Default features include the standalone suite, Markdown adapter, runtime, and deterministic guardrail support. `full` adds the optional LLM boundary and Rspress adapter.

## Quickstart

Get from a freshly cloned repo to your first command in under two minutes.

### Prerequisites

- **Rust 1.85+** (install via [rustup](https://rustup.rs)) — or —
- **Docker** (the workspace ships a dev container with all tooling pre-installed)

### Install from source (recommended)

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

### Install agent hooks (after the CLI binary is on PATH)

Once `scryrs` is installed and reachable on `PATH`, install trace hooks for your
agent harness:

```bash
scryrs init --agent claude-code
scryrs init --agent pi
```

**Note:** `scripts/install` only installs the CLI binary. It does not create or modify
`.claude/`, `.pi/`, `.scryrs/`, `scryrs.json`, git hooks, or shell profile files.
Hook installation is a separate step performed by `scryrs init --agent <NAME>`
after the `scryrs` binary is on your `PATH`.

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
      Emit a versioned JSON placeholder for repository hotspots.
  scryrs record --stdin
      Ingest JSONL trace events from stdin.
  scryrs record --file <PATH>
      Ingest JSONL trace events from a file.
  scryrs init --agent <NAME>
      Install the scryrs trace hook for a supported agent harness.
      Supported harnesses: claude-code, pi

RECORD OUTPUT
  A single-line JSON summary on stdout.
  Rejection diagnostics are written as JSON objects to stderr,
  one per rejected non-empty line.

HOTSPOTS OUTPUT
  A single-line JSON placeholder on stdout.

EXAMPLES
  scryrs hotspots /path/to/repo
  scryrs hotspots .
  scryrs record --stdin < events.jsonl
  scryrs record --file session.jsonl
  scryrs init --agent claude-code
  scryrs init --agent pi

OPTIONS
  -h, --help       Print this help message and exit
  -V, --version    Print version and exit
  -hj, --help-json Print machine-readable CLI surface description and exit

EXIT CODES
  0    Success (hotspots: JSON written; record: all events accepted; init: hook installed)
  1    Hotspots: I/O error writing output. Record: rejected events or I/O error. Init: I/O error.
  2    Usage error (invalid arguments); record: also fatal I/O error (unreadable file); init: unsupported harness, collision, or self-install refusal
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

### Run the placeholder command

```bash
$ cargo run -p scryrs-cli -- hotspots .
{"schemaVersion":"0.1.0","command":"hotspots","status":"placeholder"}
```

This emits a single-line JSON envelope. The `status: "placeholder"` field means no engine behavior is wired yet — the command always returns this same structure regardless of what path you give it. It confirms the CLI pipeline works and exit code 0 means success.

### Error paths

The CLI follows a three-code exit convention with command-specific semantics:

| Exit code | Meaning     |
|-----------|-------------|
| 0         | Success (hotspots: JSON written; record: all events accepted) |
| 1         | Hotspots: I/O error. Record: rejected events or I/O error |
| 2         | Usage error; record: also fatal I/O error |

**Missing required argument** — exit 2:

```bash
$ cargo run -p scryrs-cli -- hotspots
scryrs hotspots: missing required PATH argument
Usage: scryrs hotspots <PATH>
See `scryrs --help`
# exit code: 2
```

### Current limitations

- **Three commands:** `hotspots`, `record`, and `init` are the supported commands. Everything else (`trace`, `propose`, `graph`, `route`, `adapters`, `report`, `suggest-docs`) produces an "unknown command" error.
- **Placeholder output:** `hotspots` always returns `{"status":"placeholder"}` regardless of the path argument. No analysis engine is wired yet.
- **Record is ingestion-only:** `scryrs record` validates and persists trace events. It does not trigger hotspot analysis, graph building, or other downstream processing.
- **No engine behavior:** The CLI is a contract shell — argument parsing, help text, error messages, and output formatting are frozen, but the analysis internals are not implemented.
- **What's not listed:** No speculative future commands or features appear here. The quickstart documents exactly what exists today.

### Troubleshooting

| Symptom | Likely cause | Fix |
|---------|-------------|-----|
| `unknown command: 'X'` | You ran a command that doesn't exist yet | Run `cargo run -p scryrs-cli -- --help` to see available commands |
| `missing required PATH argument` | You ran `hotspots` without a path | Add a path: `cargo run -p scryrs-cli -- hotspots .` |
| `error[E0463]: can't find crate` | Missing Rust toolchain or wrong directory | Install Rust 1.85+ via rustup, ensure you're in the repo root |
| Build hangs or runs out of memory | First build compiles many dependencies | It's normal for a fresh `cargo build`. Subsequent builds use cached artifacts and are much faster. |

## Current status

v0 CLI contract. `scryrs record` ingests JSONL trace events via `--stdin` or `--file <PATH>`, validates against the shared `TraceEvent` schema, persists accepted events to `.scryrs/events.jsonl`, and returns deterministic summary counts and rejection diagnostics. `scryrs hotspots <PATH>` emits a versioned JSON placeholder. Engine behavior comes next.

## Local checks

```bash
scripts/check
scripts/test
scripts/security
scripts/precommit-run
```
