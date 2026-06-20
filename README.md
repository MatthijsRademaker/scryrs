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

The CLI ships a single v0 placeholder command:

```bash
cargo run -p scryrs-cli -- hotspots /path/to/repo
```

Default features include the standalone suite, Markdown adapter, runtime, and deterministic guardrail support. `full` adds the optional LLM boundary and Rspress adapter.

## Quickstart

Get from a freshly cloned repo to your first command in under two minutes.

### Prerequisites

- **Rust 1.85+** (install via [rustup](https://rustup.rs)) — or —
- **Docker** (the workspace ships a dev container with all tooling pre-installed)

### Build from source

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

USAGE
scryrs hotspots <PATH>

ARGUMENTS
<PATH>    Path to the repository root directory (required)

OUTPUT
A single-line JSON object with the following envelope:
{
"schemaVersion": "0.1.0",
"command": "hotspots",
"status": "placeholder"
}

EXAMPLES
scryrs hotspots /path/to/repo
scryrs hotspots .

OPTIONS
-h, --help       Print this help message and exit
-V, --version    Print version and exit

EXIT CODES
0    Success (output written to stdout)
1    I/O error (output could not be written)
2    Usage error (invalid arguments)
```

**`--version`** prints the binary version:

```bash
$ cargo run -p scryrs-cli -- --version
scryrs 0.1.0
```

**`--help-json`** prints a machine-readable surface description:

```bash
$ cargo run -p scryrs-cli -- --help-json
{"surfaceVersion":"0.1.0","binary":"scryrs","commands":[...],...}
```

The JSON document describes every command, argument, flag, output field, and exit code — suitable for parsing by tooling or agents.

### Run the placeholder command

```bash
$ cargo run -p scryrs-cli -- hotspots .
{"schemaVersion":"0.1.0","command":"hotspots","status":"placeholder"}
```

This emits a single-line JSON envelope. The `status: "placeholder"` field means no engine behavior is wired yet — the command always returns this same structure regardless of what path you give it. It confirms the CLI pipeline works and exit code 0 means success.

### Error paths

The CLI follows a three-code exit convention:

| Exit code | Meaning     |
|-----------|-------------|
| 0         | Success     |
| 1         | I/O error   |
| 2         | Usage error |

**Missing required argument** — exit 2:

```bash
$ cargo run -p scryrs-cli -- hotspots
scryrs hotspots: missing required PATH argument
Usage: scryrs hotspots <PATH>
See `scryrs --help`
# exit code: 2
```

### Current limitations

- **One command only:** `hotspots` is the only command. Everything else (`trace`, `propose`, `graph`, `route`, `adapters`, `report`, `suggest-docs`) produces an "unknown command" error.
- **Placeholder output:** `hotspots` always returns `{"status":"placeholder"}` regardless of the path argument. No analysis engine is wired yet.
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

v0 CLI contract frozen. Single placeholder command `scryrs hotspots <PATH>` emits versioned JSON. Engine behavior comes next.

## Local checks

```bash
scripts/check
scripts/test
scripts/security
scripts/precommit-run
```
