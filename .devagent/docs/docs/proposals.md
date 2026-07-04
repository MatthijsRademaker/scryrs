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

A semantic grouping proposal such as `domain_term:auth` is review artifact only while it remains in `.scryrs/proposals/`. It cites exact source node IDs and evidence, but it does not rewrite graph truth by existing. That proposal can become graph input only after review records an accepted `ProposalReviewDecision` under `.scryrs/accepted/`.

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

## Three-Zone Review Artifact Layout

Proposal inbox files and review decision artifacts live in separate paths so proposal documents remain review-only records and review outcomes become durable evidence.

```text
.scryrs/
  proposals/           review inbox only — immutable proposal documents
    <proposal-id>.json
  accepted/            durable reviewed evidence — accepted decisions
    <proposal-id>.json
  rejected/            explicit rejection records — rejected decisions
    <proposal-id>.json
```

| Zone | Path | Purpose | Mutated by which command? |
| --- | --- | --- | --- |
| Proposal inbox | `.scryrs/proposals/{proposalId}.json` | Review-only candidate knowledge | `scryrs propose` writes here |
| Accepted evidence | `.scryrs/accepted/{proposalId}.json` | Durable accepted review outcome with reviewed content | `scryrs proposals accept` |
| Rejected decisions | `.scryrs/rejected/{proposalId}.json` | Explicit rejection record | `scryrs proposals reject` |

Accepted and rejected review decisions are recorded as separate `ProposalReviewDecision` artifacts rather than by mutating the original proposal inbox files. The original `.scryrs/proposals/{proposalId}.json` remains a review-only record regardless of acceptance or rejection.

`ProposalReviewDecision` is a versioned contract (`REVIEW_DECISION_SCHEMA_VERSION = "1.0.0"`) that reuses the existing `EvidenceLink`, `ProposalTargetType`, `ProposedContent`, and `SemanticGraphGrouping` types. Accepted decisions carry `targetType` plus `acceptedContent`; rejected decisions carry no accepted-content payload.

Trust boundary:

- `.scryrs/proposals/` remains non-authoritative inbox state only
- `.scryrs/accepted/` is the only review artifact set that publishing consumes; `scryrs publish markdown` and `scryrs publish rspress` both read `.scryrs/accepted/*.json` and never publish directly from proposal inbox files
- `.scryrs/accepted/` can also become graph-build input when the accepted decision targets `semantic_graph_grouping`
- `.scryrs/rejected/` records explicit rejections but is ignored by graph, route generation, and both publish modes

## Review CLI

Proposal review is now exposed as a grouped plural command surface:

```text
scryrs proposals list <PATH> [--state pending|accepted|rejected|all]
scryrs proposals accept <PATH> <ID> --reviewer <NAME> --rationale <TEXT> --decided-at <RFC3339>
scryrs proposals reject <PATH> <ID> --reviewer <NAME> --rationale <TEXT> --decided-at <RFC3339>
```

The naming split is intentional:

- `scryrs propose` **generates** inbox proposals
- `scryrs proposals ...` **reviews** existing inbox proposals
- `scryrs publish ...` **publishes** accepted review decisions only, in a separate explicit step

### `proposals list`

`list` reads `.scryrs/proposals/`, `.scryrs/accepted/`, and `.scryrs/rejected/` and emits a deterministic JSON array sorted by `proposalId` ascending.

Each row contains:

- `proposalId`
- `title`
- `targetType`
- `createdAt`
- `state` (`pending`, `accepted`, or `rejected`)

The optional `--state` filter accepts only `pending`, `accepted`, `rejected`, or `all`. Invalid filters exit `2`. Listing validates both proposal inbox documents and any encountered review-decision artifacts before emitting output; malformed JSON, schema-version mismatch, semantic validation failure, conflicting accepted+rejected states, or tampered review artifacts all fail loudly with exit `2` and no partial stdout.

### `proposals accept` and `proposals reject`

Both terminal review commands require explicit provenance metadata:

- `--reviewer <NAME>`
- `--rationale <TEXT>`
- `--decided-at <RFC3339>`

There are no defaults, and `decidedAt` is never derived from wall-clock time. Before either command writes a decision, the source inbox file must exist at `.scryrs/proposals/{proposalId}.json`, deserialize as `ProposalDocument`, match its deterministic content-derived `proposalId`, and pass `ProposalDocument::validate()`.

Accepted decisions copy:

- `targetType` from the proposal
- `proposedContent` into `acceptedContent`
- proposal `evidence` into `sourceEvidence`

Rejected decisions copy only `sourceEvidence` and set `outcome = rejected`; they omit `targetType` and `acceptedContent`.

Accepting a proposal is still ledger-only. `scryrs proposals accept` writes `.scryrs/accepted/{proposalId}.json`, but it does not create generic Markdown output, it does not update `.devagent/docs/docs/accepted-knowledge/`, and it does not touch `.devagent/docs/docs/_nav.json`. Operators must run `scryrs publish markdown` or `scryrs publish rspress` separately to materialize accepted knowledge.

### Determinism and conflicts

The review CLI is deterministic and overwrite-averse:

- rerunning `accept` or `reject` with byte-identical output succeeds as a no-op
- rerunning the same outcome with different bytes fails with exit `2`
- attempting `accept` when a rejected artifact already exists fails with exit `2`
- attempting `reject` when an accepted artifact already exists fails with exit `2`
- simultaneous accepted and rejected artifacts for one proposal ID are a conflicting terminal state; `proposals list` fails with exit `2`

### Review boundary and exit codes

Review commands preserve the existing trust boundary:

- `.scryrs/proposals/{proposalId}.json` is never mutated by review
- `.devagent/docs/`, `.scryrs/graph.json`, and `.scryrs/routes.json` are never created, modified, or deleted by review commands

Exit-code contract:

- `0` — success
- `1` — serialization or filesystem write failure
- `2` — usage or input error (invalid filter, invalid proposal/review artifact, unknown proposal ID, conflicting terminal state)

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
- Proposal inbox artifacts are still not consumed automatically by graph build, route generation, or adapters.
- Accepted review decisions can affect graph build only through `.scryrs/accepted/`, and only accepted `semantic_graph_grouping` targets project into graph structure today.
- Route generation still consumes `.scryrs/graph.json` only; it never reads proposal or review-artifact directories directly.
- Publishing now consumes reviewed `.scryrs/accepted/*.json` artifacts only. `scryrs publish markdown` writes plain Markdown to a caller-chosen output root, and `scryrs publish rspress` writes Rspress `accepted-knowledge/` pages plus `_nav.json` updates as a separate explicit operator action.

## Related Pages

- [Graph](./graph.md) — graph evidence and identity boundaries that proposals must respect
- [Route Manifests](./route-manifests.md) — route artifacts proposals must not silently mutate
- [Architecture](./architecture.mdx) — crate boundaries for `scryrs-curator`, `scryrs-curator-llm`, and adapters
- [Product Roadmap](./roadmap.mdx) — Phase 6 review-first proposal engine and Phase 9 optional LLM layer
