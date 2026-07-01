## 1. Extend the hotspots CLI contract

- [ ] 1.1 Add `--mode local|live`, `--server-url`, and `--repository-id` to `scryrs hotspots <PATH>` while preserving the current local default when live mode is not selected.
- [ ] 1.2 Resolve live server URL and repository ID through the established precedence chain (CLI flags → process env → `.scryrs/.env` → `scryrs.json remote`) and update dispatch/error tests for the new valid flags.
- [ ] 1.3 Update help text and `--help-json` to describe live mode, input precedence, and the no-merge behavior.

## 2. Implement live hotspot export

- [ ] 2.1 Add a dedicated live-mode helper in `crates/scryrs-cli/src/hotspots.rs` that fetches `GET /v1/repositories/{repository_id}/hotspots?window=cumulative` through a testable fetch seam and never opens local SQLite.
- [ ] 2.2 Validate HTTP status, JSON shape, live schema version, and returned repository identity before converting `LiveHotspotsResponse` into `HotspotsReport`.
- [ ] 2.3 Populate live-exported artifact metadata with `generatedAt` from the response, `storePath` as `live:<query_url>`, and live-derived `runMetadata` values.
- [ ] 2.4 Write `<PATH>/.scryrs/hotspots.json` atomically, mirror the same JSON to stdout, and print live source identity to stderr.

## 3. Preserve local/live separation and downstream compatibility

- [ ] 3.1 Ensure live export never reads `.scryrs/scryrs.db`, never merges local-only subjects, and leaves any existing hotspot artifact untouched on live fetch or validation failure.
- [ ] 3.2 Preserve the existing local `scryrs hotspots <PATH>` path and its current end-to-end coverage.
- [ ] 3.3 Verify the exported artifact is accepted unchanged by `scryrs graph`, `scryrs route`, and `scryrs propose`.

## 4. Add focused verification

- [ ] 4.1 Add tests for live export success, config-precedence resolution, local/live no-merge behavior, non-2xx responses, timeouts/connection failures, malformed JSON, schema-version mismatch, repository-ID mismatch, and no-partial-write behavior.
- [ ] 4.2 Add a downstream smoke test covering `hotspots --mode live` export followed by `graph`, `route`, and `propose` on the same fixture repository.

## 5. Update project documentation

- [ ] 5.1 Update `.devagent/docs/docs/hotspots.md` to describe explicit live artifact export.
- [ ] 5.2 Update `.devagent/docs/docs/live-hotspots.md` to document live export as a server-owned source materialization path and to restate that local and live sources do not merge.
- [ ] 5.3 Update `.devagent/docs/docs/cli-v0-contract.md` to document the `scryrs hotspots` live-mode flags and artifact behavior.
