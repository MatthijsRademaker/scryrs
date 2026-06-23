# Claude Code Setup

This repository is configured for use with Claude Code. The project rules, custom prompts, and agent configurations are documented in **[AGENTS.md](./AGENTS.md)** — read it before working on this codebase.

## Directory Structure

Claude Code integrates with the existing project structure via `.claude/`:

- `.claude/agents` → symlink to `.pi/agents/` (custom agent definitions)
- `.claude/prompts` → symlink to `.pi/prompts/` (slash commands)
- `.claude/skills` → symlink to `.pi/skills/` (custom skills)
- `.claude/rules` → symlink to `.pi/rules/` (project-specific guardrails)
- `.claude/extensions` → symlink to `.pi/extensions/` (Claude Code extensions)
- `.claude/settings.json` → symlink to `.pi/settings.json` (Pi configuration)

This allows Claude Code to automatically discover your slash commands, skills, agents, and rules while maintaining the primary development workflow under `.pi/`.

## Key Project Rules

See [AGENTS.md](./AGENTS.md) for:
- Coding philosophy (think before coding, simplicity first, surgical changes)
- File organization and naming conventions
- Failure-fast principles and error handling
- Documentation structure (`.devagent/docs/` vs `src/dashboard/docs/`)
- Pi hook source ownership and runtime configuration rules

## Quick Start

1. Read [AGENTS.md](./AGENTS.md) for project culture and coding standards
2. Review `.pi/rules/` for project-specific guardrails
3. See `.devagent/docs/` for internal architecture and design decisions

## Documentation

| Path | Purpose |
|---|---|
| `.devagent/docs/` | Internal architecture, design decisions, testing patterns |
| `src/dashboard/docs/` | Public-facing user documentation |
| `.pi/rules/` | Project-specific guardrails and constraints |
