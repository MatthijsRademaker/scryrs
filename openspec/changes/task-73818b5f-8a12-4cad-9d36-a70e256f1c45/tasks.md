## 1. Production store query methods

- [ ] 1.1 Add `query_hotspots(repository_id, window)` to `ServerStore` that fetches all accumulator rows for given repository+window, materializes `HotspotEntry` with counts/evidence/sessionCount from stored JSON, applies six-key tie-break sorting, assigns 1-based ranks.
- [ ] 1.2 Add `poll_signals(repository_id, after_id, limit)` to `ServerStore` that queries `hotspot_signals` ordered by `id ASC` with optional `WHERE id > ?` and row limit.
- [ ] 1.3 Unit tests for `query_hotspots`: empty repository, single entry, multi-entry sorting, tie-break on firstEvidenceId, unknown window.
- [ ] 1.4 Unit tests for `poll_signals`: empty result, id order, after_id filter, limit truncation, repository isolation.

## 2. SSE signal payload type

- [ ] 2.1 Add `HotspotSignalEvent` struct to `scryrs-types` with `id: i64` plus all `HotspotSignal` fields.
- [ ] 2.2 Add serde round-trip test for `HotspotSignalEvent`.

## 3. Hotspot query endpoint

- [ ] 3.1 Add `HotspotQueryParams` query parameter model to `scryrs-types` with `window: Option<String>`, `session_id: Option<String>`.
- [ ] 3.2 Register `GET /v1/repositories/{repository_id}/hotspots` on server router.
- [ ] 3.3 Implement handler: validate window (only cumulative supported), reject session_id with deferral error, call `store.query_hotspots`, build `LiveHotspotsResponse`.
- [ ] 3.4 Integration tests: valid query, unknown repo empty entries, unsupported window 400, session_id 400, no filesystem-path fields.

## 4. SSE signal stream endpoint

- [ ] 4.1 Add SSE dependencies to `scryrs-server/Cargo.toml` (tokio-stream or equivalent).
- [ ] 4.2 Register `GET /v1/repositories/{repository_id}/signals` on server router.
- [ ] 4.3 Implement SSE handler: open separate read-only `rusqlite::Connection`, accept `after` query param, poll signals with `WHERE id > ?`, produce SSE frames with `id:` and `data:`.
- [ ] 4.4 Handle keepalive (30s heartbeat comments) and graceful disconnect cleanup.
- [ ] 4.5 Integration tests: Content-Type header, id ordering, after replay, empty repository, disconnect cleanup.

## 5. Spec and contract updates

- [ ] 5.1 Create `openspec/specs/live-hotspot-query-stream/spec.md` with new endpoint contracts.
- [ ] 5.2 Update `live-hotspot-server-contract` spec: replace deferred signal payload placeholder with concrete `HotspotSignalEvent` reference.

## 6. Validation

- [ ] 6.1 `cargo test --workspace` — all existing and new tests pass.
- [ ] 6.2 `cargo clippy --workspace` — no new warnings.
- [ ] 6.3 Hotspot query matches batch `score_hotspots` output for equivalent event sets.
- [ ] 6.4 Signal stream ordering deterministic under concurrent same-second writes.
- [ ] 6.5 `.scryrs/hotspots.json` export path unaffected by new APIs.