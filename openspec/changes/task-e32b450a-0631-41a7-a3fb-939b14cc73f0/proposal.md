# Trace Foundation 08 — Add `scryrs.json` Hook Manifest Artifact

## Why

The repository documents a provisional `scryrs.json` shape in `trace-hook-contract.md`, but no real manifest file exists at the repo root. Hook metadata is currently split across multiple sources: the hardcoded installer registry in `crates/scryrs-cli/src/init.rs`, hook source files with inline constants (`hooks/claude-code/scryrs-hook.mjs`, `hooks/pi/index.ts`), per-harness READMEs, and project documentation. This fragmentation means there is no single deterministic source of truth for trace-capture behavior.

Creating the real `scryrs.json` artifact provides:
- One machine-readable contract listing supported harnesses, their reference source paths, intercepted tools, and captured event families.
- A single canonical declaration of the trace schema version and `scryrs record` invocation contract.
- An explicit boundary that scryrs is trace-collection only (not a tool catalog, MCP surface, or callable registry).

The provisional skeleton in the docs uses a harness-agnostic `hooks.tool_events`/`hooks.session_lifecycle` split designed before any hooks existed. It claims full `SessionStart`+`SessionEnd` lifecycle capture, but neither existing hook supports that: Claude Code is PreToolUse-only with zero lifecycle events, and Pi defers `SessionEnd`. The real manifest must accurately represent per-harness capabilities.

## What Changes

- **New file:** `scryrs.json` at repository root — a versioned manifest describing hook interfaces and the record invocation contract.
- **Schema design:** Flat `supported_harnesses` array (not the provisional nested `hooks.tool_events`/`hooks.session_lifecycle` structure), with per-harness entries containing reference source paths, intercepted tool names, captured event families, lifecycle capabilities, and explicit limitations.
- **Top-level fields:** `manifest_version`, `trace_schema_version`, `record` (invocation contract), `supported_harnesses` array, and an explicit `scope`/boundary declaration.
- **Per-harness accuracy:** Claude Code entry lists no lifecycle events and reflects PreToolUse-only unconditional-Success outcome. Pi entry lists `SessionStart` lifecycle support and documents the deferred `SessionEnd` limitation.
- **No installer integration:** The manifest is repository metadata only. It is NOT consumed by the current `scryrs init` installer, consistent with the accepted init-installer spec (Decision D8).
- **No MCP/tool-catalog fields:** The manifest contains zero MCP methods, agent-callable tool registrations, or tool-output-rewriting behavior.

## Impact

- **Affected specs:** Adds new spec `scryrs-manifest` defining the manifest schema and correctness requirements.
- **Affected code:** No Rust crate, CLI behavior, or hook source files are modified. This is an artifact-only change.
- **Affected docs:** The manifest existence must be reflected in `trace-hook-contract.md` (Task 05 follow-up). The provisional skeleton in that doc will be superseded by the real file.
- **No breaking changes:** No existing behavior, CLI surface, or wire format changes.