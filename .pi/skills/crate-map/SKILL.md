---
name: crate-map
description: "Orientation map of every crate in the scryrs workspace — what each does, who depends on whom, where the binary lives. Use when: starting any task touching crates/, deciding which crate owns a change, or a task mentions a scryrs-* crate you don't know. Do not use for: task-specific steps — use add-cli-command, add-adapter, graph-core-changes, or dashboard-ui once you know where you're working."
metadata:
  curated: true
  sources:
    - Cargo.toml
    - crates/*/Cargo.toml
    - xtask/Cargo.toml
  last-verified: 2026-07-04
  verified-commit: 8f8ca6cdcee59fb31b4a12f5ab8c48787f2c345c
---

# Crate Map

16 crates + `xtask` in one Cargo workspace (`Cargo.toml` members). One shipped binary: **`scryrs`**, built from `crates/scryrs-cli` (`default-members` is scryrs-cli).

## The leaf everything sits on

- **`scryrs-types`** — shared wire contracts and schema-version constants (`SCHEMA_VERSION`, `GRAPH_SCHEMA_VERSION`, …). Depends on no internal crate; every other crate depends on it. If you're changing a cross-crate type, it lives here — see the `graph-core-changes` skill.

## Domain crates (each depends only on scryrs-types unless noted)

- **`scryrs-core`** — trace ingestion, hotspot scoring, SQLite event store, query. Consumed by cli, dashboard, server.
- **`scryrs-graph`** — knowledge-graph container + deterministic `to_document` materialization. Does NOT build the graph; the build pipeline is in `crates/scryrs-cli/src/graph.rs`.
- **`scryrs-curator`** — deterministic proposal engine over hotspot/graph evidence.
- **`scryrs-llm`** — provider-neutral LLM boundary. **`scryrs-curator-llm`** — optional LLM-assisted curator layer (depends on scryrs-llm; consumed by no other crate).
- **`scryrs-policy`**, **`scryrs-sandbox`**, **`scryrs-telemetry`** — guardrails trio (policy decisions, tool sandboxing, privacy-safe telemetry).
- **`scryrs-runtime`** — agent-side routing/retrieval helpers.

## Adapters (two unrelated meanings of "adapter" — do not conflate)

- **`scryrs-adapter-harness`** — translates agent-harness (Claude Code / Pi) tool events into canonical `TraceEvent`s. Defines the `HarnessAdapter` trait + `adapter_for()` registry. NOT a test harness.
- **`scryrs-adapter-markdown`** / **`scryrs-adapter-rspress`** — publishing adapters (accepted knowledge → docs surfaces). Function-based, no trait; rspress layers on markdown. See the `add-adapter` skill.

## Servers & UI

- **`scryrs-server`** — HTTP trace-ingest server (`scryrs server`), depends on core + types.
- **`scryrs-dashboard`** — dashboard server; its `build.rs` compiles the Vue 3 frontend at `crates/scryrs-dashboard/frontend/` with Bun. See the `dashboard-ui` skill.

## Assembly

- **`scryrs-cli`** — the `scryrs` binary. Depends on adapter-harness + types always; everything else behind feature flags (`core`, `dashboard`, `graph`, `curator`, `markdown`, `runtime`, `guardrails`, `server` are default; `full` adds `llm`, `rspress`). Command wiring is hand-rolled in `crates/scryrs-cli/src/dispatch.rs` — see the `add-cli-command` skill.
- **`xtask`** — dev-tool scaffold (`bootstrap`, `ci-fast`), currently stubbed; not shipped.

## Verification

Never run cargo/bun on the host. `scripts/check`, `scripts/test`, `scripts/precommit-run` run everything in Docker — see the `testing-this-repo` skill.
