## Context

The repository has a proven Markdown publisher (`publish_accepted_markdown` in `crates/scryrs-adapter-markdown`) that loads only `.scryrs/accepted/*.json`, validates `ProposalReviewDecision`, writes deterministic `<output-root>/<target-type>/<proposal-id>.md` for the four Markdown-backed target types (`docs_note`, `adr`, `skill`, `debugging_playbook`), skips non-Markdown types (`memory_patch`, `semantic_graph_grouping`), and renders review provenance with evidence backlinks — all without Rspress coupling.

The Rspress adapter crate (`crates/scryrs-adapter-rspress`) exists as a stub with only `descriptor()` and a passive `RspressRoute` struct. The docs site at `.devagent/docs/` builds via `bun run build` and emits `doc_build/llms.txt` and `doc_build/llms-full.txt` (configured via `rspress.config.ts` with `llms: true`). The current `_nav.json` has three hand-authored sections with no generated content area. Neither `scripts/check` nor `.github/workflows/ci.yml` runs the docs build.

## Goals / Non-Goals

### Goals

- Define the Rspress adapter as a downstream publishing layer over accepted knowledge, not a new source of truth.
- Publish approved Markdown-backed knowledge into the existing Rspress docs source tree deterministically.
- Update docs navigation (`_nav.json`) and llms-facing outputs in a repeatable way that survives reruns without drift.
- Preserve the existing review-first boundary: pending proposals and rejected artifacts must never become published docs.
- Add build verification that proves the resulting Rspress site and generated llms surfaces remain valid and include published knowledge.

### Non-Goals

- Do not change `ProposalReviewDecision`, proposal generation, or accepted/rejected ledger semantics just to satisfy Rspress.
- Do not publish directly from `.scryrs/proposals/` or invent a bypass around accepted evidence.
- Do not let Rspress frontmatter, route shape, or navigation concerns leak back into `scryrs-core`, `scryrs-curator`, `scryrs-graph`, or generic Markdown contracts.
- Do not turn this slice into a new docs-framework abstraction or a generic CLI/product-surface redesign.

## Decisions

### Decision 1: Compile-time dependency on scryrs-adapter-markdown

The Rspress adapter adds `scryrs-adapter-markdown` as a Cargo dependency and calls `publish_accepted_markdown()` into an ephemeral temp scratch directory. It then reads, transforms, and relocates those files into the Rspress docs tree. This avoids duplicating accepted-only parsing, `proposalId` sorting, `ProposalReviewDecision` validation, non-Markdown skip logic, and rendering.

The Markdown adapter spec states "SHALL not depend on scryrs-adapter-rspress" — this is a unidirectional upstream→downstream constraint. The Rspress adapter depending on the Markdown adapter is architecturally sound as a downstream consumer. The reviewer's staging-consumer concern was resolved in favor of the architect's and lead-dev's recommendations since the Rust ecosystem pattern favors library dependencies over CLI subprocess invocations, and the downstream direction preserves the framework-agnostic property of the Markdown adapter.

### Decision 2: Staging through temp directory

The Rspress adapter calls `publish_accepted_markdown(repo_root, temp_dir)` where `temp_dir` is ephemeral. It then reads the published files, transforms them (adding Rspress frontmatter), and writes them into `.devagent/docs/docs/accepted-knowledge/`. The temp directory provides a clean interface boundary — the Rspress adapter owns transformation and placement without depending on the Markdown adapter's internal rendering format.

### Decision 3: Generated subtree at accepted-knowledge/

Pages are written to `.devagent/docs/docs/accepted-knowledge/<target-type-slug>/<proposal-id>.md`. This dedicated subtree prevents collision with hand-authored docs, makes adapter ownership explicit, and enables declarative cleanup: the adapter removes the `accepted-knowledge/` directory before regenerating.

### Decision 4: Rspress frontmatter confined to the adapter

Generated pages carry minimal Rspress-specific frontmatter added by the adapter: `title` (derived from target type + proposal ID) and `sidebar_label` (target-type-slug:truncated-proposal-id). No new Rspress-driven fields or variants are added to `scryrs-types` contracts.

