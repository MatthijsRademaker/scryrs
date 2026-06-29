## Why

`scryrs-adapter-markdown` currently renders raw `ProposalDocument` inbox content and explicitly does not publish reviewed artifacts. The review workflow now records accepted evidence under `.scryrs/accepted/` as `ProposalReviewDecision`, but there is no generic Markdown path that promotes that reviewed knowledge into stable files.

This change closes that review-to-publish gap for Markdown without coupling the generic adapter to Rspress. It ensures only accepted evidence is publishable, pending `.scryrs/proposals/*.json` suggestions remain non-authoritative, and published Markdown preserves the reviewed rationale and evidence backlinks that justify the content.

## What Changes

1. Extend `crates/scryrs-adapter-markdown` from a proposal renderer into a library-only publishing foundation centered on accepted `ProposalReviewDecision` artifacts under `.scryrs/accepted/`.
2. Add a publishing entrypoint that takes a repository root plus caller-supplied output root, validates accepted decisions, sorts them by `proposalId`, filters to Markdown-backed accepted content, and writes deterministic files at `<output-root>/<target-type>/<proposal-id>.md`.
3. Publish all Markdown target types supported by the review contract (`docs_note`, `adr`, `skill`, `debugging_playbook`), while skipping non-Markdown accepted decisions (`memory_patch`, `semantic_graph_grouping`) without treating them as errors.
4. Render each published file as plain Markdown with a deterministic review-metadata block (`proposalId`, `targetType`, `reviewer`, `decidedAt`, `rationale`), followed by the accepted body and an `Evidence backlinks` section derived from `sourceEvidence`.
5. Keep the foundation generic: create/overwrite current target files only, do not delete stale output, do not read proposal inbox titles or other `.scryrs/proposals/` metadata, do not depend on `scryrs-adapter-rspress`, and do not assume `.devagent/docs/` layout or Rspress frontmatter/routes.
6. Add automated tests for accepted-only input, deterministic ordering/pathing, idempotent reruns, malformed accepted artifacts failing loudly, missing `.scryrs/accepted/` as empty success, pending proposals ignored, evidence/rationale rendering, and non-Markdown accepted decisions being skipped.
7. Update `.devagent/docs/docs/proposals.md` and `.devagent/docs/docs/production-suite.md` to describe the accepted-evidence-to-Markdown publishing boundary and the generic Markdown-before-Rspress sequencing.

## Impact

- **Code:** `crates/scryrs-adapter-markdown` gains accepted-decision loading, validation, deterministic pathing, file output, and reviewed-markdown rendering.
- **Tests:** Adapter tests expand to cover publish semantics, failure modes, and determinism guarantees.
- **Docs:** `proposals.md` and `production-suite.md` are updated to reflect that Markdown publishing consumes `.scryrs/accepted/`, not the proposal inbox.
- **CLI surface:** No new `scryrs publish ...` or `scryrs markdown ...` command is introduced in this foundation slice; CLI contract work is deferred.
- **Architecture:** The Markdown adapter remains a generic publishing surface independent from the separate Rspress adapter.