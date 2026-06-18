# Swarm Pi Default Setup

This directory is the **scaffold source** for all Swarm Pi resources in this project. It was created by `swarm init` from bundled Swarm defaults and is used to materialize the runtime Pi configuration under `.pi/`.

## What lives here

| Directory | Purpose |
|---|---|
| `agents/` | Default agent definition files (`.md`) — scaffolded into `.pi/agents/` |
| `prompts/` | Default command prompt templates (`.md`) — scaffolded into `.pi/prompts/` |
| `skills/` | Default agent skills — scaffolded into `.pi/skills/` |
| `rules/` | Default shared rules — scaffolded into `.pi/rules/` |
| `config/` | Default runtime config — scaffolded into `.pi/config/` |
| `extensions/` | Default Pi extensions — scaffolded into `.pi/extensions/` |
| `package.json` | Declares the scaffold package metadata |

## How it works

1. **`swarm init`** seeds this directory from bundled Swarm defaults (in the binary).
2. **The materializer** reads files here and writes them into `.pi/` as the live runtime configuration.
3. **Pi, the manager, and the dashboard** consume only the materialized `.pi/` resources — they do not read from here.
4. **Future `swarm update`** flow will compare `.pi/` against this directory to detect customizations and safely update unchanged files.

## Do not edit

Files in this directory are **scaffold defaults**, not runtime configuration. To customize agent behavior, prompts, skills, or rules, edit the materialized files under `.pi/` instead:

- Agent definitions → `.pi/agents/<name>.md`
- Command prompts → `.pi/prompts/<name>.md`
- Skills → `.pi/skills/<name>/SKILL.md`
- Rules → `.pi/rules/<name>.md`

Changes made here will be overwritten by future `swarm init` or `swarm update` runs.
