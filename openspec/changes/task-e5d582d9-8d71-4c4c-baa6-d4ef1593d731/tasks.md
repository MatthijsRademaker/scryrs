## 1. Add the Docker-backed verification entrypoint

- [ ] 1.1 Create `scripts/verify-live-hotspots` as the authoritative live-workflow verification entrypoint using `scripts/lib/docker-verification.sh`.
- [ ] 1.2 Build the release `scryrs` binary in Docker, copy it to `.docker-fixtures/scryrs`, and run the fixture in a Debian/glibc `node:22` container.
- [ ] 1.3 Provide a clear summary and non-zero exit behavior when the fixture fails.

## 2. Implement the live-hotspots end-to-end fixture

- [ ] 2.1 Add `scripts/verification/live-hotspots-e2e.mjs` to allocate a nonzero port, start `scryrs server` with a fresh temp store, wait for readiness via the hotspot query API, and clean up the server process on every exit path.
- [ ] 2.2 Configure two explicit remote producer identities through `SCRYRS_REMOTE_*` environment variables and submit deterministic overlapping JSONL through `scryrs record --file`.
- [ ] 2.3 Assert cumulative hotspot state for the overlapping subject, including shared evidence and multi-session/multi-agent contribution, after the initial submissions.
- [ ] 2.4 Re-submit one agent's identical JSONL and assert duplicate acknowledgments plus unchanged cumulative hotspot state.
- [ ] 2.5 Implement the two-phase SSE verification (`after=0`, then `after=<last_seen_id>`) with manual `text/event-stream` parsing, explicit disconnects, and timeouts.
- [ ] 2.6 Fail loudly on startup errors, transport failures, malformed JSON, assertion failures, or SSE timeouts.

## 3. Document the verification path

- [ ] 3.1 Update `scripts/verification/README.md` with the new entrypoint, prerequisites, and direct invocation examples.
- [ ] 3.2 Document what the suite proves: multi-agent cumulative ingest, duplicate idempotency, SSE replay/resume, and loud failure behavior.
- [ ] 3.3 Record that live dashboard smoke is out of scope and that the initial CI posture is standalone/nightly rather than PR-gate.