## Why

`scryrs record` already provides the local JSONL ingestion boundary, and the live-hotspot server contract already defines the remote batch API, but there is still no CLI transport that forwards accepted events to a configured server. That gap blocks live multi-agent hotspot collection and forces a choice between local-only SQLite writes or pushing HTTP logic into hook integrations.

This change closes that gap by making remote ingest an explicit `scryrs record` mode while preserving the current local default. It also keeps Pi and Claude integrations transport-dumb by leaving networking inside the CLI-owned Rust ingestion path, and it makes transport failures loud and deterministic instead of pretending success.

## What Changes

1. **Add explicit remote transport mode to `scryrs record`**: resolve remote configuration from `scryrs.json` plus environment variables, keep local mode as the default when no ingest URL is configured, and fail before any network call when required remote identity cannot be resolved.
2. **Reuse local validation and submit one remote batch**: continue parsing and validating JSONL locally, emit the same deterministic rejection diagnostics for malformed or schema-invalid lines, derive stable `producer_event_id` values for accepted events, and submit one `ServerIngestEnvelope` batch to `POST /v1/trace-events/batch` with a bounded timeout.
3. **Make remote results and failures deterministic**: keep the existing local summary contract unchanged, add remote-mode summary counts for `accepted`, `duplicate`, `rejected`, and `failed`, and treat timeout, connection, non-2xx, and malformed-response failures as loud exit-2 errors with no fake success and no local fallback.
4. **Keep hook integrations dumb and discovery surfaces accurate**: keep HTTP logic out of Pi and Claude shims, document the remote config contract and local-vs-remote behavior in help/docs/manifest surfaces, and add tests covering mode selection, config precedence, successful remote submission, duplicate handling, rejection handling, and transport failure paths.

## Impact

- **CLI implementation**: add shared remote-config and remote-submit logic in `crates/scryrs-cli`, branch `scryrs record` between unchanged local persistence and explicit remote submission, and keep hook-facing transport logic inside Rust rather than JavaScript shims.
- **Configuration contract**: extend `scryrs.json` with optional remote defaults and honor environment overrides for ingest URL, repository/workspace/agent identity, and timeout.
- **User-visible behavior**: local mode remains the default and keeps the current SQLite contract; remote mode skips `.scryrs/scryrs.db`, reports deterministic remote counts, and fails loudly on transport or server errors.
- **Verification/docs**: update record/help/manifest/hook documentation surfaces and add regression plus remote-mode tests without adding retries, dual-write behavior, auth, or dashboard/live-query scope.
