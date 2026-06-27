## 1. Add explicit init mode and live validation

- [x] 1.1 Extend `scryrs init` CLI parsing, help text, and help-json to expose `--mode local|live` while preserving `local` as the default for `scryrs init --agent <NAME>`.
- [x] 1.2 Add deterministic live-mode inputs for remote configuration, validate them before any writes, and return exit code 2 with deterministic diagnostics on missing or invalid live config.
- [x] 1.3 Keep unsupported harness handling, local-mode collisions, and the current local init output contract unchanged.

## 2. Implement mode-specific install and manifest behavior

- [x] 2.1 Split `.scryrs/` scaffolding by mode so local mode still initializes `.scryrs/scryrs.db` while live mode creates `.scryrs/`, `.scryrs/.gitignore`, and `.scryrs/hooks/` only.
- [x] 2.2 Implement live-mode `scryrs.json` create-or-merge behavior for the `remote` section, preserving unrelated manifest keys and refusing partial writes.
- [x] 2.3 Preserve the existing local-mode Pi source-repo dogfooding path and refuse live-mode init inside the scryrs source checkout.
- [x] 2.4 Update mode-specific next-step text so local installs keep current guidance and live installs describe server startup/connection and Docker-based workflow.

## 3. Fix hook project-root resolution for live mode

- [x] 3.1 Change `remote_config::resolve_remote_config()` to accept an optional base path and use it for ancestor manifest discovery on the hook path.
- [x] 3.2 Pass the parsed event cwd/base directory from `scryrs hook` into remote config resolution while keeping record-path behavior intact.
- [x] 3.3 Extend `hooks/pi/index.ts` only enough to forward `process.cwd()` with each emitted event.
- [x] 3.4 Preserve fail-open behavior and ensure remote hook failures do not create local fallback writes.

## 4. Add Docker runtime artifacts for the live server

- [x] 4.1 Add a Docker image definition that runs `scryrs server --bind 0.0.0.0 --port 8081 --store /data/scryrs/server.db`.
- [x] 4.2 Add a compose example with persistent SQLite storage, a stable `scryrs-server` service name, and an attachable network for multi-agent container use.
- [x] 4.3 Document how agent workspaces on the same Docker network point live-mode init at the containerized server endpoint.

## 5. Update docs and discovery surfaces

- [x] 5.1 Update `README.md`, CLI docs, and live-hotspot docs so they describe local-vs-live init, live-mode exclusivity, and the Docker workflow consistently.
- [x] 5.2 Keep trace-hook documentation aligned with the no-direct-HTTP rule and the event-cwd-based remote config resolution behavior.

## 6. Add regression coverage

- [x] 6.1 Extend init tests to prove default local behavior is unchanged and live mode validates config before writing.
- [x] 6.2 Add tests for live-mode `scryrs.json` writing/merging, skipped `.scryrs/scryrs.db` scaffolding, and source-repo refusal behavior.
- [x] 6.3 Add hook tests covering event-cwd-based manifest discovery, Pi cwd forwarding, and fail-open remote server failures.
- [x] 6.4 Add Docker artifact smoke or contract checks covering bind/store defaults, persistent volume usage, and expected service/network names where feasible.
