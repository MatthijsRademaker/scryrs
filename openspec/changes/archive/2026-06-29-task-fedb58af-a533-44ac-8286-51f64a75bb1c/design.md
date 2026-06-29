## Context

The current Markdown adapter is a narrow foundation that renders `ProposalDocument` Markdown from the proposal inbox and explicitly does not perform publishing, file I/O, or review-workflow behavior. Meanwhile, the accepted review path already exists: `scryrs proposals accept` writes versioned `ProposalReviewDecision` artifacts under `.scryrs/accepted/`, carrying the reviewed content, rationale, reviewer metadata, decision timestamp, and source evidence needed for publication.

This task defines the first generic Markdown publishing foundation that promotes reviewed knowledge out of `.scryrs/accepted/` into deterministic Markdown files without depending on Rspress conventions or reading pending inbox proposals.

## Goals / Non-Goals

**Goals**

- Publish reviewed Markdown knowledge from `.scryrs/accepted/*.json` only.
- Produce deterministic paths and byte-stable reruns for the same accepted input and output root.
- Preserve accepted body content plus review provenance and evidence backlinks in the rendered Markdown.
- Support all Markdown-backed accepted target types: `docs_note`, `adr`, `skill`, and `debugging_playbook`.
- Treat missing `.scryrs/accepted/` as valid empty input and fail loudly on malformed or semantically invalid accepted decisions.
- Keep the generic Markdown adapter independent of `scryrs-adapter-rspress`, `.devagent/docs/` layout assumptions, Rspress routes, and frontmatter.
- Update proposal and production-suite docs so the publish boundary matches the reviewed-evidence model.

**Non-Goals**

- No new CLI command or CLI help/dispatch contract in this foundation slice.
- No publishing from `.scryrs/proposals/` or `.scryrs/rejected/`.
- No stale-output deletion or mirror-mode cleanup.
- No Rspress route generation, `.devagent/docs/` writes, or `llms` surface generation.
- No proposal auto-acceptance, proposal generation changes, graph ingestion changes, or dashboard review UX.
- No LLM summarization or rewriting of accepted content.

## Decisions

### Decision 1: Scope is a library-only publishing foundation

Implement the Markdown publisher inside `crates/scryrs-adapter-markdown` as a library API, using the accepted-decision artifact set plus a caller-supplied output root. The long-lived CLI contract for publish commands is explicitly deferred.

### Decision 2: Accepted review decisions are the only publish input

The publisher reads `.scryrs/accepted/*.json`, deserializes `ProposalReviewDecision`, validates each artifact, and never consults `.scryrs/proposals/*.json` for titles or other metadata. This preserves the review-first trust boundary and prevents pending proposals from becoming publishable by accident.

### Decision 3: Deterministic pathing uses only accepted-decision fields

Because `ProposalReviewDecision` does not carry proposal titles, file naming and fallback identity must use `targetType` and `proposalId` only. Published files live at:

`<output-root>/<target-type>/<proposal-id>.md`

Loaded decisions are sorted by `proposalId` ascending before publication so filesystem iteration order cannot affect output.

### Decision 4: All Markdown-backed accepted target types are in scope

This foundation publishes accepted `docs_note`, `adr`, `skill`, and `debugging_playbook` content. Accepted `memory_patch` and `semantic_graph_grouping` artifacts are non-Markdown and are skipped rather than treated as publish errors.

### Decision 5: Render plain Markdown with review metadata and evidence backlinks

Each published file is generic Markdown only:

1. deterministic heading/identity derived from `proposalId` and `targetType`
2. `Review Metadata` block containing `proposalId`, `targetType`, `reviewer`, `decidedAt`, and `rationale`
3. accepted Markdown body from `acceptedContent`
4. `Evidence backlinks` section that includes every `sourceEvidence` entry and preserves source kind, subject, row IDs when present, and any doc reference, description, or score fields when present

No frontmatter, Rspress route hints, or `.devagent/docs/` assumptions are allowed.

### Decision 6: Create/overwrite only; no cleanup semantics in this slice

The publisher writes or overwrites current deterministic targets only. It does not delete orphaned Markdown files when accepted artifacts disappear. Cleanup policy is deferred to a later task.

### Decision 7: Empty input and invalid input have different outcomes

A missing `.scryrs/accepted/` directory is a valid no-op success. Malformed JSON, schema mismatch, or semantic validation failure in an accepted artifact fails loudly instead of producing partial fake success.

## Risks

- **Skipped non-Markdown decisions can become invisible if tests are weak.** Cover `memory_patch` and `semantic_graph_grouping` fixtures so the skip policy is explicit and regression-proof.
- **`ProposalReviewDecision` has no title field.** Any attempt to pull titles from `.scryrs/proposals/` would violate the accepted-only boundary; headings and filenames must stay derived from `proposalId` and `targetType`.
- **Filesystem iteration is nondeterministic.** Loading must sort by `proposalId` before writing.
- **Docs can drift from the new boundary.** `proposals.md` and `production-suite.md` must be updated in the same change so the reviewed-evidence publishing contract is documented where the current gap is described.

## Conflict Resolution

Round 1 surfaced three disagreements: whether this slice needed a CLI contract, whether it should publish only `docs_note` or all Markdown target types, and whether stale output should be deleted. The accepted architect decision resolves those conflicts for this change:

- no new CLI surface here; ship a library-only publisher
- publish all Markdown-backed accepted target types
- skip non-Markdown accepted decisions without making them errors
- use create/overwrite semantics only and defer stale-file deletion

These resolutions keep the task tightly aligned with the backlog scope: closing the accepted-evidence-to-Markdown foundation without introducing new product-surface decisions.

## Traceability

- **Task / dossier:** backlog task `fedb58af-a533-44ac-8286-51f64a75bb1c` and dossier timestamp `2026-06-29T17:18:01.263Z` define the accepted-only publishing problem, deterministic output requirements, Rspress-independence boundary, and required docs updates.
- **Accepted decisions:** `1-swarm-architect-recommendation` resolves the open product questions by selecting a library-only API, all Markdown target types, non-Markdown skip behavior, metadata/backlink rendering, create/overwrite semantics, and `proposalId` sorting. Lead-dev and reviewer recommendations contribute the constraint that CLI work is not required for this foundation and must not be invented.
- **Repository evidence:** `crates/scryrs-adapter-markdown/src/lib.rs` currently renders `ProposalDocument` only; `crates/scryrs-types/src/lib.rs` defines `ProposalReviewDecision`; `.devagent/docs/docs/proposals.md` and `production-suite.md` document the review-first boundary and current publishing gap; `crates/scryrs-adapter-rspress/src/lib.rs` confirms the separate Rspress adapter boundary.