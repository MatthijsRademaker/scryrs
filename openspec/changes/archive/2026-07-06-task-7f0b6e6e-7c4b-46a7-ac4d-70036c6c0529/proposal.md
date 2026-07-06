## Why

The project documentation surface mixes three different states: commands that are already shipped, product loops that are only partially closed, and older placeholder limitations that no longer match code. The worst drift is in `roadmap.mdx` and `production-suite.md`, while `proposals.md` and `route-manifests.md` are largely accurate but need audit confirmation.

Concrete inaccuracies identified through refinement:

- **roadmap.mdx Phase 6** claims `debugging_playbook` remains excluded from v1 generation — but `crates/scryrs-curator/src/lib.rs` ships `debugging_playbook_proposal()` at FailedLookup >= 2.
- **roadmap.mdx Phase 6** omits the shipped `scryrs proposals list|accept|reject` review CLI surface entirely.
- **roadmap.mdx Phase 7** has no partial-delivery note despite `scryrs publish markdown` and `scryrs publish rspress` being shipped.
- **roadmap.mdx Phase 8** lists all three required deliverables as future work, even though `scryrs route explain --query`, route hints, and stable reason strings are all shipped.
- **roadmap.mdx Production-Ready Suite Path P4** lists route-explain as blocking future work — already shipped.
- **roadmap.mdx Near-term milestones M7, M9, M10** describe shipped behavior as future milestones without a shipped marker.
- **production-suite.md Current State route-manifests row** says 'Runtime explanation and context loading decisions missing' — `scryrs route explain` is shipped.
- **production-suite.md P4 milestone** lists shipped items under 'Must ship' without the ✅ Shipped marker that P3 has.

## What Changes

Surgical edits to four doc files anchored on `cli-v0-contract.md` plus Rust CLI/runtime sources as the truth baseline:

1. **`roadmap.mdx`** — Add partial-delivery markers to Phases 6, 7, and 8; fix stale `debugging_playbook` claim; add shipped review CLI to Phase 6; mark near-term milestones M7, M9, M10 as delivered; update Production-Ready Suite Path P4 to reflect shipped route explain.
2. **`production-suite.md`** — Rewrite route-manifests current-state gap from 'explanation missing' to 'explanation shipped, automatic context loading missing'; add ✅ Shipped marker to P4 milestone items; link remaining real gaps (live-signal-feed-motion, public-binary-and-image-distribution).
3. **`proposals.md`** — Audit-only: confirm accepted-only publish boundary, review-first semantics, and three-zone artifact layout already match shipped behavior. No edits unless wording drifts from source truth.
4. **`route-manifests.md`** — Audit-only: confirm deterministic matching, packed relevance, loadTarget/non_loadable boundaries already match shipped behavior. No edits unless wording drifts from source truth.

Verify through the existing `scripts/check` lane (which already runs `verify-docs-publish`).

## Impact

- Project developers see accurate distinction between shipped command surfaces and product-complete behavior in roadmap and production-suite docs.
- Roadmap Phases 6-8 explicitly mark what is shipped (`scryrs proposals ...`, debugging_playbook, `scryrs publish ...`, `scryrs route explain|bundle`) and what remains deferred.
- Production-suite status tables stop misrepresenting shipped commands as missing and instead point to real remaining gaps.
- No code, CLI contract, adapter, or runtime behavior changes.
- No canonical OpenSpec repo publication.