# Proposals

scryrs proposals are review-only knowledge suggestions. They turn hotspot and graph evidence into deterministic inbox artifacts without mutating published docs, `.scryrs/graph.json`, `.scryrs/routes.json`, or any future runtime memory truth.

## What a Proposal Represents

A **proposal** is not accepted knowledge. It is candidate knowledge with explicit rationale and evidence. That review-first boundary is core product behavior: scryrs may suggest durable docs, skills, memory patches, or semantic groupings, but it does not silently merge them into source-of-truth artifacts.

## Proposal Document Contract

Every proposal is serialized as `ProposalDocument`.

| Field | Purpose |
| --- | --- |
| `schemaVersion` | Proposal contract version. Current value: `1.0.0` (`PROPOSAL_SCHEMA_VERSION`). |
| `id` | Deterministic SHA-256 content address derived from `targetType` plus canonical `proposedContent`. |
| `targetType` | Closed target kind such as `docs_note` or `semantic_graph_grouping`. |
| `title` | Short reviewer-facing summary. |
| `rationale` | Non-empty explanation of why proposal exists. |
| `proposedContent` | Target-type-specific content payload. |
| `evidence` | Non-empty `EvidenceLink` array reusing existing provenance vocabulary. |
| `createdAt` | Proposal timestamp. Deterministic generator uses hotspot report `generatedAt`. |

## Supported Target Types

The contract supports six target types:

| Target type | Content shape | Current deterministic generator |
| --- | --- | --- |
| `docs_note` | Non-empty markdown | Generated |
| `adr` | Non-empty markdown | Generated |
| `skill` | Non-empty markdown | Generated |
| `debugging_playbook` | Non-empty markdown | Not generated in v1 |
| `memory_patch` | Structured JSON object | Generated |
| `semantic_graph_grouping` | Structured grouping object | Generated only when graph input already carries qualifying hotspot-backed node families |

`semantic_graph_grouping` content must include:

- `sourceNodeIds`
- `targetGroupNodeId`
- `targetGroupLabel`

## Deterministic Inbox Layout

Proposal files live under `.scryrs/proposals/`.

```text
.scryrs/
  proposals/
    <proposal-id>.json
```

Identity rules:

- same `targetType` + same canonical `proposedContent` => same `id`
- same `id` => same inbox filename stem
- rerunning deterministic generation overwrites same file path rather than inventing new IDs

This makes proposal artifacts cacheable, diffable, and review-friendly.

## Current Deterministic Generator

`scryrs propose <PATH>` is current deterministic proposal command.

Input requirements:

- `.scryrs/hotspots.json` must exist
- `.scryrs/graph.json` must exist

Behavior:

- loads hotspot artifact to reuse `generatedAt`
- loads graph artifact for evidence-backed proposal rules
- generates `ProposalDocument` values through `crates/scryrs-curator`
- validates every proposal before writing
- writes only under `.scryrs/proposals/`
- prints proposal count to stdout

### Current heuristic rules

| Rule | Trigger | Output |
| --- | --- | --- |
| Docs note | Every hotspot entry | `docs_note` proposal |
| Skill | Hotspot has at least one failure outcome | `skill` proposal |
| Memory patch | `score >= 4` and failure ratio `>= 0.5` | `memory_patch` proposal |
| ADR | Same subject appears across `>= 2` subject kinds with aggregate score `>= 10` | `adr` proposal |
| Semantic grouping | Cross-kind graph node family shares subject stem and already carries qualifying hotspot-backed evidence | `semantic_graph_grouping` proposal |

`debugging_playbook` is intentionally excluded from deterministic generation in v1.

### Semantic grouping boundary

Low-level subject identities remain authoritative:

- `file:auth`
- `search:auth`
- `symbol:auth`

A semantic grouping proposal such as `domain_term:auth` is review artifact only. It cites exact source node IDs and evidence, but it does not rewrite graph truth by existing.

## Review-First Boundary

Proposal generation is fenced off from authoritative outputs.

What `scryrs propose` **does** mutate:

- `.scryrs/proposals/*.json`

What it **must not** mutate:

- docs source under `.devagent/docs/`
- `.scryrs/graph.json`
- `.scryrs/routes.json`
- published Markdown or Rspress outputs
- any acceptance/rejection ledger

This boundary is enforced in tests in `crates/scryrs-cli/src/propose.rs`.

## Optional Model-Assisted Curation

`crates/scryrs-curator-llm` is shipped as bounded, library-only assist layer. It does not add CLI flags or default-feature behavior.

Foundation 01 boundaries:

- lives in dedicated `scryrs-curator-llm` crate
- depends on `scryrs-llm`, not on deterministic policy path
- accepts bounded `EvidencePack` input only
- assigns stable input-local evidence citation IDs
- constructs explicit, tool-free `ModelRequest` values (`allow_tools = false`)
- supports two library APIs: proposal drafting and semantic grouping suggestion
- rejects malformed output, unknown citations, unknown source node IDs, and shape mismatches by failing whole run

What it does **not** do:

- no `scryrs propose --llm`
- no hosted-provider or credential workflow in CLI docs
- no automatic graph mutation
- no automatic route changes
- no acceptance lifecycle or review UI

Model output is proposal input only.

## Current Limitations

- Proposal generation is deterministic and local-file based. No dashboard review flow exists yet.
- Proposal inbox artifacts are not consumed automatically by graph build, route generation, or adapters.
- Current graph build is structural. Semantic grouping proposals depend on explicit qualifying graph evidence rather than hidden inference.
- Publishing approved proposals into Markdown or Rspress remains adapter-phase work.

## Related Pages

- [Graph](./graph.md) — graph evidence and identity boundaries that proposals must respect
- [Route Manifests](./route-manifests.md) — route artifacts proposals must not silently mutate
- [Architecture](./architecture.mdx) — crate boundaries for `scryrs-curator`, `scryrs-curator-llm`, and adapters
- [Product Roadmap](./roadmap.mdx) — Phase 6 review-first proposal engine and Phase 9 optional LLM layer
