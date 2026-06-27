## Why

Hotspot coverage exists today as product vision, architecture notes, CLI contract text, OpenSpec requirements, and a README example, but there is no discoverable project-doc page that explains hotspots as a domain concept. Readers who encounter `scryrs hotspots <PATH>` must infer the product meaning from scattered technical sources — the CLI contract page describes schema fields and exit codes, the architecture page describes crate topology, and the vision page names hotspots as a strategic goal — but none of them tell a developer what a hotspot represents, what user problem it solves, or how to act on a `HotspotsReport`. This is the "missing why" gap: the current documentation makes implementation and schema details available, yet still forces readers to infer the product meaning of hotspots from technical sources.

## What Changes

1. **New domain-oriented doc page** — `.devagent/docs/docs/hotspots.md` explains hotspots in domain terms: what a hotspot represents (repeated context churn around a subject across multiple agent sessions), what user/developer pain it surfaces (undiagnosed repeated effort, missing durable knowledge, failure clusters), how the local batch workflow works (hook captures `TraceEvent` → `scryrs record` persists to SQLite → `scryrs hotspots` runs deterministic scoring → `HotspotsReport` to stdout and `.scryrs/hotspots.json`), how to interpret each core field (subject, subjectKind, score, counts, sessionCount, evidence.rowIds), and what decisions a developer can make from hotspot output (prioritize docs for high-score subjects, investigate failure-clustered subjects, trace evidence.rowIds back to specific agent actions).

2. **Nav entry** — `.devagent/docs/docs/_nav.json` gains a new entry under the "Technical" section (after Trace Hook Contract) linking to `/hotspots`.

3. **Cross-links from existing pages** — `vision.md`, `architecture.mdx`, `cli-v0-contract.md`, and `trace-hook-contract.md` gain minimal "Related Pages" links to the new `hotspots.md`.

4. **Live-server boundary labeling** — Any mention of the live hotspot server (`scryrs server`, live accumulators, Signal SSE streams) is explicitly labeled as a separate deployment mode, not part of the local batch workflow narrative.

## Impact

- **Affected specs:** None — this is a documentation-only change. No OpenSpec capabilities, production Rust code, CLI behavior, scoring formulas, or schema definitions are modified.
- **Affected code:** None — no Rust, TypeScript, Cargo, or test changes.
- **Affected docs:** `.devagent/docs/docs/hotspots.md` (new), `.devagent/docs/docs/_nav.json` (modified), `.devagent/docs/docs/vision.md` (modified — minimal cross-link), `.devagent/docs/docs/architecture.mdx` (modified — minimal cross-link), `.devagent/docs/docs/cli-v0-contract.md` (modified — minimal cross-link), `.devagent/docs/docs/trace-hook-contract.md` (modified — minimal cross-link).
- **Migration needed:** None.
- **Build impact:** `bun run build` in `.devagent/docs/` must succeed with no broken links.