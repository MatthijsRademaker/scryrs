# AGENTS.md

This file contains project-wide instructions for all agents.
It is merged with agent-specific prompts at runtime.

## Verification

Before committing or claiming a task is done, verify your changes:

```bash
scripts/precommit-run
```

## Code conventions

- Prefer executable truth in source files over documentation.
- Run Go and Make commands from ``src/`` (the Go module root).
- Follow existing patterns in the codebase.
