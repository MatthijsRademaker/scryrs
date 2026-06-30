## Why

Running live mode still assumes consumers check out the scryrs source repository and start the root `docker-compose.yml` from there. That is wrong abstraction: consumer workspaces should be self-contained, and the live-server bootstrap should live under the project's own `.scryrs/` directory. At the same time, the current networking model forces agent containers onto a scryrs-owned network, when the better topology is the reverse: the scryrs server should join an existing agent network and be reachable there as `http://scryrs:8081`.

## What Changes

- Add a workspace-local live bootstrap path under `.scryrs/` so `scryrs init --agent <name>` scaffolds a managed `.scryrs/compose.yml` and `.scryrs/.env` for consumer projects in live mode.
- Add `scryrs up` as a thin orchestration command that starts the workspace-local Compose stack from `.scryrs/compose.yml` and `.scryrs/.env`; it does not install hooks, invent config, or manage agents.
- Require explicit container-network configuration for the scaffolded live path so the generated Compose service joins an existing external Docker network and is reachable there as `http://scryrs:8081`.
- Keep live init fail-fast and all-or-nothing: missing required live inputs, invalid network config, or conflicting managed values produce exit 2 before partial writes.
- Make live init idempotent across multi-harness setup in one workspace: a second `scryrs init --agent ...` reuses the same managed infra files instead of overwriting them, while still installing the requested harness.
- Tighten Pi installer behavior from collision-only refusal to content-aware idempotency: identical installed content is a no-op; divergent content still fails loudly.
- Reframe docs and operator guidance so user-facing setup points to workspace-local bootstrap, explicitly distinguishes consumer scaffold from repo-root packaging/dev artifacts, and documents the external-network alias model (`scryrs` on existing agent network).

## Capabilities

### New Capabilities

- `workspace-live-bootstrap`: Workspace-local live-server scaffold and `scryrs up` orchestration for consumer projects, including managed `.scryrs/compose.yml`, external-network attachment, and deterministic bootstrap behavior.

### Modified Capabilities

- `init-installer`: Live-mode init requirements change to scaffold and preserve managed workspace infra files, validate external network inputs, and support content-idempotent re-runs across harness installs.
- `live-hotspot-server-packaging`: Packaging/docs requirements change from repo-root compose plus scryrs-owned network assumptions to consumer-scaffolded compose that joins an existing external agent network and exposes the `scryrs` endpoint contract.
- `cli-discovery-ux`: Help and usage discovery requirements change to document the new `scryrs up` command and the managed live bootstrap workflow.
- `cli-docs-update`: CLI reference and related setup docs must describe the workspace-local bootstrap flow, external-network assumptions, and the distinction between consumer runtime scaffold and repository packaging artifacts.

## Impact

- CLI command surface and dispatch: new `scryrs up` command, help text, help-json, usage validation, and next-step guidance.
- Live init behavior in `crates/scryrs-cli/src/init.rs` and related tests.
- New workspace-managed runtime artifacts under `.scryrs/`, especially `.scryrs/compose.yml` plus additional `.env` keys for Docker network selection.
- Docker/Compose examples and smoke checks, including the relationship between root packaging artifacts and consumer-scaffolded artifacts.
- User-facing docs: `live-server-setup.md`, `cli-v0-contract.md`, and adjacent live-mode/setup pages.
