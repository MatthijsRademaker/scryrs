## Context

scryrs ships three operator-facing commands whose defaults assume single-machine, offline operation: `init` defaults to `--mode local`, `record` stays in local SQLite transport unless a remote ingest URL is resolved, and `dashboard` runs against local `.scryrs` artifacts unless both `--server-url` and `--repository-id` are passed. The product's primary use case is multi-agent fleets sharing one live server, so the defaults are inverted relative to intent.

Today, live mode requires the operator to supply `--ingest-url`, `--workspace-id`, and `--agent-id` on every invocation (or pre-populate `scryrs.json`'s `remote` section). Remote config already resolves through a precedence chain (`scryrs.json remote` < `SCRYRS_REMOTE_*` env). The proposal flips the default to live and resolves identity primarily from a new `.scryrs/.env` dotenv file, failing fast with guidance when required values are absent. Local mode survives as an explicit `--mode local` opt-in.

Constraints: this is a deliberately breaking change; the existing remote-config resolution and exit-code contracts (0/1/2) must be preserved in shape; live mode is still refused inside the scryrs source checkout; hook artifacts must not gain direct HTTP logic.

## Goals / Non-Goals

**Goals:**
- Make live mode the zero-flag default for `init`, `record`, and `dashboard`.
- Introduce `.scryrs/.env` as the canonical, project-local source of remote-ingest configuration.
- Define a single, deterministic precedence order for resolving remote config across all three commands.
- Fail fast with actionable guidance when the default is live but required config is absent.
- Keep `--mode local` (and dashboard local mode) a first-class, unchanged opt-in.

**Non-Goals:**
- Auto-generating identity values (no synthesized workspace/agent IDs) — absence is an error, per the chosen env-first approach.
- Auto-starting or managing a `scryrs server` process from the CLI.
- Changing the server-side ingest, accumulator, or signal contracts.
- Changing the `TraceEvent` wire schema or hook transport mechanism.
- Removing local mode.

## Decisions

### Decision 1: `.scryrs/.env` is a dotenv file loaded by the CLI, not a new manifest section
Remote config moves into `.scryrs/.env` holding `SCRYRS_REMOTE_*` keys in standard `KEY=value` dotenv form. The CLI loads it into the resolution chain below process env.

- **Why:** The user explicitly wants config under `.scryrs/`. A dotenv file is the lowest-friction shape that reuses the existing `SCRYRS_REMOTE_*` variable names already honored by `record`, so no new key vocabulary is introduced. It is gitignorable (machine/agent-specific identity) without touching the committed `scryrs.json`.
- **Alternatives considered:** (a) A new JSON block in `scryrs.json` — rejected because identity (agent/workspace) is per-machine and shouldn't be committed, and the user asked for `.env`. (b) CLI flags only — rejected; defeats the zero-flag goal.

### Decision 2: One shared resolution precedence, highest wins
`CLI flags > process environment > .scryrs/.env > scryrs.json remote`. `repository_id` falls back to the normalized Git remote-origin contract when unresolved by any layer. `timeout_ms` defaults to `3000`.

- **Why:** A single chain used by all three commands keeps behavior predictable and testable. Process env above `.scryrs/.env` lets CI/containers override the file without editing it; flags stay the ultimate override for one-off invocations.
- **Alternatives considered:** Putting `.scryrs/.env` above process env — rejected; container/CI overrides are the common operational escape hatch and must win over a checked-out file.

### Decision 3: Default flips to live; absence of config is a fast, guided failure
With no `--mode`, commands resolve in live/remote mode. If the ingest URL or a required identity field cannot be resolved from any layer, the command exits `2` before any network call or file write, printing guidance: how to populate `.scryrs/.env`, or how to pass `--mode local`.

- **Why:** Silent fallback to local (today's behavior for `record`) is exactly the failure mode the change targets — it hides disconnected agents. Fail-fast with remediation text matches the product intent and the chosen env-first answer.
- **Alternatives considered:** Falling back to local on missing config — rejected; reintroduces the silent-island problem.

### Decision 4: `init` may scaffold a `.scryrs/.env` template
Live-mode `init` writes (create-or-merge, never clobbering existing values) a `.scryrs/.env` template with the `SCRYRS_REMOTE_*` keys and inline comments, and ensures `.scryrs/.gitignore` covers `.env`.

- **Why:** Makes the default path self-documenting and gives operators an obvious place to fill in identity. Gitignoring prevents committing per-agent identity.
- **Alternatives considered:** No scaffold (pure error guidance) — rejected as higher-friction for the new default; create-but-clobber — rejected to preserve operator edits on re-run.

### Decision 5: Local mode stays selectable and behaviorally unchanged
`--mode local` for `init`/`record` and the no-live-flags path being explicitly requestable for `dashboard` preserve the exact current local behavior (`.scryrs/scryrs.db`, local artifact reads).

- **Why:** Single-developer offline use is still valid; flipping the default must not delete the capability.

## Risks / Trade-offs

- **Breaking change to muscle memory / scripts** → Mitigate with explicit release notes, a `--mode local` escape hatch, and `doctor` diagnostics that detect "live default but no `.scryrs/.env`" and print remediation.
- **Operators surprised by fail-fast where `record` used to silently run local** → Mitigate with a precise, deterministic stderr message naming the missing field(s) and both remediation paths.
- **Secret/identity leakage if `.scryrs/.env` is committed** → Mitigate by having `init` ensure `.scryrs/.gitignore` ignores `.env`; document it.
- **Dashboard default flip may break local-only users on launch** → Mitigate by clear startup error when live config is absent, naming how to revert to local mode.
- **Source-checkout refusal interaction** → live is refused in the scryrs source repo; with live now the default, bare `init` inside the source checkout must emit the existing refusal plus guidance to use `--mode local` (for Pi) — not a generic failure.

## Migration Plan

1. Land config-resolution changes (`.scryrs/.env` loader + shared precedence) behind the existing `remote_config` resolution path first, with no default change — pure addition.
2. Flip defaults for `init`, `record`, `dashboard`; update arg defaults/validation in `dispatch.rs`.
3. Update help text, `--help-json`, and `doctor` diagnostics.
4. Update docs (`live-server-setup.md`, `live-hotspots.md`, `cli-v0-contract.md`, README quickstart) to present live as default, local as opt-in.
5. Release notes call out the breaking default and the `--mode local` / `.scryrs/.env` remediation.

**Rollback:** Revert the default-flip commit (step 2); the `.scryrs/.env` loader (step 1) is additive and safe to keep.

## Open Questions

- Should `.scryrs/.env` support a non-`SCRYRS_REMOTE_*` short-key form (e.g. `INGEST_URL=`) for ergonomics, or strictly the canonical env names? (Leaning strictly canonical to avoid a second vocabulary.)
- Should `doctor` gain a dedicated check that validates `.scryrs/.env` completeness, or is the per-command fail-fast sufficient?
