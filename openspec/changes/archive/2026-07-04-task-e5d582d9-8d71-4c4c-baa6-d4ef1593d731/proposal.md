## Why

The live hotspot server already has contract coverage and in-process Rust tests for ingest, idempotency, accumulators, query behavior, and SSE replay, but it still lacks a maintainer-facing proof that the shipped `scryrs` binary works end to end in the real multi-agent live workflow. This task closes that gap with a headless, receipt-backed verification path that starts a fresh server, drives remote `scryrs record` from two agent identities, and proves cumulative hotspot state, duplicate no-op behavior, and signal replay/resume through real HTTP APIs.

## What Changes

1. **Add a standalone Docker-backed verification entrypoint**: create `scripts/verify-live-hotspots` following the existing `scripts/verify-trace-capture` pattern. It should use `scripts/lib/docker-verification.sh` to build the real release `scryrs` binary, copy it into `.docker-fixtures/`, and run a Node-based fixture in a Debian/glibc `node:22` container so the verification stays headless and CI-compatible.
2. **Add a focused live-workflow fixture**: create `scripts/verification/live-hotspots-e2e.mjs` to allocate a nonzero port, start `scryrs server` with a fresh temp SQLite store, wait for readiness via the live query API, configure two remote producers through `SCRYRS_REMOTE_*` environment variables, submit overlapping JSONL through `scryrs record --file`, and assert cumulative multi-agent hotspot state.
3. **Verify duplicate idempotency and SSE cursor behavior through the shipped binary**: re-submit one agent's identical JSONL to prove duplicate acknowledgments do not change cumulative hotspot state, then connect to `GET /v1/repositories/{repository_id}/signals` with `after=0` and `after=<last_seen_id>` to prove persisted replay and resume behavior on the infinite SSE stream.
4. **Keep the scope narrow and deterministic**: use the server's existing default threshold of `10` by generating enough deterministic `EditMade` events to cross it, use environment-variable remote configuration instead of `scryrs init --mode live`, exclude live dashboard smoke, and keep the script as a standalone/nightly verification path rather than a PR-gate default.
5. **Document the verification path**: update `scripts/verification/README.md` with invocation, prerequisites, what the script proves, failure expectations, and the initial nightly-lane recommendation.

## Impact

- Maintainers gain a reproducible end-to-end proof for the real live workflow instead of relying only on in-process coverage.
- The verification exercises the actual server runtime, server-owned SQLite store, remote `record` transport, hotspot query API, and SSE cursor semantics together.
- The change stays within existing contracts: no wire-schema changes, no new threshold flag, no auth/TLS/deployment expansion, and no live dashboard product work.
- Documentation becomes the authoritative operator path for running and interpreting the live verification suite.