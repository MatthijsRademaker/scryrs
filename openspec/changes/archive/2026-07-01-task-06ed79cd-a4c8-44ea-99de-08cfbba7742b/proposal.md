## Why

`scryrs server` already owns the cumulative live hotspot rankings for multi-agent repositories, but the artifact-driven CLI pipeline still expects a local `.scryrs/hotspots.json` in the existing `HotspotsReport` shape. Operators using live ingest therefore cannot feed current live rankings into `scryrs graph`, `scryrs route`, or `scryrs propose` without manual translation, and any solution must keep live and local sources explicitly separate.

## What Changes

- Extend `scryrs hotspots <PATH>` with an explicit `--mode live|local` contract plus `--server-url` and `--repository-id` inputs for live export while preserving current local behavior when live mode is not selected.
- Resolve live server identity through the established remote precedence chain (CLI flags → process env → `.scryrs/.env` → `scryrs.json remote`) so operators can reuse the same live configuration patterns already used elsewhere.
- Add a dedicated live export path that queries `GET /v1/repositories/{repository_id}/hotspots?window=cumulative`, validates the remote response, converts `LiveHotspotsResponse` into the existing `HotspotsReport` envelope, and writes `<PATH>/.scryrs/hotspots.json` atomically.
- Make live provenance explicit by setting `storePath` to a `live:<query_url>` source descriptor, deriving `runMetadata` from live entries instead of local SQLite, echoing the written JSON to stdout, and printing the live source identity to stderr.
- Preserve strict source separation: live export must never open `.scryrs/scryrs.db`, score local events, or merge local and live evidence.
- Add focused tests for export success, transport/schema/identity failures, no-merge behavior, preserved local mode, and downstream `graph` → `route` → `propose` smoke using the exported artifact.
- Update help text, help JSON, and project docs to document the explicit live export mode and its no-merge behavior.

## Impact

- Affects the hotspots CLI surface and dispatch (`crates/scryrs-cli/src/dispatch.rs`, `dispatch_tests.rs`, `help_text.rs`, `help_json.rs`).
- Adds a live export implementation path in `crates/scryrs-cli/src/hotspots.rs` using existing `ureq`/remote-error patterns without changing the live server API or the `HotspotsReport` schema consumed downstream.
- Keeps downstream `scryrs graph`, `scryrs route`, and `scryrs propose` contracts unchanged; compatibility is achieved by materializing the existing artifact shape rather than teaching downstream commands new live flags.
- Keeps existing local `scryrs hotspots <PATH>` behavior intact.
- Leaves evidence provenance relabeling for graph output out of scope for this change; live-exported evidence row IDs continue to flow through the existing downstream pipeline unchanged.
