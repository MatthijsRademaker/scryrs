---
name: docs-writer
description: Write and rewrite project documentation. Use when you discover outdated docs, stale diagrams, missing concepts, wrong terminology, or missing cross-references. This skill covers the writer's judgment, what deserves a doc, what belongs in code comments instead, and how to structure the result.
---

# Docs Writer

This skill tells you when, what, and how to write or rewrite project documentation. It encodes the patterns demonstrated in the `architecture.md` rewrite — the same judgment calls about abstraction level, diagram style, and structural organization.

There are two documentation trees in this project. This skill covers only the **first one**:

| Path                   | Audience               | What belongs                                                                                  |
| ---------------------- | ---------------------- | --------------------------------------------------------------------------------------------- |
| `.devagent/docs/docs/` | **Project developers** | Architecture, contracts, runtime models, extension guides, design decisions, testing patterns |
| `src/dashboard/docs/`  | **Swarm users**        | Public-facing content shipped with the dashboard — shader/material reference examples         |

If the user asks about the dashboard docs, do not use this skill. Switch to the `frontend-design` skill instead.

## Architecture Page Contract

When `.devagent/docs/docs/architecture.mdx` exists, its `<likec4-view>` embeds MUST match the actual view IDs in `.devagent/architecture/views/`. Stale view IDs are a doc bug. After changing architecture views, update `architecture.mdx` and rebuild docs.

## When to Write or Rewrite a Doc

### Signals that a doc is needed

- **Code-drift** — the code does something materially different from what the doc describes (wrong algorithm, wrong topology, wrong interface, removed concept).
- **Outdated terminology** — the doc uses names that no longer match the codebase (e.g., "Pi subagent" when the code says `task_reactive` runtime with `PiRunner` backend).
- **Missing concept** — a significant architectural pattern exists in the code but has no documentation entry (e.g., `ExecutionStrategy` pattern, `OutcomeArtifact` system, workflow engine).
- **Wrong diagram** — an ASCII art or text diagram suggests incorrect relationships (processes shown as nested when they're co-equal, missing runtime layers, hierarchies that don't exist).
- **Broken cross-references** — "See [X](./nonexistent.md)" leads to a 404 or a page that doesn't match the described topic.
- **Missing "why"** — the doc describes _what_ something does but never explains _why_ it exists, what problem it solves, or what alternatives were rejected. This is the most common deficiency.
- **Implementation details in prose** — the doc describes line-by-line logic, parameter types, or function signatures that belong in code comments or godoc. Move them out.
- **After an architecture change** — new agent type added, new runtime model, new interface or contract, new directory structure that changes how components are organized. If you had to read the code to understand the change, the docs need updating so the next person doesn't have to.
- **After a code review discovers confusion** — if a reviewer (human or agent) had to ask "how does X work?" or "why is Y structured like that?", capture the answer.

### Signals that don't need a doc

- **A function signature changed** — update the godoc/comment, not the prose doc.
- **A bug was fixed** — the fix is in the code; no doc needed unless the fix reveals a misunderstanding of the architecture that needs correcting.
- **A new utility function was added** — docs describe patterns, not individual helpers.
- **A config value changed** — unless it's a new concept (e.g., a new runtime mode), a changed default doesn't warrant a doc update.

## What to Capture (and What to Leave Out)

### Belongs in docs

- **Architecture principles** — separation of concerns, abstraction layers, why things are split the way they are.
- **Interface boundaries** — Go interfaces, gRPC service definitions, runner contracts. Show the interface signature, list implementations, explain what each is for.
- **Execution models** — lifecycle (register → claim → execute → report), runtime vs strategy vs backend, event-driven vs polled vs long-lived.
- **Non-obvious file layouts** — where prompts live, where agent definitions live, how they're embedded. Show the directory tree.
- **Cross-package relationships** — who depends on what, how data flows between slices.
- **Design decisions** — why one approach was chosen over another. Reference ADRs (`decisions/*.md`) where they exist.
- **Source-of-truth rules** — when two files define overlapping behavior (command template vs task prompt), which one wins, and what the current alignment is.
- **Concrete how-it-works for non-obvious mechanics** — things like outcome artifact enforcement, workflow engine evaluation, command sync. If a developer would have to read 3+ source files to understand it, it belongs in docs.

### Belongs in code comments / godoc, not prose

