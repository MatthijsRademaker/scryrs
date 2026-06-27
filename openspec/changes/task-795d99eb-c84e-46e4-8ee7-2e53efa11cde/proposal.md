## Why

`scryrs init` still behaves like a local-only installer even though remote ingest, `scryrs hook`, and `scryrs server` already exist. There is no explicit init-time choice between local and live setup, no safe path for writing a target project's remote ingest configuration, no guarantee that installed hooks resolve live configuration from the target project root, and no Docker runtime artifact for running `scryrs server` as a shared service for multiple agent containers.

## What Changes

1. **Add an explicit init mode choice**: extend `scryrs init` with `--mode local|live`, keeping `local` as the default so `scryrs init --agent <NAME>` preserves current local behavior.
2. **Add live-mode project configuration**: in `--mode live`, install the same harness hook transport, validate deterministic remote identity inputs, and create-or-merge the target project's `scryrs.json` `remote` section instead of scaffolding a local trace database.
3. **Preserve live-mode exclusivity**: live-mode init keeps `.scryrs/` warning-log scaffolding but does not create `.scryrs/scryrs.db`, does not dual-write, and does not add hook-side HTTP logic.
4. **Fix hook project-root resolution for live mode**: thread an explicit base path into remote config resolution, use the event `cwd` on the hook path, and extend the Pi shim only enough to forward `process.cwd()`.
5. **Add container packaging for the live server**: provide a Docker image definition and compose example for `scryrs server --bind 0.0.0.0 --port 8081 --store /data/scryrs/server.db`, with persistent storage and a stable multi-agent network endpoint.
6. **Update discovery, docs, and tests**: refresh CLI help/help-json, README, live-hotspot docs, and regression coverage for local init, live init validation, hook fail-open behavior, cwd-aware remote config resolution, and Docker artifacts.

## Impact

- **CLI surface**: `crates/scryrs-cli` gains an explicit init mode plus live-mode validation and next-step guidance.
- **Manifest ownership**: `scryrs init` becomes allowed to create or merge `scryrs.json` only for live mode; local mode remains manifest-agnostic.
- **Hook path correctness**: `crates/scryrs-cli/src/remote_config.rs`, `crates/scryrs-cli/src/hook.rs`, and `hooks/pi/index.ts` align remote config discovery with the target project root while preserving fail-open behavior and CLI-owned transport.
- **Deployment packaging**: the repository gains Docker runtime artifacts and documented multi-agent startup flow for the existing `scryrs server` surface.
- **Contract boundaries preserved**: no inner `TraceEvent` schema changes, no new hook-side HTTP client logic, no local/remote dual-write, and no auth/TLS/hosted deployment scope.