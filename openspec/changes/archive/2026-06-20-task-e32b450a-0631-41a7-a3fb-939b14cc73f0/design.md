# Design: `scryrs.json` Hook Manifest Artifact

## Context

The project needs a real, checked-in `scryrs.json` manifest so that hooks, docs, and future installer code share one deterministic source of truth. Currently:
- The installer (`crates/scryrs-cli/src/init.rs`) hardcodes harness metadata via a typed `HARNESS_REGISTRY` with `include_str!()` paths.
- Hook implementations (`hooks/claude-code/scryrs-hook.mjs`, `hooks/pi/index.ts`) each in-line the trace schema version and tool whitelists.
- Project docs (`trace-hook-contract.md`) define a provisional skeleton that is harness-agnostic and doesn't distinguish per-harness lifecycle capabilities.
- The init-installer spec explicitly forbids reading or depending on `scryrs.json` (Decision D8).

The design challenge is to create a manifest structure that truthfully represents the two existing hooks with their *different* capabilities while keeping the manifest as repository metadata (not installer input).

## Goals / Non-Goals

### Goals
- Provide a single, parseable `scryrs.json` at repo root with a narrow, versioned schema.
- Enumerate supported harnesses (`claude-code`, `pi`) with canonical reference source paths, intercepted tool names, and captured event families.
- Encode the trace schema version (`0.1.0` matching `scryrs_types::SCHEMA_VERSION`) and the `scryrs record` invocation contract.
- Accurately represent per-harness lifecycle capabilities: Claude Code (none), Pi (SessionStart only, SessionEnd deferred).
- Include an explicit `scope` section declaring this is a hook-interface manifest — not a tool catalog, MCP descriptor, or callable surface.

### Non-Goals
- Do NOT modify `scryrs init` to read `scryrs.json`. The installer's hardcoded registry remains the sole source of truth for install targets.
- Do NOT change the `TraceEvent` wire schema, `scryrs record` CLI behavior, or hook business logic.
- Do NOT add consumer-side config paths (`.claude/`, `.pi/extensions/`) to the manifest.
- Do NOT model scryrs as a generic tool registry, MCP descriptor, or agent-callable surface.

## Decisions

### D1: `supported_harnesses` array structure
**Decision:** Replace the provisional `hooks.tool_events`/`hooks.session_lifecycle` split with a flat `supported_harnesses` array containing one entry per harness.

**Rationale:** The provisional structure was designed before hooks existed and assumes all harnesses share the same lifecycle capabilities. In reality, Claude Code emits zero lifecycle events and Pi emits only `SessionStart`. A per-harness structure accurately represents that each harness has different lifecycle support. It also cleanly separates record-invocation metadata (top-level) from harness-specific hook contracts (per-entry).

**Sources:** Architect recommendation, Lead-dev recommendation, Reviewer blocker #2, hooks/claude-code/README.md (no lifecycle), hooks/pi/README.md (SessionEnd deferred).

### D2: Per-harness entries include both intercepted tool names and captured event families
**Decision:** Each harness entry includes `interceptedTools` (harness-specific tool names like `"read"`, `"bash"`, `"grep"`) and `capturedEventFamilies` (canonical `TraceEventType` families like `"FileOpened"`, `"CommandExecuted"`).

**Rationale:** Harness integrators need the tool-name-to-event-family mapping to write hooks. Schema consumers (installer code, docs) need canonical event families. Both audiences are served by including both fields. Tool names are framed as observational metadata (what the hook observes), structurally distinct from any MCP or tool-registration schema.

**Sources:** Architect risk mitigation, Reviewer blocker #2, Reviewer question Q3, hooks/claude-code/README.md tool mapping table, hooks/pi/README.md tracked tools table.

### D3: Scope/anti-pattern declaration in the manifest itself
**Decision:** The manifest includes an explicit `scope` field (value: `"hook-interface-manifest"`) and a `_anti_patterns` section declaring what the manifest is NOT.

**Rationale:** The task acceptance criteria require no MCP methods, no tool registrations, and no tool-output-rewriting behavior. Including this boundary in the manifest itself is anti-regression insurance against future misuse. It makes the constraint self-documenting and machine-checkable.

**Sources:** Architect suggested requirement, Lead-dev risk mitigation, trace-hook-contract.md anti-pattern section, task AC-4.

### D4: Manifest is repository metadata, not installer input
**Decision:** The manifest does NOT include consumer install targets and explicitly states it is not consumed by the current `scryrs init` installer.

**Rationale:** The accepted init-installer spec (Decision D8) says the installer SHALL NOT create, read, or depend on `scryrs.json`. Changing installer behavior without updating that spec would silently contradict an accepted architectural decision. The hardcoded `HARNESS_REGISTRY` in `init.rs` remains the sole source of truth for install targets.

**Sources:** Architect blocker #2, Lead-dev risk #1, Reviewer blocker #1, init-installer spec lines 234-252.

### D5: `manifest_version` set to `"0.1.0"`
**Decision:** The manifest's own schema version is `"0.1.0"`, matching the provisional skeleton's version. This is independent of `trace_schema_version`.

**Rationale:** The provisional skeleton used `"0.1.0"` as a placeholder. Since the real schema is being introduced for the first time, `"0.1.0"` is appropriate. The manifest version is distinct from the trace schema version and will evolve independently.

**Sources:** Lead-dev question unresolved by refinement, provisional skeleton in trace-hook-contract.md.

## Risks

- **Schema drift (trace_schema_version):** The manifest hardcodes `trace_schema_version: "0.1.0"`. If `crates/scryrs-types/src/lib.rs` changes `SCHEMA_VERSION`, the manifest must be updated manually. Mitigation: The manifest documents where the canonical version is defined, and a code-review checklist item ensures it is updated when SCHEMA_VERSION changes.
- **Provisional skeleton divergence:** `trace-hook-contract.md` still shows the old harness-agnostic skeleton. The real file uses a different structure. Docs Task 05 must reconcile this. The manifest includes a `manifest_version` field that can be used for doc-to-file cross-referencing.
- **Tool-name misinterpretation:** Including harness-specific tool names risks confusion with a tool registry. Mitigation: Tool names are clearly labeled as `interceptedTools` (observational) under each harness entry, with the explicit `_anti_patterns` section declaring this is not a tool catalog.

## Traceability

- **Task:** e32b450a-0631-41a7-a3fb-939b14cc73f0 (Trace Foundation 08)
- **Dossier:** 2026-06-20T19:06:53.139Z
- **Decisions:** 1-swarm-architect-recommendation, 1-swarm-lead-dev-recommendation, 1-swarm-reviewer-recommendation
- **Round 1 agents:** swarm-architect, swarm-lead-dev, swarm-reviewer
- **Canonical sources:** crates/scryrs-types/src/lib.rs (SCHEMA_VERSION), crates/scryrs-cli/src/init.rs (HARNESS_REGISTRY), hooks/claude-code/scryrs-hook.mjs + README.md, hooks/pi/index.ts + README.md, .devagent/docs/docs/trace-hook-contract.md, .devagent/docs/docs/cli-v0-contract.md, openspec/specs/init-installer/spec.md