- **Function signatures and parameter types** — godoc is right there.
- **Step-by-step call graphs** — an "X calls Y which calls Z" text chain is fragile and valuable only when you're debugging.
- **Configuration fields and defaults** — unless the config structure itself is a concept worth documenting.
- **Internal implementation details** — how a specific algorithm works, what edge cases it handles.

## How to Structure a Doc Page

### 1. State the architecture problem immediately

The first paragraph after the heading must answer: _What does this slice/component do, and who does it talk to?_ One or two sentences. No throat-clearing.

**Bad:**

> In the agents slice, there are several different types of agents that perform various functions in the swarm system. These agents are implemented using a runtime model that...

**Good:**

> The agents slice (`src/agents/`) powers Swarm's autonomous workflow: worker agents implement tasks, the product owner generates new tasks from project context, and review/refinement specialists run as task-reactive agents backed by the Pi CLI. All agent runtimes communicate with the manager over gRPC.

### 2. Show topology with an ASCII diagram

For any page that describes how components fit together, draw a topology diagram. Rules:

- **Show containers/host boundaries** (Docker containers, processes, shared binaries)
- **Show co-equal processes flat, not nested** — independent processes as siblings, not children
- **Show what talks to what** over what protocol
- **Label source directories** so readers know where to look
- Keep it within 15-20 lines max. A diagram that takes 40 lines is too detailed.

```
┌──────────┐     ┌────────────────────────────────┐
│ Manager  │◄───►│ Agent Containers               │
│ gRPC     │     │                                │
│ REST     │     │  ┌─ Worker (long-lived)        │
│ Postgres │     │  ├─ Reviewer (task_reactive)   │
└──────────┘     │  ├─ Architect (task_reactive)  │
                 │  └─ ...                        │
                 └────────────────────────────────┘
```

### 3. Show the runtime flow with a second diagram

When there's a lifecycle or execution loop, draw a second diagram showing the flow. This is separate from the topology diagram — topology shows _where things are_, flow shows _how things work_.

```
┌──────────────────────────────────┐
│ Register → ClaimLoop {           │
│   Poll → Claim → Execute → Report│
│   Handle pending actions (PRs)   │
│ }                                │
│                                  │
│ ExecutionStrategy implementations:│
│ ├─ WorkExecution                 │
│ └─ ReviewExecution               │
└──────────────────────────────────┘
```

### 4. Explain the abstraction layers

Architecture docs should reveal the layer cake. For each layer, explain:

- **What it owns** (lifecycle? behavior? execution?)
- **How it's implemented** (interface, struct, file path)
- **Why it exists** (what problem it solves, what decoupling it provides)

Pattern: after the diagrams, write a paragraph that begins with the key insight:

> The key architectural separation: the **Runtime** owns the lifecycle (register, poll, claim, report), the **ExecutionStrategy** defines agent-specific behavior, and the **AgentRunner** (PiRunner) handles the actual cognitive execution.

This is the "aha" sentence — the one thing a developer needs to grasp before they can navigate the code.

### 5. Use tables with column schemas that match the subject

Don't default to "Name / Description / Notes." Choose columns that answer the questions a reader actually has:

| Agent | Definition | Purpose | Execution model |
| ----- | ---------- | ------- | --------------- |

For each row:

- **Identity** — how is it named? (Go interface? `.pi/agents/*.md`?)
- **Definition** — where is it defined? (interface file, agent definition path)
- **Purpose** — what does it do? (one clear sentence)
- **Execution model** — how does it run? (runtime type, strategy name, backend)

### 6. Subsections for non-obvious mechanisms

After the main architecture description, add subsections for anything a developer would have to reverse-engineer from code:

- **Outcome enforcement** — "The runtime enforces explicit outcome reporting through file-based OutcomeArtifact files..."
- **Source-of-truth rules** — "The command template is authoritative for behavioral instructions..."
- **Migration notes** — "Planned for migration to generic runtime..."

### 7. Pin file paths

Every significant file or directory mentioned should include its absolute path from the repo root. Readers use these to navigate. Format: `` `src/path/to/file.go` ``

### 8. Cross-reference other doc pages

Every page should end with a "Related Pages" section:

- [Link](../page.md) — one-line description of what that page covers

Include only the 2-4 most immediately relevant pages. Don't dump the entire sidebar.

### 9. Add historical notes when naming or structure is legacy

When the code has evolved but old names persist in config, env vars, or conventions, call it out:

