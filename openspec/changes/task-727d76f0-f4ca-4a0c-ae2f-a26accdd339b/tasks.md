## 1. Remote config and transport foundation

- [ ] 1.1 Add a shared remote-config resolver in `crates/scryrs-cli` that discovers the nearest ancestor `scryrs.json`, reads optional `remote` defaults, applies environment overrides, treats a missing or empty ingest URL as local mode, and returns deterministic local-vs-remote transport mode before input/store I/O.
- [ ] 1.2 Add a shared remote-submit layer in `crates/scryrs-cli` with a blocking HTTP client (`ureq`) plus a testable transport abstraction.
- [ ] 1.3 Derive deterministic `producer_event_id` values from canonical `TraceEvent` JSON plus 1-based physical line number and build `ServerIngestEnvelope` version `1.0.0` batches from accepted events only.

## 2. `scryrs record` remote-mode behavior

- [ ] 2.1 Preserve the existing local `scryrs record --stdin/--file` path unchanged when remote mode is absent, including local SQLite writes, stdout summary shape, rejection diagnostics, and exit semantics.
- [ ] 2.2 In remote mode, reuse local JSONL validation, skip local `EventStore` open/create/write behavior entirely, and submit one remote batch for the invocation's accepted events.
- [ ] 2.3 Map `BatchIngestResponse` results to deterministic remote summary counts for `accepted`, `duplicate`, `rejected`, and `failed`, with duplicates non-fatal, per-item rejections producing exit `1`, and transport/server failures producing loud exit `2` diagnostics with no fake success.
- [ ] 2.4 Enforce the remote timeout contract with a default of `3000` ms and configurable override through `SCRYRS_REMOTE_TIMEOUT_MS`.

## 3. Hook-path and configuration-surface integration

- [ ] 3.1 Keep Pi and Claude integrations transport-dumb by leaving HTTP logic out of hook shims and hook configuration.
- [ ] 3.2 Reuse the shared CLI-owned remote transport path wherever the existing hook-facing ingestion topology needs it, while preserving hook fail-open semantics.
- [ ] 3.3 Extend `scryrs.json` documentation/contract with optional `remote` defaults for `ingest_url`, `repository_id`, `workspace_id`, `agent_id`, and `timeout_ms`.

## 4. Discovery and documentation updates

- [ ] 4.1 Update `scryrs --help`, `scryrs --help-json`, and other touched CLI discovery surfaces to describe the unchanged local default, explicit remote mode, config precedence, remote counts, timeout behavior, and loud transport-failure semantics.
- [ ] 4.2 Update the README and relevant CLI/hook contract docs to explain that remote mode is CLI-owned, skips local SQLite, and does not add offline retry or hook-side HTTP logic.

## 5. Verification

- [ ] 5.1 Add tests for local-vs-remote mode selection, including proof that remote mode does not open or create `.scryrs/scryrs.db`.
- [ ] 5.2 Add tests for config precedence and ancestor `scryrs.json` discovery.
- [ ] 5.3 Add tests for missing remote identity, deterministic `producer_event_id` replay, successful accepted batches, duplicate acknowledgements, and per-item server rejections.
- [ ] 5.4 Add tests for timeout, connection, non-2xx, and malformed-response failures with deterministic exit-2 diagnostics.
- [ ] 5.5 Re-run or extend hook-path regressions as needed to prove hook integrations stay HTTP-free and preserve fail-open behavior when the shared remote transport is reused.