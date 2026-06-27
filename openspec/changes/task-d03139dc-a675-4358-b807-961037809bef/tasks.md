## 1. Create live-hotspots.md Domain Page

- [ ] 1.1 Create `.devagent/docs/docs/live-hotspots.md` with the following sections:
  - Problem statement: isolated local `.scryrs/scryrs.db` files prevent multi-agent teams from sharing hotspot state; teams must poll or re-run `scryrs hotspots` on each machine
  - What live mode achieves: central ingest, idempotent shared scoring, current rankings via query API, replayable threshold-crossing signals via SSE
  - End-to-end live workflow: hooks/`scryrs record` submit remote batches → `scryrs server` owns persistence + dedup → accepted subject-bearing events update cumulative accumulator state atomically → clients query `GET /v1/repositories/{id}/hotspots` or stream `GET /v1/repositories/{id}/signals`
  - When to use live mode vs local batch: server-owned source of truth vs. local SQLite, exclusive modes (no dual-write), cumulative live state vs. deterministic batch re-scoring, session-scoped queries vs. full-store batch, signal streaming vs. polling `.scryrs/hotspots.json`
  - Field interpretation: domain-level explanation of `LiveHotspotsResponse` fields (score as cumulative agent effort, cursor as opaque/reserved, sessionCount semantics in cumulative vs session-scoped queries) and `HotspotSignal` fields (delta as triggering-event contribution, threshold-crossing semantics, evidence row IDs as traceability)
  - How to get started: basic `scryrs server` invocation, configuring hooks for remote mode (with cross-link to `trace-hook-contract.md` appendix for full transport configuration)
  - Related Pages section linking to `hotspots.md`, `cli-v0-contract.md`, `trace-hook-contract.md`, `architecture.mdx`, `roadmap.mdx`
- [ ] 1.2 Verify all field and behavior claims against `openspec/specs/live-hotspot-server-contract/spec.md`, `openspec/specs/live-hotspot-accumulators/spec.md`, and `crates/scryrs-cli/src/server.rs` help text
- [ ] 1.3 Ensure the page defers full endpoint tables, JSON schemas, exit codes, and allowed values to `cli-v0-contract.md` with explicit cross-reference

## 2. Update Navigation

- [ ] 2.1 Add `{ "text": "Live Hotspots", "link": "/live-hotspots" }` to the Technical section of `.devagent/docs/docs/_nav.json`, positioned after the existing Hotspots entry

## 3. Fix Stale and Inconsistent Wording

- [ ] 3.1 Fix `cli-v0-contract.md:419` (agent-facing Server command section): change "HTTP server with a single POST endpoint" to "HTTP server with three REST endpoints"
- [ ] 3.2 Fix `subjectKind` casing in `cli-v0-contract.md` `LiveHotspotsResponse` example (line ~206): change `"subjectKind": "File"` to `"subjectKind": "file"`
- [ ] 3.3 Fix `subjectKind` casing in `cli-v0-contract.md` `HotspotSignal` example (line ~235): change `"subjectKind": "File"` to `"subjectKind": "file"` and `"subjectKind": "Search"` to `"subjectKind": "search"` if present
- [ ] 3.4 Verify no other stale "single POST endpoint" or PascalCase subjectKind occurrences exist in the docs

## 4. Update hotspots.md Callout

- [ ] 4.1 Replace the "Related: Live Hotspot Server" section in `hotspots.md` (lines 117-124) with a concise cross-link paragraph:
  > **Related: Live Hotspot Server** — The live hotspot server (`scryrs server`) is a separate deployment mode that provides central ingestion, shared live state, and signal streaming for multi-agent teams. See [Live Hotspots](./live-hotspots.md) for the domain narrative and mode comparison.

## 5. Add Cross-Links from Adjacent Pages

- [ ] 5.1 Add `- [Live Hotspots](./live-hotspots.md) — domain narrative, mode comparison, and end-to-end live workflow` to the Related Pages section of `cli-v0-contract.md`
- [ ] 5.2 Add `- [Live Hotspots](./live-hotspots.md) — domain narrative and end-to-end workflow for the live hotspot server` to the Related Pages section of `trace-hook-contract.md`
- [ ] 5.3 Add `- [Live Hotspots](./live-hotspots.md) — domain-oriented explanation of live server mode, shared state, and signal streaming` to the Related Pages section of `architecture.mdx`
- [ ] 5.4 Add `- [Live Hotspots](./live-hotspots.md) — domain narrative and usage guidance for Phase 4 live server features` to the Related Pages section of `roadmap.mdx`

## 6. Verify Docs Build

- [ ] 6.1 Run the docs build (`bun run build` in `.devagent/docs/`) and confirm exit code 0 with no broken link warnings
- [ ] 6.2 Confirm the `/live-hotspots` route is present in the generated site
- [ ] 6.3 Verify all cross-links resolve correctly by spot-checking the built site