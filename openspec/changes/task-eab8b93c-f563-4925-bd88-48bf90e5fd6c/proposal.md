## Why

scryrs now has a shared `TraceEvent` schema in `crates/scryrs-types`, a deterministic `scryrs record` ingestion endpoint with a documented CLI contract, and a project-docs framework under `.devagent/docs/docs/`. What it still lacks is one canonical harness-integration contract that tells integrators what to capture, what metadata/session boundaries to preserve, how to invoke `scryrs record`, what scryrs must never do, and which integration path fits their harness.

Without this contract, harness authors would improvise behavior — potentially violating scryrs' trace-collection-only boundary, inventing alternate ingestion paths, or misusing the future `scryrs.json` manifest as a tool registry. The roadmap already treats non-interfering proxy capture as Phase 1 truth, and the current docs contain contradictory claims about whether `record` exists.

This task publishes that contract as a single project-doc page, documents the `scryrs.json` manifest shape, corrects stale roadmap claims, and defines integration tiers so every harness has a clear on-ramp.

## What Changes

- **New project-doc page `.devagent/docs/docs/trace-hook-contract.md`** — canonical single source of truth for harness integration, added to `_nav.json` under Technical.
- **Non-interference + fail-open rules** — documented unambiguously: scryrs never rewrites tool stdout/stderr/exit status/semantics, never proxies tool execution, never registers as an agent-callable business tool or MCP/tool catalog surface.
- **TraceEvent mapping** — references the existing schema in `crates/scryrs-types/src/lib.rs` and lists required event families, envelope fields, and session demarcation rules aligned to `SessionStart`/`SessionEnd` first-class events.
- **`scryrs record` invocation contract** — documents `--stdin` and `--file <PATH>` as the only supported ingestion modes, referencing `cli-v0-contract.md` for the deterministic output/exit contract.
- **`scryrs.json` manifest shape** — documented as hook-interface + record-invocation manifest only, with example minimal shape and explicit statement that it is not a tool catalog, MCP descriptor, or business-tool surface. No checked-in file is created in this task.
- **Integration-tier matrix** — three tiers (full hook, plugin, rules-file fallback) with supported/planned harness coverage (Pi, Claude Code) and explicit limitations per tier, especially for rules-file fallback.
- **Reference hook links** — Pi and Claude Code marked as forthcoming Phase 1 deliverables, linked to the roadmap Phase 1 section.
- **Roadmap staleness fix** — `.devagent/docs/docs/roadmap.mdx` updated to remove the stale claim that `record` does not exist and that the CLI only exposes placeholder `hotspots` behavior.

## Impact

- **Affected specs:** New `trace-hook-contract` capability spec; no changes to existing `scryrs-record-endpoint` or `trace-event-schema` specs.
- **Affected code:** No Rust crate or CLI behavior changes. Only project docs and navigation.
- **Affected docs:** New `.devagent/docs/docs/trace-hook-contract.md`; modified `.devagent/docs/docs/_nav.json`; modified `.devagent/docs/docs/roadmap.mdx`.
- **Downstream consumers:** Harness integrators (Pi hooks, Claude Code hooks, any future harness) read the contract doc as single source of truth.
- **No breakage:** This is purely additive documentation. No existing API, CLI, or schema contract changes.