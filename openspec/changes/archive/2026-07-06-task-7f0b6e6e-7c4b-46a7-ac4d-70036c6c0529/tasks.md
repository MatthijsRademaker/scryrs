## 1. Roadmap docs truth

- [x] 1.1 Fix Phase 6 `debugging_playbook` claim: remove "`debugging_playbook` remains excluded from v1 generation" and instead state it is generated when FailedLookup count >= 2.
- [x] 1.2 Add Partial delivery note to Phase 6 naming shipped review CLI (`scryrs proposals list|accept|reject`), shipped proposal generator (`scryrs propose`), and shipped debugging_playbook generation. List remaining gaps: no dashboard review UX, no broader accepted-evidence consumers.
- [x] 1.3 Add Partial delivery note to Phase 7 naming shipped `scryrs publish markdown` and `scryrs publish rspress` commands. List remaining gap: broader docs-surface targets beyond Markdown/Rspress.
- [x] 1.4 Restructure Phase 8 from future-work framing to Partial delivery note. Mark `scryrs route explain`, `scryrs route bundle`, route hint schema, and explain_hints as shipped. Explicitly state this is NOT product-complete: retrieval is deterministic manifest matching with packed display relevance, explicit non_loadable targets, and no automatic context loading.
- [x] 1.5 Update Production-Ready Suite Path P4 row: replace "Define route hint contract and implement `scryrs route explain`" with the actual remaining gap (route explain shipped; semantic ranking and automatic context loading remain deferred).
- [x] 1.6 Mark near-term milestones M7 (review loop beta), M9 (runtime explain beta), and M10 (adapter suite beta) as delivered with ✅ Shipped marker.

## 2. Production suite docs truth

- [x] 2.1 Rewrite route-manifests Current State gap from "Runtime explanation and context loading decisions missing" to reflect shipped explain: "`scryrs route explain` shipped; automatic context-loading loop still missing (deterministic manifest matching only, no semantic retrieval or autonomous source/docs loading)."
- [x] 2.2 Add ✅ Shipped marker to P4 milestone Must-ship column for route hint schema, `scryrs route explain`, and deterministic evidence-backed reasons. Note remaining gap is semantic usefulness / automated context loading.
- [x] 2.3 Link remaining real gaps to active OpenSpec changes where applicable (live-signal-feed-motion for live-hotspot browser verification, public-binary-and-image-distribution for release distribution).

## 3. Audit proposals and route-manifests docs

- [x] 3.1 Audit `proposals.md`: confirm accepted-only publish boundary, review-first semantics, three-zone artifact layout, debugging_playbook generation, and review CLI commands all match shipped behavior. Only edit if wording drifts from source truth.
- [x] 3.2 Audit `route-manifests.md`: confirm deterministic matching, packed relevance, explicit loadTarget/non_loadable boundaries, route bundle contract, and explain ranking semantics all match shipped behavior. Only edit if wording drifts from source truth.

## 4. Docs build verification

- [x] 4.1 Run `scripts/check` and confirm the docs publish/build lane succeeds without errors.
- [x] 4.2 If docs verification fails, debug the failure against the specific docs content edits made in tasks 1-2.
