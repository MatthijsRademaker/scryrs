## 1. Adapter publishing foundation

- [x] 1.1 Add an accepted-decision publishing entrypoint in `crates/scryrs-adapter-markdown` that takes repository root and output root paths.
- [x] 1.2 Load only `.scryrs/accepted/*.json`, deserialize `ProposalReviewDecision`, validate each artifact, and treat a missing `.scryrs/accepted/` directory as empty success.
- [x] 1.3 Sort accepted decisions by `proposalId`, filter to Markdown-backed accepted content, and map each publishable artifact to `<output-root>/<target-type>/<proposal-id>.md`.
- [x] 1.4 Implement create/overwrite-only file output and loud failure behavior for malformed or semantically invalid accepted artifacts.

## 2. Markdown rendering behavior

- [x] 2.1 Replace proposal-inbox rendering assumptions with reviewed-decision rendering based only on `proposalId`, `targetType`, `acceptedContent`, `reviewer`, `decidedAt`, `rationale`, and `sourceEvidence`.
- [x] 2.2 Render each file as plain Markdown with deterministic identity, a `Review Metadata` block, the accepted body, and an `Evidence backlinks` section.
- [x] 2.3 Skip accepted `memory_patch` and `semantic_graph_grouping` artifacts without treating them as publish errors.
- [x] 2.4 Keep the adapter generic: do not depend on `scryrs-adapter-rspress`, do not assume `.devagent/docs/`, and do not read `.scryrs/proposals/` for supplementary metadata.

## 3. Verification

- [x] 3.1 Add tests proving pending proposals alone do not publish and mixed pending/accepted repositories publish accepted IDs only.
- [x] 3.2 Add tests for deterministic ordering/pathing and byte-stable reruns with the same input and output root.
- [x] 3.3 Add tests for metadata/evidence rendering, including preservation of row IDs and optional evidence fields when present.
- [x] 3.4 Add tests proving malformed accepted artifacts fail loudly, non-Markdown accepted artifacts are skipped, and missing `.scryrs/accepted/` is a no-op success.

## 4. Documentation

- [x] 4.1 Update `.devagent/docs/docs/proposals.md` to state that Markdown publishing consumes reviewed `.scryrs/accepted/` artifacts rather than raw proposal inbox files.
- [x] 4.2 Update `.devagent/docs/docs/production-suite.md` to describe generic Markdown publishing as the reviewed-knowledge release step before any Rspress-specific surface.

## 5. Scope guardrails

- [x] 5.1 Do not add a new `scryrs publish ...` or `scryrs markdown ...` CLI command in this change.
- [x] 5.2 Do not add stale-output deletion, Rspress routing/frontmatter behavior, or proposal-inbox publishing shortcuts in this foundation slice.
