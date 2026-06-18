# Dev Swarm Project

This project is configured to use the **Dev Swarm** - an autonomous AI agent system for software development.
The dashboard is the primary place to authenticate and work through onboarding.

## Overview

The swarm consists of specialized AI agents that work together to implement features, review code, and manage the backlog:

| Agent | Role |
|-------|------|
| **Worker** | Implements features and fixes bugs |
| **Architect** | Reviews architecture decisions |
| **Lead Dev** | Quality gate and code review |
| **Reviewer** | Reviews PRs and merges approved code |
| **Product Owner** | Generates and prioritizes stories |

## First Setup

If this is your first time in the repository, use this sequence:

```bash
# 1) Initialize or upgrade project scaffolding
swarm init
```

`swarm init` creates `.pre-commit-config.yaml` when missing. Worker clones install and enforce these hooks before commit, so host-side `pre-commit install` is optional.

### Agent Harness

Swarm uses **Pi** as its agent harness. Pi configuration is managed through `.pi/settings.json` in the repository root. All required extensions (`pi-web-access`, `pi-lens`, `pi-subagents`) are declared there and auto-installed by Pi on first use.

`scripts/` contains Docker-backed verification scripts. Task-reactive agents run as privileged Docker-in-Docker containers with `SWARM_DIND_ENABLED=true`. Each agent starts its own internal Docker daemon (available at `/var/run/dind/docker.sock`) and runs verification scripts against that daemon. Verification scripts use `scripts/lib/docker-verification.sh`, run normal tool containers as the workspace UID/GID, and store Go/npm/pre-commit tool caches in project-scoped Docker named volumes instead of repository-local `.cache/go-build`, `.cache/go-mod`, `.cache/npm`, or `.cache/pre-commit` paths. This gives agents the ability to run `go vet`, `go test`, and other checks without requiring Go/Node.js installed in the agent image, while keeping container operations isolated from the host Docker daemon and avoiding root-owned workspace cache failures.

`.devagent/.env` is local runtime configuration for this checkout. It stores project identity (`PROJECT_ID`), startup defaults (ports and feature flags), and optional image overrides (`MANAGER_IMAGE`, `TASK_REACTIVE_IMAGE`, `PRODUCTOWNER_IMAGE`, `DASHBOARD_IMAGE`, `DOCS_IMAGE`, `TRAEFIK_IMAGE`). Re-run `swarm init` in a new clone to recreate it; it does not need to be committed.

## Auth Setup

```bash
# Set your API key
export ANTHROPIC_API_KEY=sk-ant-...

# Verify Pi is installed
pi --version
```

Pi workers use the API key passed through Docker volumes. Configure your key in the dashboard Auth Setup panel.

## Runtime Availability

The open-source install MVP guarantees `install -> swarm init`.

`swarm manager start` launches the manager, dashboard, docs, and supporting runtime services.

Manual image overrides remain supported in `.devagent/.env`. Until public image endpoints are available everywhere, override them in `.devagent/.env` using `MANAGER_IMAGE`, `TASK_REACTIVE_IMAGE`, `PRODUCTOWNER_IMAGE`, `DASHBOARD_IMAGE`, `DOCS_IMAGE`, and `TRAEFIK_IMAGE`.

If manager startup fails with a port conflict, check for containers using the default ports:

```bash
docker ps --filter "publish=18080"
docker ps --filter "publish=50051"
```

Then either stop the conflicting container or change `MANAGER_PORT` / `MANAGER_GRPC_PORT` in `.devagent/.env`.

## Project Structure

```
.devagent/                  # Swarm project configuration
├── .env                    # Local runtime config for this checkout
├── .gitignore              # Ignores local runtime artifacts
├── docs/                   # Internal developer docs (Rspress)
└── README.md               # This file

.pi/                        # Authoritative agent configuration (source of truth)
├── agents/                 # Agent definitions (swarm-*.md)
├── prompts/                # Command prompt templates
├── rules/                  # Agent rules (verification workflow)
├── skills/                 # Agent skills for autonomous agents
├── swarm-pi-default-setup/ # Scaffold/update source bundle
├── package.json            # Materialized Pi package manifest
└── settings.json           # Pi configuration entrypoint

scripts/                    # Docker-backed verification scripts
.pre-commit-config.yaml     # Repository hook policy enforced by workers
AGENTS.md                   # Project-wide agent instructions
```

## Where to Look First

- `.devagent/README.md` - Onboarding, commands, and troubleshooting
- `.devagent/.env` - Local runtime config, including PROJECT_ID and manual image overrides
- `.pi/agents/` - Primary swarm agents (authoritative)
- `.pi/prompts/` - Command prompt templates (feedback/plan/review/execute)
- `.pi/skills/` - Skills loaded by agents (including `swarm-board`)
- `scripts/` - Docker-backed verification scripts (build, test, lint)
- `AGENTS.md` - Project-wide instructions for all agents

## How It Works

### Task Flow

 ```
Backlog → Refinement → PreparingForDev → ReadyForDev → InProgress → Review → Done
            ↑              ↑                  ↑             ↓          ↓
       (Architect)    (Spec Writer)        (Worker)      (Worker)   (Reviewer)
        (Lead Dev)                                          ↓          ↓
                                                       Creates PR   Merges PR
```

## Commands

```bash
# Infrastructure
swarm manager start                 # Start the runtime (manager, dashboard, docs)
swarm manager stop                  # Stop the runtime

# Initialization
swarm init                          # Initialize or upgrade project scaffolding
```

Agent fleet management (starting, stopping, updating agents) is done through the dashboard **Agent Fleet** panel — see the Dashboard section below.

## Dashboard

Access the dashboard at:

```bash
swarm manager start

http://localhost:18080/dashboard/
```

The dashboard provides:
- **Auth Setup** — Connect GitHub and configure your API key
- **Task Board** — Kanban board with full task lifecycle tracking
- **Agent Fleet** — Start, stop, and configure agent runtime configs (worker, reviewer, architect, lead-dev)
- **Configuration** — Startup configuration and system runtime settings panels
- **Workflows** — Manage and assign agent workflows
- Real-time event stream and statistics

Note: Dashboard/API are available after the manager starts successfully.

## Swarm Documentation

Access the hosted Swarm documentation at:

```
http://localhost:18080/swarm-docs/
```

## Manager API

The manager exposes a REST API at `http://localhost:18080/api/v1/`:

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/tasks` | GET | List tasks |
| `/tasks` | POST | Create task |
| `/tasks/{id}` | GET | Get task details |
| `/stats` | GET | Dashboard statistics |
| `/agents` | GET | List active agents |
| `/events` | GET | Recent events |

## Troubleshooting

### Authentication issues

Start the manager if it is not already running, then open the dashboard and use the Auth Setup panel:
```bash
swarm manager start

# Then open:
http://localhost:18080/dashboard/
```
