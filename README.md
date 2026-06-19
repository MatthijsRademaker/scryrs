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

## Current status

v0 CLI contract frozen. Single placeholder command `scryrs hotspots <PATH>` emits versioned JSON. Engine behavior comes next.

## Local checks

```bash
scripts/check
scripts/test
scripts/security
scripts/precommit-run
```
