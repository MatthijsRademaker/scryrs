## Context

The current live-mode operator story is split across the wrong boundary. User-facing docs still tell consumers to check out the scryrs source repository and run the repository root `docker-compose.yml`, while `scryrs init` only handles hook installation plus remote-ingest identity. That leaves consumer workspaces without a self-contained runtime scaffold and bakes in a scryrs-owned network model (`scryrs-net`) that is awkward for real agent deployments.

This change makes the consumer project the unit of live bootstrap. The consumer workspace should own the generated `.scryrs/.env` and `.scryrs/compose.yml`, while the scryrs repository root Docker artifacts remain packaging and maintainer-oriented assets. The live server should join an existing external agent network and be reachable there as `http://scryrs:8081`. This keeps `scryrs up` infra-only and preserves the existing all-or-nothing init stance.

Constraints:

- `scryrs init` live mode must remain fail-fast: no partial writes when required inputs are missing or conflicting.
- Multi-harness installation in one workspace must reuse shared live bootstrap state instead of rewriting it.
- Pi runtime install behavior must become idempotent for identical content without silently overwriting divergent content.
- Docs must explicitly distinguish consumer bootstrap artifacts from repository packaging/dev artifacts.
- Scope stops at local/team-managed Docker Compose. No hosted deployment, no cluster manager, no `scryrs down` in this change.

## Goals / Non-Goals

**Goals:**

- Make live bootstrap self-contained in the consumer workspace under `.scryrs/`.
- Add `scryrs up` as a thin launcher for workspace-managed Compose artifacts.
- Replace the dedicated scryrs-network assumption with explicit attachment to an existing external agent network.
- Preserve deterministic, all-or-nothing live init validation.
- Make repeated live init across supported harnesses idempotent for shared infra files and Pi runtime install.
- Update docs and help text so the new contract is explicit everywhere users encounter setup guidance.

**Non-Goals:**

- No automatic network creation or multi-network orchestration.
- No health-management daemon, process supervisor, or richer lifecycle surface (`down`, `restart`, etc.).
- No change to server ingest protocol, hotspot accumulation, or hook business logic.
- No attempt to make one `.scryrs/.env` simultaneously optimal for both host-native and container-native live URLs.
- No removal of repository root Docker artifacts; they remain valid for packaging/dev-maintainer use.

## Decisions

### 1. Consumer bootstrap moves into `.scryrs/`, but root Docker artifacts stay

`scryrs init` in live mode will generate consumer-managed `.scryrs/.env` and `.scryrs/compose.yml`. The repository root `Dockerfile` and `docker-compose.yml` remain in place as packaging and maintainer-facing assets.

**Why:** One file set cannot cleanly serve both roles. Consumer bootstrap needs workspace-local, idempotent, generated artifacts. Repository root artifacts need to remain canonical packaging/dev assets for building and smoke-checking the image.

**Alternative rejected:** Reusing the repository root `docker-compose.yml` as the consumer truth. That keeps the current broken checkout dependency and mixes maintainer concerns with consumer runtime state.

### 2. `scryrs up` is a thin wrapper around workspace-local Docker Compose

`scryrs up` will locate the generated `.scryrs/compose.yml` and `.scryrs/.env`, validate that the scaffold exists, optionally preflight the configured external network, then run the equivalent of `docker compose up -d` against those files. It will not install hooks, rewrite config, or infer missing identity.

**Why:** The command should stay explainable and debuggable. Users can reason about it as a thin convenience wrapper over Docker Compose instead of a second hidden runtime layer.

**Alternative rejected:** Making `scryrs up` a broader environment manager that also performs init, network creation, hook repair, or process supervision. That expands scope and blurs responsibilities.

### 3. External network attachment becomes explicit live bootstrap input

Live bootstrap needs one more required input: the existing Docker network name that agent containers already use. The scaffolded Compose file will reference that network as an external network and attach the server service to it.

Configuration source order should mirror the existing live-config pattern where it makes sense:

- CLI flag `--docker-network <NAME>`
- process environment
- `.scryrs/.env`

It should not be written into `scryrs.json remote`, which remains transport identity, not container-orchestration metadata.

**Why:** The network name is required to generate a valid consumer compose file, and it is operational bootstrap state rather than trace-ingest identity.

