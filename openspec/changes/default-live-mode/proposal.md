## Why

scryrs is built for multi-agent fleets, but the CLI defaults to single-machine local mode: `init`, `record`, and `dashboard` all assume an isolated `.scryrs/scryrs.db` unless the operator explicitly opts into live mode with a cluster of flags. Teams running several agents must remember `--mode live` plus `--ingest-url`, `--workspace-id`, and `--agent-id` on every workspace, which is easy to get wrong and silently falls back to disconnected local stores. Making the live server config the default — resolved from a checked-in-style `.scryrs/.env` — aligns the out-of-box behavior with the product's primary use case and removes per-invocation flag ceremony.

## What Changes

- **BREAKING:** `scryrs init` defaults to **live mode**. With no `--mode` flag, init configures the workspace for remote ingest instead of scaffolding a local SQLite store.
- **BREAKING:** `scryrs record` defaults to **remote transport**. Remote ingest is the default transport; local SQLite persistence becomes the opt-in path.
- **BREAKING:** `scryrs dashboard` defaults to **live mode**, proxying a configured live server instead of reading local `.scryrs` artifacts.
- Live-mode identity is resolved from environment variables (`SCRYRS_REMOTE_INGEST_URL`, `SCRYRS_REPOSITORY_ID`, `SCRYRS_WORKSPACE_ID`, `SCRYRS_AGENT_ID`, `SCRYRS_REMOTE_TIMEOUT_MS`), loaded from a new **`.scryrs/.env`** file in the project root. When required values are absent, commands **fail fast with actionable guidance** (how to populate `.scryrs/.env` or pass `--mode local`) rather than silently degrading.
- `--mode local` (and the dashboard's local path) remains a fully supported, explicit opt-in for single-developer, zero-network use. Local behavior is unchanged when selected.
- `--ingest-url`, `--workspace-id`, `--agent-id`, and `--repository-id` flags continue to work and take precedence over `.scryrs/.env` values; `--repository-id` still derives from the Git remote origin when omitted.
- Help text, `--help-json`, and the docs (`live-server-setup.md`, `live-hotspots.md`, README quickstart) are updated so live mode is presented as the default and local mode as the opt-in.

## Capabilities

### New Capabilities
- `scryrs-env-config`: Loading remote-ingest configuration from a `.scryrs/.env` dotenv file, including precedence (CLI flags > process env > `.scryrs/.env` > `scryrs.json` `remote`), parse rules, and absent-config error guidance.

### Modified Capabilities
- `init-installer`: Default `--mode` flips from `local` to `live`; live mode no longer hard-requires the remote flags when `.scryrs/.env` supplies them; absent-config failure guidance; source-checkout refusal of live mode is reconciled with the new default.
- `scryrs-record-endpoint`: Default transport flips from local SQLite to remote; local persistence becomes the explicit opt-in; mode-resolution and exit-code semantics updated for the new default.
- `live-dashboard-mode`: Live mode becomes the default; `--server-url`/`--repository-id` may be supplied via `.scryrs/.env`; local dashboard becomes the explicit opt-in.
- `scryrs-manifest`: Remote-config resolution order updated to include `.scryrs/.env`, and the `remote` object's role relative to the new default and env file is clarified.

## Impact

- **Code:** `crates/scryrs-cli/src/init.rs`, `record.rs`, `dashboard.rs`, `remote_config.rs`, `dispatch.rs` (arg defaults/validation), `help_text.rs`, `help_json.rs`, and `doctor.rs` (diagnostics for the new default and `.scryrs/.env`).
- **Config / filesystem:** new `.scryrs/.env` read path; `init` may scaffold a `.scryrs/.env` template. `.scryrs/.env` should be git-ignored guidance considered.
- **Docs:** `.devagent/docs/docs/live-server-setup.md`, `live-hotspots.md`, `cli-v0-contract.md`, and root `README.md` quickstart reframed around the live default.
- **Behavior / migration:** existing users relying on the implicit local default must add `--mode local` or populate `.scryrs/.env`; this is a breaking workflow change requiring clear release notes.
- **Tests:** `dispatch_tests.rs`, `init_tests.rs`, `record_tests.rs`, and live/local mode fixtures updated for the flipped default.
