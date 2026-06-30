## 1. Config resolution foundation (additive, no default change)

- [x] 1.1 Add a `.scryrs/.env` dotenv loader in `remote_config.rs` that parses `KEY=value`, ignores blanks and `#` comments, and recognizes the five `SCRYRS_REMOTE_*` keys; treat a missing file as empty.
- [x] 1.2 Implement the shared resolution precedence (CLI flags > process env > `.scryrs/.env` > `scryrs.json` `remote`) with `repository_id` Git-origin fallback and `timeout_ms` default `3000`, as a single reusable resolver.
- [x] 1.3 Add a fail-fast helper that, given an incomplete remote config, returns deterministic stderr guidance naming missing field(s) and both remediation paths (populate `.scryrs/.env`, use `--mode local`).
- [x] 1.4 Unit-test the loader and precedence resolver (each layer override, comment/blank handling, missing-file tolerance, repository-id fallback) in `remote_config.rs` tests.

## 2. Flip `scryrs init` default to live

- [x] 2.1 Change the `--mode` default to `live` in `dispatch.rs` arg wiring and update validation to no longer hard-require live flags when resolution succeeds.
- [x] 2.2 Route live-mode identity through the shared resolver; on unresolved required config, exit `2` with guidance before any write.
- [x] 2.3 Scaffold a create-or-merge `.scryrs/.env` template (with `SCRYRS_REMOTE_*` keys + comments) without clobbering existing values; ensure `.scryrs/.gitignore` covers `.env`.
- [x] 2.4 Update source-checkout refusal so a bare (live-default) `init` inside the scryrs repo emits the live refusal plus `--mode local` guidance.
- [x] 2.5 Update `init_tests.rs`: default-is-live, `--mode local` opt-in, guided failure with no partial writes, env-template scaffold, source-checkout refusal.

## 3. Flip `scryrs record` default to remote

- [x] 3.1 Add `--mode local|live` handling to `record` dispatch with remote as default; select local SQLite only on explicit `--mode local`.
- [x] 3.2 Resolve remote config via the shared resolver; remove silent local fallback — fail fast (`2`) with guidance when required config is unresolved and local mode is not selected.
- [x] 3.3 Preserve existing remote submission, producer-event-id derivation, summary shape, and exit-code semantics.
- [x] 3.4 Update `record_tests.rs`: default-is-remote, explicit-local SQLite path, fail-fast guidance, env/manifest precedence, nearest-ancestor manifest discovery.

## 4. Flip `scryrs dashboard` default to live

- [x] 4.1 Add `--mode local` opt-in; make live the default in `dashboard.rs`, resolving `--server-url`/`--repository-id` via the shared resolver (mapping `ingest_url`/`repository_id`).
- [x] 4.2 On live default with unresolved server URL or repository identity, fail startup with a clear configuration error naming the missing values and remediation.
- [x] 4.3 Keep live read-only with no local-artifact merge; preserve `/api/meta` mode reporting.
- [x] 4.4 Update dashboard dispatch/integration tests for the flipped default and explicit local opt-in.

## 5. Discovery surfaces and diagnostics

- [x] 5.1 Update `help_text.rs` so `init`/`record`/`dashboard` help present live as the default and `--mode local` as the opt-in, and document `.scryrs/.env`.
- [x] 5.2 Update `help_json.rs` machine-readable surface (default mode, `.scryrs/.env`) and adjust `dispatch_tests.rs` help/help-json assertions.
- [x] 5.3 Add a `doctor.rs` diagnostic detecting "live default but `.scryrs/.env` / remote config incomplete" with remediation text.

## 6. Documentation

- [x] 6.1 Reframe `.devagent/docs/docs/live-server-setup.md` around the live default; document `.scryrs/.env` keys, precedence, and `--mode local` opt-in.
- [x] 6.2 Update `live-hotspots.md` getting-started and `cli-v0-contract.md` mode/precedence/exit-code sections.
- [x] 6.3 Update root `README.md` quickstart to lead with the live default and the `--mode local` escape hatch; add release-note text for the breaking default change.

## 7. Verification

- [x] 7.1 Run `scripts/precommit-run` (fmt, check, clippy, test) and fix findings. — `scripts/check` and `scripts/test` both exit 0.
- [x] 7.2 Run `scripts/verify-live-hotspots` and confirm the end-to-end loop passes under the new defaults. — PASSED (38/38 checks).
- [x] 7.3 Run `openspec validate default-live-mode` and confirm the change is valid.
