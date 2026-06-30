## 1. Slim `init` to hook-install only

- [x] 1.1 Remove `--mode` and all live/local config arguments (`--ingest-url`, `--workspace-id`, `--docker-network`, `--repository-id`, `--agent-id`) from the `init` dispatch/arg parsing.
- [x] 1.2 Strip live-config resolution, validation, `scryrs.json` writing, and `.scryrs/` scaffolding from `init.rs`; `init` installs only the embedded harness hook.
- [x] 1.3 Remove the live-mode source-checkout refusal from `init` (no live mode on `init`); keep Pi hook dogfooding allowed and Claude Code consumer config refused in the source checkout.
- [x] 1.4 Replace `init` next-step text with hook-focused output that points to `scryrs setup <mode>` and reloading the agent (no remote URL / `scryrs up` guidance).
- [x] 1.5 Update `init` exit-code paths so missing config can no longer cause failure (only `--agent` validation, write errors, and pre-existing-file/self-install refusals remain).

## 2. Add the `setup <mode>` command

- [x] 2.1 Add a native `scryrs setup <mode>` command with `mode` as a required positional (`local` | `live`); unknown/missing mode exits 2 with deterministic guidance.
- [x] 2.2 Implement `setup local`: scaffold `.scryrs/scryrs.db` + `.scryrs/.gitignore`, idempotent, manifest-agnostic (never touches `scryrs.json`).
- [x] 2.3 Implement `setup live` core: require only `ingest_url` + `workspace_id` (precedence `CLI > env > .scryrs/.env > scryrs.json`), derive `repository_id` from git origin, leave `agent_id` to runtime; create-or-merge `scryrs.json` `remote` writing only the committed constants; fail-fast (exit 2, no partial writes) on missing/invalid/conflicting inputs.
- [x] 2.4 Make compose/self-host scaffolding an explicit opt-in (`setup live --with-compose`): only that path scaffolds `.scryrs/compose.yml` + overrides-only `.scryrs/.env` and requires `docker_network`.
- [x] 2.5 Refuse `setup live` inside the scryrs source checkout; keep `setup local` allowed there for dogfooding.

## 3. Migrate the interactive wizard from `init` to `setup live`

- [x] 3.1 Move the completed TTY live-config wizard implementation out of `init.rs` into the `setup` module so it runs under `setup live`.
- [x] 3.2 Ensure `init` never prompts (it collects no config); remove any wizard entry point from the `init` path.
- [x] 3.3 Preserve the wizard's non-interactive opt-out and deterministic exit-2 fail-fast behavior on `setup live` for non-TTY / automation contexts.

## 4. Update `scryrs up` and live-bootstrap wiring

- [x] 4.1 Repoint live-bootstrap triggering from `init --mode live` to `setup live`; ensure `scryrs up` consumes the `setup live --with-compose` scaffold.
- [x] 4.2 Make `scryrs up` fail loudly with guidance pointing at `setup live --with-compose` when the managed compose scaffold is absent.

## 5. CLI discovery, docs, and tests

- [x] 5.1 Update dispatch, `--help`, and `--help-json` so `init` shows `--agent` only (no `--mode`/live args) and `setup` is listed with its `local`/`live` modes and live inputs.
- [x] 5.2 Update golden/snapshot tests for the new `init` and `setup` help surfaces.
- [x] 5.3 Update README / quickstart / examples / CLI reference to the two-step `init` → `setup live` (→ `setup live --with-compose` + `scryrs up`) flow.
- [x] 5.4 Add/relocate unit and golden tests covering: hook-only `init` (no config side effects), `setup local`, `setup live` required-input validation and merge/conflict behavior, compose opt-in, source-checkout refusal, and wizard-on-`setup`.

## 6. Reconcile the wizard spec and verify

- [x] 6.1 If `add-init-interactive-wizard` is archived first, add a `init-interactive-wizard` MODIFIED delta retargeting the wizard to `setup live`; otherwise confirm the `setup-command` requirement + migration task fully cover the retarget.
- [x] 6.2 Run the full verification suite (build, tests, golden snapshots, `openspec validate`) and confirm `scryrs doctor` guidance reflects the `init` + `setup` split.
