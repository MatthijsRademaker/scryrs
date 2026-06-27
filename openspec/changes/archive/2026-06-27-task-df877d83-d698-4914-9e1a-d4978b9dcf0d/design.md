## Context

scryrs produces a `HotspotsReport` from persisted trace events. The implementation is production-grade: `crates/scryrs-core/src/scoring.rs` defines weight constants (`WEIGHT_FILE_OPENED=1`, `WEIGHT_EDIT_MADE=3`, `WEIGHT_FAILED_LOOKUP=4`, `FAILURE_BONUS=2`, etc.) and a six-key tie-break chain; `openspec/specs/hotspot-report/spec.md` encodes the deterministic contract with 16 requirements; `crates/scryrs-cli/src/hotspots.rs` materializes the report; `README.md` shows an example. The documentation information architecture, however, has no dedicated hotspot page — `_nav.json` lists Vision & Goals, Product Roadmap, Architecture, CLI v0 Contract, and Trace Hook Contract, but no hotspot entry. The existing pages describe hotspots from implementation, schema, or strategic perspectives, but none teaches a developer what a hotspot is in domain terms or how to interpret a report.

This creates a discoverability and comprehension gap. A developer who wants to understand what `scryrs hotspots` produces and why must read `cli-v0-contract.md` (schema + exit codes), `architecture.mdx` (crate topology), `vision.md` (strategic promise), and the OpenSpec spec (formal contract) — none of which explains the domain concept first.

The docs-writer skill explicitly identifies "missing why" as the most common documentation deficiency and says docs should explain the problem a component solves before implementation details. The dossier identifies this as the core problem to fix.

## Goals / Non-Goals

### Goals
1. Create a dedicated `.devagent/docs/docs/hotspots.md` page that explains hotspots in domain terms before any schema or architecture detail.
2. Make the page discoverable via docs navigation (`_nav.json`) and cross-links from existing related pages.
3. Teach readers how to interpret the core `HotspotEntry` fields: subject, subjectKind, score, counts (eventType + outcome), sessionCount, and evidence.rowIds.
4. Provide decision-use guidance: what actions a developer should take based on score, session breadth, and failure density.
5. Clearly label live server hotspot behavior as an adjacent deployment mode, not part of the local batch workflow narrative.
6. Verify all claims against the current source-of-truth files (`scoring.rs` weight constants, `hotspot-report/spec.md` contract, `cli-v0-contract.md` schema).

### Non-Goals
1. Changing hotspot scoring, ranking, schema, CLI behavior, dashboard functionality, or server functionality.
2. Rewriting all architecture and roadmap pages.
3. Inventing or describing graph, proposal, or runtime features as if they are part of current hotspot behavior.
4. Modifying `README.md` — the README is out of scope; the new doc page links to the README for quickstart context rather than duplicating it.
5. Modifying OpenSpec specs or canonical `.c4` architecture diagrams.

## Decisions

