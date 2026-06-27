## Why

Live hotspot documentation exists today, but it is spread across architecture, roadmap, CLI-contract, and remote-mode appendix pages that mostly describe endpoints, schemas, and internal mechanics. Readers can find how the server is built, yet still have to infer what live hotspots are for, what problem shared live state solves, when to use it instead of local batch hotspots, and how remote ingest, accumulators, queries, and signals fit together as a product workflow.

Every existing live-hotspot reference is architecture or contract framed:

- `hotspots.md` mentions live mode only as a 7-line callout at lines 117-124
- `architecture.mdx` lists live accumulator and endpoint bullets as crate responsibilities
- `roadmap.mdx` describes Phase 4 through required deliverables and accepted limitations, not user outcomes
- `cli-v0-contract.md` documents endpoints, schemas, and exit codes but not the domain story
- `trace-hook-contract.md` explains transport/identity rules but not why remote mode matters

A developer or team evaluating scryrs for multi-agent CI/harness setups must currently jump between five pages to reconstruct the answer to "should we run a live server and what will it give us?" This change fills that gap.

## What Changes

1. **New domain-first live hotspot page** — Create `.devagent/docs/docs/live-hotspots.md` mirroring the successful `hotspots.md` pattern: open with the operational pain (isolated local `.scryrs/scryrs.db` files prevent multi-agent shared visibility), explain what live mode achieves (central ingest, idempotent shared scoring, current rankings, replayable signals), explain the end-to-end live workflow in domain language, clearly distinguish live server mode from local batch mode, and include field-level interpretation for `LiveHotspotsResponse` and `HotspotSignal` where it adds domain understanding beyond the contract schema.

2. **Navigation update** — Add a "Live Hotspots" entry as a sibling under the Technical section of `_nav.json`, positioned after the existing "Hotspots" entry.

3. **Cross-links** — Add bidirectional cross-links from `hotspots.md`, `cli-v0-contract.md`, `trace-hook-contract.md`, `architecture.mdx`, and `roadmap.mdx` to the new `live-hotspots.md` page.

4. **hotspots.md callout update** — Replace the existing "Related: Live Hotspot Server" section in `hotspots.md` (lines 117-124) with a concise cross-link pointing to `live-hotspots.md`, removing duplicated server endpoint descriptions.

5. **Stale wording fix** — Correct the "HTTP server with a single POST endpoint" text at `cli-v0-contract.md:419` (agent-facing Server command section) to "HTTP server with three REST endpoints", matching the correct three-endpoint documentation at line 132 of the same file and the server help text at `crates/scryrs-cli/src/server.rs:17-28`. The main `scryrs server` command table at line 132 already says "three REST endpoints" and requires no change.

6. **subjectKind casing fix** — Fix the `subjectKind` casing in live hotspot response examples at `cli-v0-contract.md:206` and `cli-v0-contract.md:235` from PascalCase (`"File"`, `"Search"`) to lowercase (`"file"`, `"search"`) to match the actual server serialization confirmed in `crates/scryrs-types/src/lib.rs` and `crates/scryrs-server/src/server.rs`.

## Impact

- **Docs site only** — No Rust source code, Cargo configuration, OpenSpec specs (outside this change's own specs directory), LikeC4 diagrams, test fixtures, CI configuration, or root README.md are modified.
- **No runtime behavior change** — Server behavior, APIs, schemas, scoring, and signal semantics are unchanged.
- **Docs build** — The Rspress docs site must build successfully with no broken links or build errors after changes.
- **Reader experience** — Live hotspot information becomes discoverable through a single, coherent domain-first page linked from navigation and all adjacent docs. Stale/conflicting server wording is corrected so cross-referenced pages are consistent.