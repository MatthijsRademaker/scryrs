## Context

The live hotspot server foundation already exists: `scryrs server` accepts remote batches, `scryrs record` and `scryrs hook` can switch to remote transport, the Pi shim is transport-only, and remote defaults already fit in `scryrs.json`. The missing product layer is setup. `scryrs init` still assumes local SQLite-first installation, remote config discovery still walks from process cwd, and the repository ships no Docker packaging for running the live server as a reusable network service.

## Goals / Non-Goals

**Goals**

- Add a deterministic, scriptable local-vs-live choice to `scryrs init` while preserving local mode as the default.
- Configure live mode by writing or merging the target project's remote ingest settings without moving transport into hook shims.
- Make hook-triggered remote config resolution use the target project root / event cwd rather than the harness process cwd.
- Keep live mode exclusive from local SQLite writes.
- Provide Docker packaging and documentation for running `scryrs server` containerized on a multi-agent Docker network.
- Update help, docs, and tests so the new setup flow is discoverable and regression-resistant.

**Non-Goals**

- Changing the inner `TraceEvent` schema or live-server wire contract except for already-existing remote config fields.
- Adding direct HTTP logic to Pi or Claude hook integrations.
- Adding dual-write, local fallback after remote failures, retry spooling, auth, TLS, hosted deployment, Kubernetes, or multi-tenant hardening.
- Replacing current local hotspot, dashboard, or live server APIs.

## Decisions

### Decision 1: `scryrs init` gets an explicit `--mode local|live` surface

The installer will add `--mode local|live` and keep `local` as the default so `scryrs init --agent <NAME>` remains the current zero-network setup path. Live mode uses deterministic flag-driven remote inputs matching the existing `remote` manifest shape: `ingest_url`, `workspace_id`, `agent_id`, and optional `repository_id`. Missing or invalid live inputs fail with exit code 2 before any install writes occur.

### Decision 2: Live mode writes only project-owned remote configuration

Live mode creates or merges only the `remote` section of the target project's `scryrs.json`, preserving unrelated manifest keys and leaving local mode unchanged. If `repository_id` is not supplied explicitly, init derives it from Git remote origin and normalizes it to the live-server repository identity contract; if no stable repository identity can be derived, init exits 2 before writing. This deliberately narrows the existing init-installer prohibition on `scryrs.json`: local mode stays manifest-agnostic, live mode is allowed to manage remote defaults.

### Decision 3: Mode-specific `.scryrs/` scaffolding preserves exclusivity

Local mode keeps the current `.scryrs/scryrs.db` plus `.scryrs/.gitignore` scaffolding. Live mode still creates `.scryrs/`, `.scryrs/.gitignore`, and `.scryrs/hooks/` so warning logs have a stable home, but it does not create `.scryrs/scryrs.db`. The existing source-repo Pi dogfooding allowance remains local-mode-only; live mode is a consumer-project setup path and should refuse to write remote consumer config into the scryrs source checkout.

### Decision 4: Hook-owned remote config resolution becomes event-rooted

`resolve_remote_config()` will accept an optional base path so hook execution can resolve `scryrs.json` from the event's target project root instead of `std::env::current_dir()`. `scryrs hook` passes the parsed event cwd/base directory into that resolver. The Pi shim stays transport-only and simply forwards `process.cwd()` in the raw event payload. `scryrs record` continues to use current-directory ancestry when no explicit base path is available.

### Decision 5: Live hook behavior stays CLI-owned and fail-open

Installed hooks remain transport-dumb. Pi still delegates to `scryrs hook pi --file <tmp>`, Claude still delegates to `scryrs hook claude-code`, and any remote submission failure stays in the Rust CLI path. Hook failures continue to log warnings and exit 0 without altering the harness-visible tool result, and live mode never falls back to local SQLite writes.

### Decision 6: Docker packaging is delivered as repository artifacts, not hosted infrastructure

The repository will add a Docker image definition and a compose example for `scryrs server`. The runtime defaults for the containerized service are `--bind 0.0.0.0 --port 8081 --store /data/scryrs/server.db`, with persistent storage and a stable service name reachable from other containers on the same Docker network. Automatic image publishing is out of scope.

### Decision 7: Discovery and verification change together

CLI help, help-json, README, and live-hotspot documentation must all describe the new live-init path and Docker workflow. Tests must cover unchanged local init behavior, live init validation and manifest writing, hook cwd propagation and remote config discovery, fail-open remote server failures, and the expected Docker artifact contracts.

## Risks

| Risk | Mitigation |
|------|------------|
| Live hooks silently resolve the wrong `scryrs.json` when the harness process cwd differs from the project root. | Thread an explicit base path into `resolve_remote_config()` and require Pi to forward `process.cwd()`. |
| Live-mode init writes partial or unstable remote identity and leaves a misleading setup behind. | Validate all required live inputs before any writes and refuse missing repository identity when it cannot be derived deterministically. |
| Live mode creates a useless local SQLite store and blurs the local-vs-live boundary. | Split `.scryrs/` scaffolding by mode so live mode creates warning-log directories only. |
| Source-repo Pi dogfooding could accidentally create consumer remote config in the scryrs checkout. | Keep the existing local-mode Pi allowance and refuse live mode in the source checkout. |
| Docker examples could bind only to localhost or lose data on restart. | Standardize the packaged runtime on `0.0.0.0:8081`, `/data/scryrs/server.db`, a persistent volume, and a named service on an attachable network. |

## Conflict Resolution

1. **Init vs `scryrs.json` prohibition**: the current init-installer rule forbidding `scryrs.json` access is preserved for local mode and explicitly narrowed for live mode only.
2. **Workspace identity strategy**: exploration left auto-generation open; this synthesis resolves the task to explicit live-mode CLI inputs rather than auto-generated workspace IDs because refinement required deterministic, scriptable setup and did not accept hidden ID generation.
3. **Repository identity strategy**: repository identity stays explicit-or-derived. Live init may accept an explicit `repository_id`, otherwise it uses the existing Git remote fallback and must normalize it to the live-server contract before writing.
4. **Source-repo behavior**: the accepted Pi source-repo dogfooding exception is preserved for local installs only; live mode is scoped to configuring consumer projects.

## Traceability

- Task: `795d99eb-c84e-46e4-8ee7-2e53efa11cde`
- Exploration dossier: `2026-06-27T09:55:13.850Z`
- Accepted decisions: `1-swarm-architect-recommendation`, `1-swarm-lead-dev-recommendation`, `1-swarm-reviewer-recommendation`
- Round outputs: `round:1:agent:swarm-architect`, `round:1:agent:swarm-lead-dev`, `round:1:agent:swarm-reviewer`
- Artifact base: `initial`