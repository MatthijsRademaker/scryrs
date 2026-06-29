# Production Suite Plan

scryrs is no longer a trace-capture scaffold. The production target is a closed evidence loop: observe agent work, score repeated attention, share live signals, organize evidence into graph and route artifacts, review proposed knowledge, publish approved knowledge, and let future agents load better context with explainable reasons.

## Current State

| Area | State | Production gap |
| --- | --- | --- |
| Trace capture | `scryrs record`, `scryrs hook`, `scryrs init`, local SQLite, fail-open hooks, Pi and Claude Code adapters | Broader harness matrix and release-grade install diagnostics |
| Hotspots | Deterministic batch scoring and `.scryrs/hotspots.json` shipped | Long-running UX around trend/history still missing |
| Live hotspots | Central ingest server, idempotency, accumulators, query API, SSE signals shipped | Dashboard live mode and multi-agent end-to-end product workflow missing |
| Graph | `scryrs graph <PATH>` builds structural graph from hotspots plus docs navigation | Cross-domain edges and accepted evidence ingestion missing |
| Route manifests | `scryrs route <PATH>` emits `.scryrs/routes.json` from graph nodes | Runtime explanation and context loading decisions missing |
| Proposals | `ProposalDocument`, inbox layout, deterministic `scryrs propose <PATH>`, `scryrs proposals list|accept|reject`, accepted/rejected review artifacts, safety checks shipped | Dashboard review UX and broader accepted-evidence consumers still missing |
| Adapters | Generic Markdown publisher consumes reviewed `.scryrs/accepted/*.json` into deterministic plain Markdown files | Rspress and llms-specific publishing surfaces still missing |
| LLM assist | Bounded `scryrs-curator-llm` library shipped | Product integration must wait for acceptance lifecycle |

## Production Loop

```text
Observe
  hooks / record / server ingest
    ↓
Detect
  batch hotspots + live accumulators + signals
    ↓
Organize
  graph build + route manifests
    ↓
Review
  proposal inbox + accept/reject ledger
    ↓
Publish
  generic Markdown / then Rspress / llms surfaces
    ↓
Route
  explainable route hints for future agents
```

The loop is production-ready only when every arrow is explicit, deterministic where it affects truth, and verified end-to-end.

## Critical Boundaries

### Deterministic truth path

These paths must stay model-free:

- trace ingest
- hotspot scoring
- live accumulator updates
- graph materialization
- route-manifest materialization
- proposal validation
- accepted-evidence ingestion

### Review-first promotion path

Proposal files are not truth. Accepted evidence is truth candidate. Production suite needs a separate acceptance artifact before proposals can influence graph, route, docs, or memory.

Required boundary:

```text
.scryrs/proposals/*.json       review inbox only — immutable proposal documents
.scryrs/accepted/*.json        reviewed evidence — accepted decisions with reviewed content
.scryrs/rejected/*.json        explicit rejection records — rejected decisions
.scryrs/graph.json             deterministic output from trace + docs + accepted evidence
.scryrs/routes.json            deterministic projection from graph
```

Review decision artifacts are versioned `ProposalReviewDecision` documents (`REVIEW_DECISION_SCHEMA_VERSION = "1.0.0"`) that record explicit accepted or rejected outcomes with mandatory provenance. In this phase, generic Markdown publishing consumes accepted review decisions, while graph build, route generation, and memory mutation still do not. Accepted-evidence ingestion into graph remains deferred to a follow-up change.

### LLM interpretation path

Model output may draft, summarize, rank, or suggest. It must not decide policy, validate ingest, score hotspots, or mutate graph truth. LLM integration becomes product only after accepted-evidence workflow exists.

## Production Milestones

| Milestone | Outcome | Must ship |
| --- | --- | --- |
| P1. Acceptance ledger | Proposals can become reviewed evidence without silent mutation | Accepted/rejected artifact contract, CLI review commands, validation, no-write guarantees |
| P2. Accepted evidence graph | Reviewed groupings and docs notes influence graph deterministically | Graph consumes accepted evidence; route manifests update from graph; provenance preserved |
| P3. Live dashboard | Multi-agent hotspots become visible product, not just API | Dashboard live mode, server API client, signal timeline, reconnect behavior |
| P4. Runtime explain | Agents can ask what to read and why | Route hint schema, `scryrs route explain`, deterministic evidence-backed reasons |
| P5. Publishing adapters | Reviewed knowledge leaves `.scryrs/` through generic Markdown first | Markdown adapter consumes accepted review decisions, then Rspress / llms surfaces layer on later deterministic outputs |
| P6. LLM assist UX | Models improve proposal quality without owning truth | Opt-in draft/group commands or UI action, bounded EvidencePack, citation validation, no auto-accept |
| P7. Production hardening | Suite can ship reliably | Release packaging, `scryrs doctor/status`, CI matrix, E2E live workflow, security and privacy checks |

## Near-Term Task Order

1. Define accepted proposal / accepted evidence contract.
2. Implement accept/reject CLI workflow over `.scryrs/proposals/`.
3. Feed accepted semantic groupings and docs evidence into graph build.
4. Build live dashboard read-only mode against `scryrs server` APIs.
5. Implement deterministic `scryrs route explain` over route manifests.
6. Publish accepted reviewed Markdown artifacts from `.scryrs/accepted/` to generic Markdown output roots.
7. Publish that reviewed Markdown into Rspress and regenerated llms surfaces.
8. Add opt-in LLM drafting over proposal inbox.
9. Harden release, diagnostics, and multi-agent E2E verification.

## Board Alignment

Backlog work should be ordered by missing loop closure, not by crate names alone:

- **High priority:** acceptance ledger, accepted-evidence graph ingestion, live dashboard mode.
- **Medium priority:** route explain, Markdown/Rspress publishing, LLM-assisted proposal drafting.
- **Low priority:** hosted multi-tenant behavior, dashboard mutation UX, automatic optimizer/rewrite behavior.

## Related Pages

- [Product Roadmap](./roadmap.mdx) — phase order and scope guardrails
- [Architecture](./architecture.mdx) — crate boundaries and deterministic/model separation
- [Graph](./graph.md) — graph identity and semantic grouping boundary
- [Route Manifests](./route-manifests.md) — route projection contract
- [Proposals](./proposals.md) — review-first proposal and LLM-assist boundaries
- [Live Hotspots](./live-hotspots.md) — live ingest and signal APIs
