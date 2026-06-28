---
name: read-project-docs
description: Consume project documentation and architecture as a structured knowledge base. Use whenever you need to understand how the project works, what conventions exist, where code lives, or what architectural decisions were made. Trigger on questions about project structure, component architecture, data flow, domain models, or when implementing a feature that requires cross-cutting understanding. Use before reading large amounts of source code — docs give you the map before you walk the terrain.
---

# Project Docs

The project has a self-documenting knowledge base:

1. **Rspress docs** — prose documentation about architecture, conventions, and development guides. Built from `.devagent/docs/docs/` and served at `http://project-docs` on the internal swarm network. Generates LLM-optimized output via `llms.txt`.

This is the project's "memory" — use it instead of reading source code blind.

## The Golden Rule: Index First, Deep-Dive Second

**Never bulk-load all docs at once.** The `llms.txt` file gives you a structured index of every documentation page with a 1-2 sentence summary. Use it to find the page(s) relevant to your task, then fetch only those specific `.md` pages.

This keeps your context window sharp. A targeted 200-line page beats a 2000-line dump every time.

## Docs System (Rspress)

### Where docs live

There are two execution contexts. Pick exactly one base URL first, then use only URL fetches from that base:

- **Swarm/agent runtime:** `http://project-docs`
- **Local developer runtime:** `http://localhost:<port>/project-docs`
port is available at .devagent/.env, if not set its 18080

**Hard rule:** never read host filesystem docs artifacts (for example `.devagent/docs/doc_build/*`) even if they exist locally. Always fetch docs via HTTP from the selected base URL.

| Location | Format | Access from |
|---|---|---|
| `<BASE_URL>/llms.txt` | Root taxonomy with top-level slices and drill links | Swarm or local developer runtime |
| `<BASE_URL>/llms-vision-strategy.txt` | Vision slice taxonomy | Swarm or local developer runtime |
| `<BASE_URL>/llms-architecture.txt` | Architecture slice taxonomy | Swarm or local developer runtime |
| `<BASE_URL>/llms-development.txt` | Development slice taxonomy | Swarm or local developer runtime |
| `<BASE_URL>/llms-*.txt` | Section-level taxonomy files | Swarm or local developer runtime |
| `<BASE_URL>/llms-full.txt` | Full content of all pages (Markdown) | Swarm or local developer runtime |
| `<BASE_URL>/<page-slug>.md` | Individual page as Markdown | Swarm or local developer runtime |

### Consumption workflow

**Step 0 — Select runtime context and base URL:**

- If running inside swarm/autonomous agents, set `BASE_URL=http://project-docs`
- **Local developer runtime:** `http://localhost:<port>/project-docs`
port is available at .devagent/.env, if not set its 18080

**Step 1 — Discover top-level slices:**
Read `<BASE_URL>/llms.txt`. This shows 3 top-level slices (Vision & Strategy, Architecture, Development) with drill links to slice files.

**Step 2 — Drill into the slice:**
Read `<BASE_URL>/llms-vision-strategy.txt`, `<BASE_URL>/llms-architecture.txt`, or `<BASE_URL>/llms-development.txt` to find the relevant section.

**Step 3 — Read targeted pages:**
Fetch only the specific `<BASE_URL>/<page>.md` files you need. Each `.md` file is pure Markdown — no HTML, no JavaScript, no navigation chrome. Perfect for context windows.

**Step 4 — Bulk load only as last resort:**
If you genuinely need everything (rare), read `<BASE_URL>/llms-full.txt`. This is the concatenated full content of every page. It is large. Prefer step 3.

### What docs typically cover

Project docs describe architecture overviews, cross-package contracts, development guides, conventions, and testing strategies. They complement (don't replace) the source code — docs tell you the "why" and "where," code tells you the "how."

### When docs are stale

Docs are living artifacts maintained by agents and developers. If you find outdated information, note it but don't trust it blindly. Cross-reference with the actual source code when in doubt.

## Which Source When

| You need to know... | Use... |
|---|---|
| How the manager works at a high level | Docs page (`manager.md`) |
| How to set up a local dev environment | Docs page (check `llms.txt` for setup/onboarding) |
| What testing conventions exist | Docs page (`testing.md`) |
| The exact API signature of a function | Source code (not docs or architecture) |

**Docs for understanding, code for implementation details.**

## Quick Start

```
1. Read llms.txt → discover top-level slices
2. Read llms-vision-strategy.txt, llms-architecture.txt, or llms-development.txt
3. Read the 1-2 .md pages relevant to your task
4. Only then dive into source code — now you know where to look
```

This approach gives you progressive depth: broad understanding first, implementation detail second.