### Decision 5: Nav merge: strip-and-rebuild

The adapter reads `_nav.json`, finds and removes any existing section where `text == "Accepted Knowledge"`, then appends a fresh section. Within it, per-target-type sub-sections appear only for types with published artifacts, with items sorted by `proposalId` ascending. Reruns produce byte-identical `_nav.json`.

### Decision 6: Build verification as a separate script

`scripts/verify-docs-publish` creates test fixtures, runs the adapter, executes `bun run build` in `.devagent/docs/`, and asserts `doc_build/llms.txt` and `doc_build/llms-full.txt` contain published proposal IDs as substrings (resilient to format drift). It lives outside the Rust crate because it depends on Node.js/Bun.

### Decision 7: Library-only, no CLI binary

The adapter exposes `publish_accepted_rspress(repository_root, docs_root) -> Result<Vec<PublishEntry>, PublishError>` as a library API. The return type `PublishEntry` provides machine-readable metadata for callers to assert against.

### Decision 8: Declarative subtree cleanup

The adapter removes `.devagent/docs/docs/accepted-knowledge/` at the start of each publish run, then regenerates from scratch (clear-and-regenerate model). Simpler and safer than tracking a write-set.

## Risks

| Risk | Mitigation |
| --- | --- |
| **Two-pass accepted read:** Rspress adapter reads `.scryrs/accepted/` again for metadata after Markdown adapter already traversed it. Minor build-time inefficiency. | Keep the metadata pass lightweight — only parse fields needed for frontmatter/nav, not re-validate proposal semantics. |
| **Stale page orphaning:** Removed accepted decisions would leave orphaned pages. | Clear `accepted-knowledge/` at start of each run (declarative model). |
| **Nav ownership conflict:** Adapter owns "Accepted Knowledge" section exclusively — hand-edits overwritten. | Document ownership in project docs. Section key is stable merge point. |
| **Build reliability:** Verification script depends on Node.js/Bun availability. | Script fails loudly with clear messages about missing bun/node. |
| **llms.txt format drift:** Rspress may change llms output format between versions. | Verification checks for proposal ID substrings, not exact line matching. |
| **Markdown adapter transitively breaks Rspress adapter:** | Acceptable — task explicitly requires Rspress adapter to be downstream. |
| **Nav changes between runs:** Hand-authored `_nav.json` could change. | Adapter reads `_nav.json` fresh each run, rebuilds section from current artifacts. |
| **Non-Markdown types accidentally produce nav entries:** | Adapter iterates Markdown adapter's published output (already filtered), not `.scryrs/accepted/` directly. |

## Conflict Resolution

Round 1 surfaced a design disagreement: whether the Rspress adapter should depend on `scryrs-adapter-markdown` at compile time (architect + lead-dev) or call it as a separate process through a staging directory (reviewer).

The reviewer argued the Markdown adapter's spec requirement "SHALL not depend on scryrs-adapter-rspress" implies symmetry. Inspection of `openspec/specs/markdown-publishing-adapter/spec.md` shows this is explicitly unidirectional: the Markdown adapter must not depend on the Rspress adapter. Nothing forbids the reverse. Downstream depending on upstream is the standard dependency direction.

**Resolution:** Adopt compile-time dependency. The Markdown adapter remains framework-agnostic. The Rspress adapter consumes its output as-is, only transforming placement and adding frontmatter.

## Traceability

- **Task / dossier:** `61a39608-2975-47a3-a9bc-0f57fe42463c`, dossier `2026-06-29T20:11:24.055Z`
- **Accepted decisions:** `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, `1-swarm-reviewer-recommendation`
- **Repository evidence:** `crates/scryrs-adapter-markdown/src/lib.rs`, `crates/scryrs-adapter-rspress/src/lib.rs`, `.devagent/docs/docs/_nav.json`, `scripts/check`, `.github/workflows/ci.yml`, `.devagent/docs/package.json`, `rspress.config.ts`, `openspec/specs/markdown-publishing-adapter/spec.md`
- **Conflict resolution:** Reviewer's staging-consumer concern resolved per above analysis.