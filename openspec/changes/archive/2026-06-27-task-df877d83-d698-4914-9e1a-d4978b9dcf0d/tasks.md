## 1. Create the hotspot domain documentation page

- [x] 1.1 Write `.devagent/docs/docs/hotspots.md` with the following structure:
  - Opening problem statement: what user/developer pain hotspots surface (repeated context churn, missing durable knowledge, undiagnosed failure clusters)
  - Domain definition: what a hotspot represents — repeated context churn around a subject (file, search query, symbol, command, or document) across multiple agent sessions, indicating knowledge that should be documented or investigated; score is a proxy for context-churn effort, not semantic importance
  - Workflow explanation: agent hook captures `TraceEvent` → `scryrs record` persists to `.scryrs/scryrs.db` → `scryrs hotspots <PATH>` runs deterministic scoring → `HotspotsReport` emitted to stdout and `.scryrs/hotspots.json`
  - Field interpretation guide: what each core field tells the developer — `subject` (identity), `subjectKind` (category: file/search/symbol/command/document), `score` (weighted frequency + failure penalty), `counts.eventType` (which agent activities dominate), `counts.outcome` (failure density), `sessionCount` (breadth of impact across independent sessions), `evidence.rowIds` (traceability back to specific events)
  - Decision-use guidance: what to do with the report — high-scored files → needs architecture docs; repeated failed lookups → knowledge gap, needs documentation; high session breadth → cross-cutting concern deserving a design decision record; high failure density → fragile area needing investigation
  - What hotspots do NOT tell you: score is not code quality, not business value, not a measure of correctness
  - Related: Live Hotspot Server — brief callout clearly labeled as a separate deployment mode (`scryrs server`, live accumulators, Signal SSE streams), referencing `cli-v0-contract.md` and `roadmap.mdx` for full details
  - Related Pages section linking to vision.md, architecture.mdx, cli-v0-contract.md, trace-hook-contract.md, and the README hotspot section
- [x] 1.2 Ensure all score-related claims are verified against the weight constants in `crates/scryrs-core/src/scoring.rs` (`WEIGHT_FILE_OPENED=1`, `WEIGHT_SEARCH_RUN=2`, `WEIGHT_SYMBOL_INSPECTED=2`, `WEIGHT_COMMAND_EXECUTED=1`, `WEIGHT_DOC_RETRIEVED=2`, `WEIGHT_EDIT_MADE=3`, `WEIGHT_FAILED_LOOKUP=4`, `FAILURE_BONUS=2`)
- [x] 1.3 Ensure all field descriptions match the HotspotEntry schema in `openspec/specs/hotspot-report/spec.md` and `cli-v0-contract.md`
- [x] 1.4 Ensure the live-server callout is explicitly labeled as adjacent and does not describe live accumulators, signals, or SSE as part of the local batch workflow

## 2. Add navigation entry

- [x] 2.1 Add `{ "text": "Hotspots", "link": "/hotspots" }` to `.devagent/docs/docs/_nav.json` under the "Technical" section, after the Trace Hook Contract entry

## 3. Add cross-links from existing pages

- [x] 3.1 Add `- [Hotspots](./hotspots.md) — domain concept, scoring, and interpretation guide for scryrs hotspot reports` to the Related Pages section of `.devagent/docs/docs/vision.md`
- [x] 3.2 Add `- [Hotspots](./hotspots.md) — domain-oriented explanation of what hotspots are, how they are scored, and how to interpret a report` to the Related Pages section of `.devagent/docs/docs/architecture.mdx`
- [x] 3.3 Add `- [Hotspots](./hotspots.md) — domain concept and interpretation guide (this page covers the CLI output contract and exit codes)` to the Related Pages section of `.devagent/docs/docs/cli-v0-contract.md`
- [x] 3.4 Add `- [Hotspots](./hotspots.md) — domain concept and interpretation guide (this page covers the hook contract and event schema)` to the Related Pages section of `.devagent/docs/docs/trace-hook-contract.md`

## 4. Verify and rebuild

- [x] 4.1 Verify every claim in the new page against `crates/scryrs-core/src/scoring.rs`, `openspec/specs/hotspot-report/spec.md`, and `.devagent/docs/docs/cli-v0-contract.md`
- [x] 4.2 Run `bun run build` in `.devagent/docs/` and confirm no dead links or build errors
- [x] 4.3 Confirm the new page appears in the docs navigation
