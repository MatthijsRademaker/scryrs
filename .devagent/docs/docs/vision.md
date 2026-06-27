# Vision & Goals

scryrs turns agent traces into living codebase knowledge. It observes how AI agents navigate a repository, detects context they keep rediscovering, and promotes repeated knowledge into durable docs, routing manifests, and reusable agent memory.

## Identity

scryrs is a **context intelligence suite for AI-assisted codebases**.

Tagline:

> Document what your agents keep rediscovering.

CLI subtitle:

> Find context hotspots. Promote them into durable knowledge.

scryrs is not an Rspress plugin. Rspress is one publishing surface. The core product is the intelligence layer that tells any docs system what it should know.

## Strategic Goals

- **Observe real agent behavior** — collect traces from coding sessions: files opened, searches run, symbols inspected, commands executed, docs retrieved, edits made, and repeated failed lookups.
- **Detect knowledge hotspots** — identify hot files, hot concepts, missing docs, context churn, repeated search terms, fragile code areas, and undocumented decision zones.
- **Promote repeated context** — create reviewable proposals for architecture notes, decision records, debugging playbooks, domain overviews, API contracts, agent skills, and memory patches.
- **Route future agents faster** — generate graph metadata, manifests, exports, and runtime hints so agents load relevant context before rediscovering it.

The product loop is:

```text
Observe → Detect → Promote → Route
```

## Product Shape

scryrs should grow as a suite with independent pieces:

```text
scryrs-core
= standalone trace ingestion, event model, and hotspot detector

scryrs-graph
= knowledge graph and routing manifest schema

scryrs-curator
= reviewable docs, skills, decisions, and memory proposal engine

scryrs-adapters
= integrations for Rspress, Markdown repos, Docusaurus, VitePress, and custom docs systems

scryrs-runtime
= agent-side retrieval and routing helper
```

Primary data flow:

```text
AI agent / coding harness
        ↓
scryrs trace collector
        ↓
event store
        ↓
hotspot analysis
        ↓
knowledge proposals
        ↓
curated docs / memory inbox
        ↓
graph + routing manifests
        ↓
agent runtime retrieval
```

Core commands should preserve the standalone hotspot-detector workflow:

```bash
scryrs record -- codex
scryrs hotspots
scryrs report
scryrs suggest-docs
```

Suite commands can extend that workflow:

```bash
scryrs graph build
scryrs memory propose
scryrs docs validate
scryrs route explain "prompt cache reuse"
scryrs export --format llms-graph
```

## Scope & Boundaries

In scope:

- Agent activity tracing from coding harnesses and CLIs.
- Hotspot scoring across files, symbols, concepts, searches, commands, docs, and failures.
- Reviewable knowledge proposals, not silent documentation changes.
- Machine-readable knowledge graph and routing manifest outputs.
- Adapters for common documentation systems.
- Runtime retrieval helpers for future agent sessions.

Out of scope:

- Replacing Rspress, Docusaurus, VitePress, or existing internal docs platforms.
- Treating `llms.txt` generation as the core product.
- Auto-merging generated knowledge without human review.
- Becoming a generic analytics dashboard disconnected from documentation and agent routing.

## Guiding Principles

### Behavior beats guesses

scryrs uses actual agent behavior to decide what knowledge deserves to exist. The differentiator is not generating routing files; it is learning from repeated context work.

### Promote, do not overwrite

Repeated context becomes proposals first: docs, ADRs, playbooks, skills, memory patches, and manifest updates. Humans review durable knowledge before it becomes source of truth.

### Integrate, do not replace

scryrs does not replace your docs system. It tells your docs system what it should know.

### Keep the core standalone

Hotspot detection must work without a docs site. Users can adopt scryrs with raw Markdown, an internal docs repo, Rspress, Docusaurus, VitePress, or no published docs surface.

### Make routing explainable

Future agents should understand why a document, skill, or memory item was recommended. Route explanations should cite traces, hotspots, graph links, and manifest rules.

## Target Users

- **AI-assisted engineering teams** that want agents to stop rediscovering the same repository context.
- **Maintainers of large codebases** who need signals about missing architecture docs, decision records, and debugging playbooks.
- **Developer experience teams** building agent-ready documentation and onboarding flows.
- **Docs maintainers** who want evidence-backed documentation priorities.
- **Agent platform builders** who need graph metadata, manifests, and runtime retrieval hints.

## Technical Context

Rspress should be a first-class adapter, not the foundation. Rspress can publish Markdown, generate `llms.txt` outputs, and expose build lifecycle hooks useful for graph and routing integration. scryrs should use those capabilities when present, while keeping all core intelligence independent from Rspress.

Possible outputs:

```text
/docs
/memory-inbox
/adr
/.scryrs/graph.json
/.scryrs/hotspots.json
/.scryrs/routes.json
llms.txt
nested llms.txt files
Markdown exports
vector index
BM25/FTS index
```

Package direction:

```text
scryrs
  Main CLI

scryrs-core
  Trace ingestion, event model, hotspot scoring

scryrs-graph
  Knowledge graph schema and validators

scryrs-curator
  Memory and doc proposal engine

scryrs-rspress
  Rspress adapter/plugin

scryrs-md
  Generic Markdown adapter

scryrs-agent
  Runtime routing/retrieval SDK
```

Rust crate direction:

```text
scryrs
scryrs-core
scryrs-trace
scryrs-graph
scryrs-curator
scryrs-adapter-rspress
scryrs-adapter-markdown
```

Future npm integrations:

```text
@scryrs/rspress
@scryrs/docusaurus
@scryrs/vitepress
```

## Product Vocabulary

```text
scryrs trace
Records agent activity from coding sessions.

scryrs hotspots
Shows which files, concepts, and searches repeatedly consume agent context.

scryrs propose
Creates reviewable documentation, decision, and skill proposals.

scryrs graph
Builds a machine-readable knowledge graph from docs, frontmatter, links, and agent traces.

scryrs route
Explains which docs an agent should read for a given task.

scryrs adapters
Publishes graph output into Rspress, Markdown, llms.txt, nested manifests, or custom formats.
```

## Anti-Goals

- Do not position scryrs as only an Rspress plugin.
- Do not make any docs framework mandatory.
- Do not optimize around generating `llms.txt` alone.
- Do not bury users in analytics without clear promotion paths.
- Do not turn agent memory into unreviewed, invisible state.
- Do not require vector search or hosted infrastructure for the basic workflow.

## Related Pages

- [Architecture](./architecture.md) — system diagrams and architecture notes as implementation matures.
- [Product Roadmap](./roadmap.md) — delivery sequence from proxy capture to routing and proposal features.
- [Hotspots](./hotspots.md) — domain concept, scoring, and interpretation guide for scryrs hotspot reports.
- [Graph](./graph.md) — domain-oriented explanation of the knowledge graph and how evidence becomes routing context