**Alternative rejected:** Auto-discovering a network from running containers or creating a new default network automatically. Both approaches are brittle and hide important topology decisions.

### 4. Container reachability contract is `http://scryrs:8081`

The generated compose service will be named `scryrs` and/or attached with explicit alias `scryrs` on the external network. The generated live ingest URL in `.scryrs/.env` will therefore be `http://scryrs:8081` for the container-first path. Host port publishing can remain for debugging and manual verification, but it is not the primary ingest contract.

**Why:** The user wants scryrs to join the agent network rather than forcing agent containers onto a scryrs-owned network. A stable in-network name keeps the contract simple for containerized harnesses.

**Alternative rejected:** `http://127.0.0.1:8081` as the primary default. That only works cleanly for host-native agents and breaks the container-first topology.

### 5. Managed infra files are idempotent by semantic equivalence, not overwrite

For `.scryrs/.env`, `.scryrs/compose.yml`, and `scryrs.json remote`, re-running live init in the same workspace should succeed when values match the existing managed bootstrap and fail loudly on incompatible managed-value conflicts. For Pi's installed runtime copy, identical content should be accepted as a no-op and divergent content should still fail.

**Why:** The workspace now carries shared infra state across multiple harness installs. Collision-only behavior is too blunt, while blind overwrite is dangerous.

**Alternative rejected:** Always overwriting managed files. That would erase operator edits and make second-harness init unsafe.

### 6. Docs must teach two artifact classes explicitly

Docs will treat these as separate concepts:

- **Consumer bootstrap artifacts**: generated under `.scryrs/` by `scryrs init`, used by `scryrs up`.
- **Repository packaging/dev artifacts**: root `Dockerfile` and `docker-compose.yml`, used by maintainers and release/dev workflows.

This distinction belongs in `live-server-setup.md`, `cli-v0-contract.md`, and help text.

**Why:** Without this, the old mental model persists and users will keep assuming that checking out the scryrs repo is part of normal live setup.

**Alternative rejected:** Only updating command behavior and hoping the docs become self-evident. They will not.

## Risks / Trade-offs

- [Risk] `.scryrs/.env` now favors container-first reachability (`http://scryrs:8081`), which may be wrong for host-native commands run against the same repo. → Mitigation: document this explicitly and keep the URL override path intact via flags/environment.
- [Risk] External network name is a new required operator input and may be unstable in ad hoc Compose setups. → Mitigation: fail fast, document the need for a stable named external network, and avoid hidden auto-discovery.
- [Risk] Keeping both root Docker artifacts and generated workspace artifacts may confuse maintainers. → Mitigation: make the artifact-class distinction explicit in docs, tests, and next-step text.
- [Risk] Semantic-idempotency checks for compose/env files can become too permissive or too strict. → Mitigation: compare managed fields deterministically, refuse conflicts loudly, and test second-harness re-runs.
- [Risk] `scryrs up` introduces a Docker dependency path that can fail in several ways (Compose missing, network missing, daemon unavailable). → Mitigation: keep failures deterministic and scoped to clear operator remediation.

## Migration Plan

1. Extend live init validation to include external network input before any file writes.
2. Add managed `.scryrs/compose.yml` generation and merge/preserve behavior for `.scryrs/.env` and `scryrs.json remote`.
3. Add `scryrs up` dispatch/help/help-json surface with deterministic validation and Compose invocation.
4. Change Pi install behavior from collision-only to content-aware no-op/refusal.
5. Update docs and examples to use workspace-local bootstrap (`scryrs init` → `scryrs up`) and to explain the external-network alias contract.
6. Add or update tests for init idempotency, compose scaffold content, `scryrs up` usage/errors, and docs/smoke expectations.

**Rollback:** Remove the `scryrs up` command and stop generating `.scryrs/compose.yml`, while leaving existing live-mode hook transport and server packaging intact. Because the new scaffold is workspace-local, rollback does not require data migration beyond ignoring or deleting generated files.

## Open Questions

- Exact CLI/env naming for the external network input: `--docker-network` / `SCRYRS_DOCKER_NETWORK` is the leading candidate, but the final name should be confirmed before implementation.
- Whether `scryrs up` should proactively check Docker network existence itself or rely on Docker Compose failure output with wrapped guidance.
- Whether the scaffolded compose file should also publish `8081:8081` by default for host-side debugging, or make that optional in favor of a pure in-network contract.
