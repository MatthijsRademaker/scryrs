## Context

`scryrs init` today is a single command that does three separable things (per the `init-installer` spec):

1. **Install the harness hook** — writes the embedded, mode-independent transport shim (`hooks/pi/index.ts` → `.pi/extensions/scryrs/index.ts`, or the Claude Code settings merge). Needs no config.
2. **Resolve + write runtime config** — in live mode, validates `ingest_url`/`workspace_id`/`docker_network`, derives `repository_id`, and writes `scryrs.json` `remote` + the `.scryrs/` scaffold.
3. **Validate config now** — live mode (the default) exits 2 if required config is missing, *before* the hook is installed.

Because the default is live and step 3 runs first, the always-needed action (#1) is gated behind config that is really a runtime input. The runtime resolver already layers config at hook-execution time (`CLI > env > .scryrs/.env > scryrs.json`), so env-driven deployments don't need `scryrs.json` at all — yet `init` forces the question at install time. The observed workaround is `scryrs init --mode local` purely to extract the hook, which misleads readers into thinking recording is local when env makes it remote.

This change separates the hook primitive from config scaffolding into two commands: `scryrs init` (hook) and `scryrs setup <mode>` (config). It coordinates with two in-flight efforts: `add-init-interactive-wizard` (whose wizard must retarget to `setup live`) and the already-shipped `cli-doctor-command` (which already provides the fail-fast readiness probe, so no new probe is needed).

## Goals / Non-Goals

**Goals:**

- `scryrs init --agent <name>` installs only the hook: idempotent, config-free, cannot fail on missing ingest config.
- `scryrs setup <mode>` owns all config scaffolding (`scryrs.json` remote section, `.scryrs/` store/scaffold), with fail-fast validation where it belongs.
- Make `setup live` require only the irreducible config (`ingest_url`, `workspace_id`); treat compose/`docker_network` scaffolding as opt-in.
- Keep runtime config-resolution precedence and hook bytes unchanged.
- Provide a clean, intuitive DX for both human consumers (host) and env-driven containers (Swarm worker), with no mode trickery.
- Retarget the interactive wizard to `setup live`.

**Non-Goals:**

- Changing hook source, TraceEvent schema, record transport, or server endpoints.
- Changing runtime config-resolution precedence (`CLI > env > .scryrs/.env > scryrs.json`).
- Building a new readiness/fail-fast probe — `scryrs doctor` already covers it.
- Backwards compatibility for `scryrs init --mode ...` (this is an intentional breaking change; no alias/shim).
- Implementing the wizard UX itself (owned by `add-init-interactive-wizard`); this change only repositions its target command.

## Decisions

### Decision 1: Two commands — `init` (hook) and `setup <mode>` (config) — not a flag

**Chosen:** a dedicated `scryrs setup <mode>` command, with `mode` as a positional (`local` | `live`). `init` keeps only `--agent`.

**Why:** the two concerns have different failure semantics and different audiences. Hook install is universal and infallible-by-design; config is environment-specific and must fail loudly when wrong. A separate verb makes the contract self-evident ("init the hook, set up the transport") and lets each command have a coherent exit-code story. A mode flag on `init` is exactly today's overload.

**Alternatives considered:**

- Keep one `init` with `--mode` but make config-free the default. Rejected: still couples two concerns in one command and one help entry; the wizard and config validation have nowhere clean to live.
- `scryrs config <mode>` naming. Reasonable, but `setup` reads better next to `init` and matches the maintainer's proposed surface. Positional mode matches existing `--mode local|live` vocabulary.

### Decision 2: `init` becomes hook-only and never touches config

**Chosen:** remove `--mode` and all live/local config args from `init`. `init` installs the embedded hook, applies the existing project-local target rules, pre-existing-file refusal, Claude Code settings merge, and source-checkout self-install rules — and stops there. It never reads or writes `scryrs.json` or `.scryrs/`.

**Why:** this is the primitive every consumer needs and it should be impossible to make it fail on config. It also removes the source-checkout "live mode refused" special case from `init` (there is no live mode on `init` anymore); the equivalent guard moves to `setup live` (refuse live setup inside the scryrs source checkout).

**Alternatives considered:**

- Have `init` still scaffold a local store by default. Rejected: that's config scaffolding (= `setup local`), and re-introduces a side effect on the primitive.

### Decision 3: `setup live` requires only `ingest_url` + `workspace_id`; compose is opt-in

**Chosen:** core `setup live` validates and writes `remote.ingest_url` + `remote.workspace_id` to `scryrs.json` (deriving `repository_id` from git origin, leaving `agent_id` to runtime autogeneration — unchanged). The `.scryrs/compose.yml` + `docker_network` scaffolding for a self-hosted live server is gated behind an explicit opt-in (`setup live --with-compose`, the natural partner of `scryrs up`), not a hard requirement.

**Why:** `docker_network`/compose is only meaningful when scryrs is *bootstrapping* a self-hosted live server. Deployments that join an existing network (the Swarm worker) or point at an external server should not be forced to supply it. Today live init hard-requires `docker_network`, which is the most common reason live setup fails for valid deployments.

**Alternatives considered:**

- Keep `docker_network` required. Rejected: forces irrelevant input on the majority of live deployments and was a primary motivation for this change.
- Drop compose scaffolding entirely. Rejected: `scryrs up` self-hosting is a real, supported flow; keep it, just make it opt-in.

### Decision 4: Worker = `init` (hook) + env (runtime) + `doctor` (probe); no `setup`, no `scryrs.json`

**Chosen:** env-driven containers run `scryrs init --agent <name>` (build time) and supply ingest config via env at runtime; they do not run `setup` and ship no `scryrs.json`. Readiness/fail-fast is `scryrs doctor` at container start.

**Why:** deployment config (ingest URL, workspace id) belongs in the orchestration layer, not baked into a generic image or written into each cloned workspace. The runtime resolver already prefers env over `scryrs.json`, so env alone yields remote/live ingest; `repository_id` derives from the cloned repo's git origin and `agent_id` from hostname. `scryrs doctor` already resolves mode + live-server reachability and exits 2 on structural error, so it is the correct fail-fast surface — the hook itself must stay fail-open.

**Alternatives considered:**

- Worker runs `setup live` in the entrypoint to write `scryrs.json` from env. Rejected (per maintainer agreement): reintroduces a config file inside a generic container and is strictly more moving parts than env + `doctor`.

### Decision 5: Migrate the interactive wizard to `setup live`

**Chosen:** the TTY wizard from `add-init-interactive-wizard` (prompt-on-missing-live-config, `--no-interactive` opt-out, validation/confirmation) becomes part of `scryrs setup live`. `init` has no config to collect and therefore no prompting.

**Why:** the wizard's entire purpose is collecting missing live config — that is `setup`'s job after the split. Leaving it on `init` would re-couple the concerns this change separates.

**Sequencing:** `add-init-interactive-wizard` is **completed in code** (the wizard currently lives on `scryrs init`) but its OpenSpec change is **not yet archived**, so `init-interactive-wizard` is not a promotable base spec. This change therefore *migrates* the existing wizard implementation from `init.rs` into the `setup` module and strips all prompting from `init`. The retarget is enforced via the `setup-command` requirement "Interactive live-config collection belongs to `setup live`, not `init`" and a migration task (see Open Questions for the clean-archive alternative).

## Risks / Trade-offs

- **[Risk] Breaking change for existing `scryrs init --mode live ...` users/scripts** → Mitigation: this is intentional and called out as BREAKING; update README/CLI docs/examples and `scryrs doctor`/help guidance to point at the `init` + `setup live` two-step. No silent alias (no-backwards-compat policy).
- **[Risk] The just-completed `init` wizard becomes dead/duplicated when `init` goes hook-only** → Mitigation: this change migrates the wizard code from `init.rs` to the `setup` module and removes prompting from `init` as part of the same change; no wizard behavior is dropped, only relocated. The unarchived `add-init-interactive-wizard` spec is reconciled at its archive time (Open Questions).
- **[Risk] Spec churn — moving requirements between `init-installer`, `workspace-live-bootstrap`, and the new `setup-command`** → Mitigation: deltas explicitly REMOVE the relocated requirements from `init-installer` and ADD them under `setup-command`, so no requirement is silently dropped or duplicated.
- **[Risk] Making `docker_network` optional could let a self-hosting user forget compose setup** → Mitigation: `scryrs up` / `setup live --with-compose` fail loudly when compose/network is needed but unresolved; `scryrs doctor` reports the gap.

## Migration Plan

1. Add the `setup` command (`local` | `live`) and move the live-config + local-store scaffolding logic out of `init.rs` into a `setup` module; keep the runtime resolver and hook embedding untouched.
2. Slim `init` to hook-only: drop `--mode` and live args; remove the live-refused-in-source-checkout guard from `init` and add the equivalent to `setup live`.
3. Make `docker_network`/compose scaffolding opt-in in `setup live`.
4. Update dispatch, help, help-json, golden snapshots, and the CLI reference for both commands.
5. Coordinate the wizard retarget with `add-init-interactive-wizard`.
6. Update README/quickstart/examples to the two-step flow; ensure `scryrs doctor` and next-step text reference it.
7. Rollback: revert is contained to `scryrs-cli` (init/setup modules, dispatch/help/docs/tests); no schema, transport, or server changes to undo.

## Open Questions

- **Archive order vs `add-init-interactive-wizard`**: the wizard is already implemented on `init` but its change is unarchived. Cleanest path: archive `add-init-interactive-wizard` first (promoting `init-interactive-wizard` to `openspec/specs/`), then add a `init-interactive-wizard` MODIFIED delta here to formally retarget it to `setup live`. If it stays unarchived, the retarget is enforced via the `setup-command` requirement + the migration task, and the wizard spec is reconciled whenever it archives. Recommend archiving the wizard change first.
- **Compose opt-in surface**: `setup live --with-compose` flag vs. folding compose scaffolding entirely into `scryrs up`'s bootstrap. Either keeps core `setup live` config-only; pick during implementation.
- **`setup` without a prior `init`**: allowed (config and hook are independent); `scryrs doctor` validates the combination. Confirm no ordering requirement is asserted.
