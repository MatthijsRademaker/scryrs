# Swarm Pi Scaffold

This directory is the materialized Pi runtime configuration for this Swarm project. It was materialized from `.pi/swarm-pi-default-setup/` by `swarm init`.

## What lives here

| Directory | Purpose |
|---|---|
| `agents/` | Agent definition files (`.md`) — runtime agent identity and skill wiring |
| `prompts/` | Command prompt templates (`.md`) — consumed by swarm commands |
| `skills/` | Agent skills — loaded by Pi at runtime |
| `rules/` | Shared rules — injected into agent context at runtime |
| `config/` | Runtime config — provider and agent YAML configuration |
| `extensions/` | Pi extensions — runtime tooling for agent harness |
| `package.json` | Pi package manifest declaring extensions, skills, and prompts |
| `settings.json` | Pi harness configuration entrypoint |
| `swarm-pi-default-setup/` | Scaffold source bundle seeded by `swarm init` — used for updates |

## How it works

1. **`swarm init`** seeds `.pi/swarm-pi-default-setup/` from bundled Swarm defaults.
2. **The materializer** reads files from `swarm-pi-default-setup/` and writes them into the active `.pi/` tree.
3. **Pi, the manager, and the dashboard** consume only the materialized `.pi/` resources — they do not read from `swarm-pi-default-setup/`.
4. **Future `swarm update`** flow compares `.pi/` against `swarm-pi-default-setup/` to detect customizations and safely update unchanged files.

## Customization

To customize agent behavior, prompts, skills, or rules, edit the materialized files in the active `.pi/` tree:

- Agent definitions → `agents/<name>.md`
- Command prompts → `prompts/<name>.md`
- Skills → `skills/<name>/SKILL.md`
- Rules → `rules/<name>.md`

Changes to `swarm-pi-default-setup/` will be overwritten by future `swarm init` or `swarm update` runs.

## Scryrs-specific notes

- `hooks/pi/index.ts` is the **only** canonical source for the Pi trace hook. The installed runtime copy at `extensions/scryrs/index.ts` is a non-canonical artifact created by `scryrs init --agent pi` for local dogfooding. Do not edit it directly.
- Agent model override fields (`modelEasy`, `modelModerate`, `modelComplex`) are active runtime configuration — do not remove them.
- The `shadcn-vue` skill is a scryrs-specific dashboard UI capability.
- `scripts/check`, `scripts/test`, `scripts/security`, and `scripts/precommit-run` are the authoritative verification surface for this Rust repository.