| Decision | Rationale | Source |
|----------|-----------|--------|
| Dedicated `.devagent/docs/docs/hotspots.md` page, not an addition to `cli-v0-contract.md` | Preserves single-responsibility: CLI contract stays a technical reference; the new page is a domain explainer. Discoverable from navigation. | swarm-architect (round:1), swarm-lead-dev (round:1), swarm-reviewer (round:1) |
| Nav entry under "Technical" section, after Trace Hook Contract | The Technical section already contains CLI Contract and Trace Hook Contract — a hotspot page belongs at the same structural level. The reviewer suggested "Documentation" section between Architecture and CLI/Tech, but the architect's placement (alongside contract pages, not between strategic docs and architecture) is functionally clearer because hotspots are a concrete feature, not a meta-doc category. | swarm-architect (round:1); swarm-reviewer's alternate placement considered and set aside in favor of architect's reasoning |
| Cross-links from vision.md, architecture.mdx, cli-v0-contract.md, and trace-hook-contract.md | The reviewer enumerated these four pages. The lead-dev suggested vision.md and architecture.mdx. All four are appropriate because each currently lacks a hotspot link. roadmap.mdx is excluded — it links heavily to vision and architecture already, and a fourth cross-link there adds less value. | swarm-reviewer (round:1), swarm-lead-dev (round:1) |
| README.md is out of scope for this task | The task prompt says "Update/Add hotspot docs" in the context of the internal docs site. The README already has a hotspot quickstart section. The lead-dev explicitly recommended linking to the README for example output rather than duplicating it. | swarm-lead-dev (round:1), dossier non-goals |
| Live server labeled as adjacent/future, not part of core narrative | The acceptance criteria require this. The live server IS implemented, but it is a separate deployment mode. The new page focuses on the local batch workflow (`record` → SQLite → `hotspots` report). A brief "Related: Live Hotspot Server" callout at the end references `cli-v0-contract.md` and `roadmap.mdx`. | swarm-lead-dev (round:1), swarm-reviewer (round:1), acceptance criterion #4 |
| Score is proxy for context-churn effort, not semantic importance | The lead-dev explicitly required this as the first domain concept taught. `scoring.rs` weights reflect agent effort cost, not code quality or business value. | swarm-lead-dev (round:1), scoring.rs weight table |
| Decision-use guidance tied to score components | The lead-dev required answering "what do I do with this?" — high-scored files → needs architecture docs; repeated failed lookups → knowledge gap; high session breadth → cross-cutting concern. | swarm-lead-dev (round:1) |
| All claims verified against scoring.rs and hotspot-report/spec.md | Acceptance criterion #5. The docs-writer skill Step 7. | swarm-reviewer (round:1), docs-writer SKILL.md |

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| New page could inadvertently describe live-server hotspot signals as current local behavior | Medium | Medium — would violate AC #4 and confuse readers about which workflow is primary | Page sections: first 80% covers only batch workflow; a single "Related: Live Hotspot Server" callout at the end references roadmap and CLI contract. |
| Domain-oriented prose could drift from scoring.rs weight constants or the OpenSpec contract | Low | High — would create documentation that contradicts executable truth | Verify every score-related claim against `crates/scryrs-core/src/scoring.rs` weight table and `openspec/specs/hotspot-report/spec.md` before considering the page complete. |
| Cross-surface drift with README.md hotspot explanation | Low | Medium — readers seeing both surfaces could be confused by inconsistent terminology | New page links to README for quickstart/example context rather than reprinting the full `HotspotsReport` envelope. Uses same field names as the README. |
| Over-explaining the schema in the domain page | Medium | Low — dilutes the domain focus with technical reference content that belongs in cli-v0-contract.md | Domain page explains what score means and what session count means; references cli-v0-contract.md for the exact weight table. Duplicating the weight table is explicitly avoided. |
| New page fails Rspress build | Low | Medium — broken build blocks docs publishing | Run `bun run build` in `.devagent/docs/` as final verification step, matching docs-writer skill Step 8. |

## Traceability

| Artifact | Source |
|----------|--------|
| Task prompt | task:df877d83-d698-4914-9e1a-d4978b9dcf0d, dossier:2026-06-27T08:25:20.647Z |
| Accepted decision: dedicated page | decision:1-swarm-architect-recommendation, decision:1-swarm-lead-dev-recommendation, decision:1-swarm-reviewer-recommendation |
| Accepted decision: nav placement | round:1:agent:swarm-architect, round:1:agent:swarm-reviewer |
| Accepted decision: cross-links | round:1:agent:swarm-reviewer (enumerated pages), round:1:agent:swarm-lead-dev (vision.md, architecture.mdx) |
| Source truth: scoring weights | `crates/scryrs-core/src/scoring.rs` lines 17-62 |
| Source truth: hotspot contract | `openspec/specs/hotspot-report/spec.md` |
| Source truth: CLI contract | `.devagent/docs/docs/cli-v0-contract.md` |
| Source truth: verification spec | `openspec/specs/hotspot-verification/spec.md` |
| Doc conventions | `.pi/skills/docs-writer/SKILL.md` |
| Current artifact snapshot | artifact:initial |