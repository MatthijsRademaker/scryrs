## 1. Create canonical hook-contract project-doc page

- [x] 1.1 Create `.devagent/docs/docs/trace-hook-contract.md` with all required sections: purpose/boundaries, non-interference + fail-open rules, TraceEvent schema reference and event-family mapping, session demarcation rules (SessionStart/SessionEnd), `scryrs record` invocation contract (`--stdin` and `--file <PATH>` only, referencing `cli-v0-contract.md`), `scryrs.json` manifest shape with "not a tool catalog" disclaimer, integration-tier matrix (full hook / plugin / rules-file fallback) with limitations, install/setup references as manual steps pending `scryrs init`, and reference links to Pi and Claude Code marked as forthcoming Phase 1 deliverables.
- [x] 1.2 Verify the page references `crates/scryrs-types/src/lib.rs` as the canonical schema source and does not redefine envelope fields, payload families, or session lifecycle types.
- [x] 1.3 Verify the page documents only `scryrs record --stdin` and `scryrs record --file <PATH>` as ingestion modes, with no alternate ingestion paths invented.
- [x] 1.4 Verify the non-interference rule is stated unambiguously: scryrs never rewrites tool stdout/stderr/exit status/semantics; scryrs does not proxy business-tool execution; hooks contain no business logic beyond formatting plus subprocess delegation; scryrs is never registered as an agent-callable business tool or MCP/tool catalog surface.

## 2. Register the new page in docs navigation

- [x] 2.1 Add `trace-hook-contract.md` entry to `.devagent/docs/docs/_nav.json` under the Technical section.
- [x] 2.2 Verify the nav entry is discoverable and consistent with existing entries.

## 3. Fix roadmap.mdx stale claims

- [x] 3.1 Update `.devagent/docs/docs/roadmap.mdx` "Current Starting Point" section to remove the claim that `record` does not exist.
- [x] 3.2 Update the claim that the CLI only exposes placeholder `hotspots` behavior to reflect that `record` exists for JSONL trace event ingestion.
- [x] 3.3 Verify the updated roadmap does not contradict the `cli-v0-contract.md` or the new `trace-hook-contract.md`.

## 4. Create trace-hook-contract spec

- [x] 4.1 Create `openspec/changes/task-eab8b93c-f563-4925-bd88-48bf90e5fd6c/specs/trace-hook-contract/spec.md` with requirements covering: hook-contract documentation, non-interference rules, trace event mapping and session demarcation, record invocation contract, scryrs.json manifest shape, integration tiers and limitations, and reference hook links.
- [x] 4.2 Verify the spec requirements align with the task acceptance criteria and accepted refinement decisions.

## 5. Validation

- [x] 5.1 Run `openspec validate --strict` on the new change to confirm spec format compliance.
- [x] 5.2 Verify cross-references: hook-contract doc does not contradict cli-v0-contract.md, roadmap.mdx reflects current state, nav entry resolves correctly.