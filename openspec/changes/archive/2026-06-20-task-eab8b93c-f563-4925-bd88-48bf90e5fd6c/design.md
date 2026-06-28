## Context

The repository has the necessary foundations for trace capture — a shared `TraceEvent` schema in `scryrs-types`, a deterministic `scryrs record` endpoint with documented output/exit contract, and a project-docs framework under `.devagent/docs/docs/`. However, harness integrators currently have no single document that tells them what to capture, how to invoke `scryrs record`, what boundaries scryrs must not cross, and which integration path to choose.

The refinement room accepted decisions that: (1) the canonical doc goes at `.devagent/docs/docs/trace-hook-contract.md` and is added to `_nav.json`; (2) `scryrs.json` shape is documented but no checked-in file is created in this task; (3) three integration tiers are defined with explicit limitations; (4) reference hooks are marked as forthcoming; (5) the stale roadmap.mdx claim about `record` not existing must be fixed.

## Goals

- Publish one canonical hook-contract document as the single source of truth for harness integrators.
- Document the required event envelope linkage to `TraceEvent`, including required metadata fields and session start/end demarcation.
- Document `scryrs.json` as a hook-interface/record-invocation manifest only, explicitly not a tool catalog or MCP surface.
- Make fail-open and non-interference rules explicit, including that scryrs never rewrites tool stdout, stderr, exit status, or semantics and is never registered as an agent-callable business tool.
- Define explicit integration tiers (`full hook`, `plugin`, `rules-file fallback`) with supported harness mapping, installation references, and limitations.
- Link reference hook examples for Pi and Claude Code so later harness work has a concrete anchor.
- Fix stale roadmap.mdx claims that contradict current `scryrs record` existence.

## Non-Goals

- Implementing Pi, Claude Code, or any other harness hook in this task.
- Changing the `TraceEvent` wire schema or the `scryrs record` CLI contract.
- Creating a checked-in `scryrs.json` file at the repository root.
- Creating a `hooks/` directory or `hooks/README.md`.
- Adding hotspot analysis, graph, proposal, adapter, or runtime-routing behavior.
- Turning `scryrs.json` into a generalized tool registry, MCP descriptor, or business-tool surface.
- Embedding harness-specific business logic inside scryrs hook guidance.

## Decisions

### D1. Canonical doc at `.devagent/docs/docs/trace-hook-contract.md`

The single source of truth for harness integration lives as a project-doc page under `.devagent/docs/docs/`. It is added to `_nav.json` under the Technical section. No `hooks/README.md` is created in this task; if one is added later, it will be a thin pointer back to the canonical doc.

**Rationale:** Project docs under `.devagent/docs/docs/` are the established home for cross-cutting contract documentation (per project-docs conventions). The architect and lead-dev both recommended this location. The nav entry ensures discoverability.

### D2. scryrs.json documented as shape only, no checked-in file

The task documents the manifest purpose, shape, intended location (repository root), and an example minimal JSON skeleton. It explicitly states the manifest is not a tool catalog, MCP descriptor, or business-tool surface. No `scryrs.json` file is created in this task — file creation is deferred to the downstream `scryrs init --agent` installer work.

**Rationale:** The acceptance criteria require documenting the manifest shape, not creating the file. The lead-dev explicitly decided against file creation in this task. The manifest shape is marked as provisional v0.1 since the location and field schema may change before Phase 1 stabilization.

### D3. Three integration tiers with explicit limitations

The integration-tier matrix defines:
- **Full hook** — harness-native subprocess hook support (e.g., Pi `.pi/hooks/`, Claude Code hook system). Full automatic event coverage and session demarcation. Planned for Pi and Claude Code.
- **Plugin** — harness-specific plugin/extension API. Requires plugin auth/development per harness. Coverage depends on plugin capabilities.
- **Rules-file fallback** — manual event-rule authoring by the user. No automatic session demarcation or event coverage. Inherently partial — cannot intercept tool events without harness cooperation. Listed with explicit limitations.

**Rationale:** The task scenario requires explicit tiers. The exploration dossier and lead-dev both emphasized that rules-file fallback must not overclaim guarantees. Only Pi and Claude Code are named as concrete harnesses because they have confirmed extension-point evidence.

### D4. Reference hooks marked as forthcoming Phase 1 deliverables

The contract doc links to Pi and Claude Code reference hook work but explicitly marks them as forthcoming Phase 1 deliverables. It points to the roadmap Phase 1 section rather than linking to non-existent implementation files or tasks.

**Rationale:** No reference hooks exist in the repository. The acceptance criteria require either linking pending tasks or marking as forthcoming. Since no specific hook-implementation tasks exist yet, the doc marks them as forthcoming and references the roadmap Phase 1 section.

### D5. Roadmap.mdx corrected to reflect current product state

The "Current Starting Point" section of roadmap.mdx is updated to remove the stale claim that `record` does not exist and that the CLI only exposes placeholder `hotspots` behavior. Updated wording reflects that `scryrs record` exists for ingestion and `scryrs hotspots` remains a placeholder.

**Rationale:** The architect identified this as a blocker, the reviewer confirmed it, and the lead-dev agreed. Any hook contract doc must reference `record` as existing, so the roadmap must not contradict that claim.

### D6. TraceEvent schema referenced, not redefined

The contract doc references `crates/scryrs-types/src/lib.rs` as the canonical schema source and documents which event families and fields must be captured. It does not define the schema from scratch — it links to the executable Rust types and the archived `trace-event-schema` spec.

**Rationale:** The architect and lead-dev both emphasized referencing the existing schema rather than redefining it. The executable Rust types are the source of truth for the wire contract.

## Risks

- **Manifest shape stability:** Once documented, the `scryrs.json` shape becomes a de facto API contract. Mitigation: mark as provisional v0.1 with explicit note that location and field schema may change before Phase 1 stabilization.
- **Harness commitment creep:** Naming harnesses beyond Pi and Claude Code without evidence of their extension capabilities creates implicit commitments. Mitigation: only Pi and Claude Code are named; others are marked TBD.
- **Rules-file overcommitment:** Rules-file fallback could be misread as providing equivalent guarantees to executable hooks. Mitigation: explicit limitations documented — requires manual rule authoring, cannot guarantee automatic session demarcation or full event coverage.
- **Roadmap staleness beyond record:** README.md still claims "One command only" which is stale. Mitigation: this task focuses on roadmap.mdx; README.md staleness is noted as a separate follow-up concern but not addressed here to keep scope bounded.

## Traceability

- Task: `eab8b93c-f563-4925-bd88-48bf90e5fd6c`
- Dossier: `2026-06-20T15:20:35.216Z`
- Accepted decisions: `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, `1-swarm-reviewer-recommendation`
- Round outputs: `round:1:agent:swarm-architect`, `round:1:agent:swarm-lead-dev`, `round:1:agent:swarm-reviewer`
- Artifact base: `initial`
- Source files: `crates/scryrs-types/src/lib.rs`, `.devagent/docs/docs/cli-v0-contract.md`, `.devagent/docs/docs/roadmap.mdx`, `.devagent/docs/docs/_nav.json`, `openspec/specs/scryrs-record-endpoint/spec.md`, `openspec/changes/archive/2026-06-20-task-c1d32950-524f-4c82-8d1e-c98db9075f55/specs/trace-event-schema/spec.md`