> **Note:** Despite the historical name, Swarm does not use Pi's built-in subagent() tool for dispatching primary agents — the name is historical. See [page](./page.md).

This prevents future readers from being misled by stale naming.

## Writing Style Principles

### No throat-clearing

Every paragraph earns its place. If you can delete a paragraph and lose nothing, delete it. This includes intro sentences like "In this section, we will discuss..." and transitions like "Now that we understand X, let's move on to Y."

### Explain the "why" before the "what"

For any architecture choice, start with the problem it solves, then describe the solution. Readers need the problem context to evaluate whether the solution is right for their use case.

### One concept per paragraph

If a paragraph tries to explain both the Runtime and the ExecutionStrategy, split it. Each paragraph makes one point. Readers (especially LLMs) rely on paragraph boundaries for chunking.

### Use code blocks for code-adjacent content

File paths, directory trees, interface signatures, and config structures all go in ` ``` ` blocks. Inline backticks are for short references (a single path, a single type name).

### Don't repeat what's in the code

If the code is clear — the interface is defined, the types are documented — the prose doc should say "X implements Y (see `path/to/file.go`)" and explain _why_ Y exists, not restate the godoc.

### Be honest about unfinished work

Doc pages that describe an aspirational state without acknowledging current gaps create more confusion than omission. If a component is "planned but not yet implemented" or migration is "pending," say so explicitly. Use the same format:

> Planned migration to `task_reactive` runtime. Currently `special`.

### Write for a skim-reader

Not every reader will read the full page. Use:

- **Bold** for the first mention of each key term
- Bullet lists for parallel items (agent types, file locations, design principles)
- Numbered lists for sequential steps (consumption workflow, migration path)
- Short paragraphs (3-5 lines max)

## Workflow: How to Write or Rewrite a Doc

### Step 1 — Assess what's stale

Read the current doc page. For every claim, cross-reference against the source code. Ask:

- Does this interface still exist at this path?
- Does this agent type still use this runtime?
- Is this diagram still an accurate representation of the topology?
- Are these cross-references valid?

Flag each discrepancy. Don't fix yet — just catalog.

### Step 2 — Identify what's missing

Read the relevant source code directories. Look for:

- Types defined but undocumented (new interfaces, new structs with important roles)
- Patterns used across multiple files (strategy pattern, artifact enforcement, runner backend)
- Directory structures that are non-obvious (embedded files, generated code, mirror directories)

Ask: "If I had to explain this code to another developer without showing them the source, what would I need to tell them?" That's your missing content.

### Step 3 — Structure the rewrite

Using the principles above, sketch the page structure:

1. Architecture problem (one sentence)
2. Topology diagram
3. Runtime flow diagram
4. Abstraction layer explanation
5. Table of components
6. Subsections for non-obvious mechanisms
7. Execution backend details
8. Related pages

### Step 4 — Write the diagrams first

Diagrams force you to understand the topology and flow before you write prose. If you can't draw the diagram correctly, you don't understand the architecture well enough to write about it. Iterate on the diagram until it's accurate — the prose will follow naturally.

### Step 5 — Write the "key insight" paragraph

This is the paragraph that begins "The key architectural separation..." or "The central design principle..." Write it immediately after the diagrams. Everything else in the page should support or elaborate on this insight.

### Step 6 — Fill in the rest

Write each section in order. Cross-reference related pages as you go. Pin file paths as you write.

### Step 7 — Verify every claim

Reread the doc. For each claim that describes behavior, confirm it against source code. If you wrote "The runtime polls every 5 seconds," check that the default is actually 5 seconds. If you wrote "Reviewer resolves from .pi/agents/swarm-reviewer.md," confirm the file exists at that path.

### Step 8 — Rebuild and verify

Run `bun run build` in `.devagent/docs/` and confirm no dead links or build errors. Verify all taxonomy files are generated in `doc_build/`: `llms.txt` (root), `llms-architecture.txt` + `llms-development.txt` (slice files), and one `llms-{section}.txt` per section (for example `llms-manager.txt`, `llms-agents.txt`, `llms-agent-cli.txt`). Cross-reference the output against the `TAXONOMY` constant in `rspress.config.ts` — every page must appear in the correct section file with the correct description.

## Related Pages

- [Project Docs Skill](../project-docs/SKILL.md) — how to _consume_ docs (ingestion workflow, llms.txt, golden rule of index-first)
