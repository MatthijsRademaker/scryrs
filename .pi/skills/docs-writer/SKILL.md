---
name: docs-writer
description: Write, rewrite, and verify project documentation in `.devagent/docs/`. Use proactively after any code or architecture change that affects documented behavior — new crates, Cargo features, traits, contracts, or LikeC4 views — not only when you stumble on outdated docs, stale diagrams, missing concepts, wrong terminology, or broken cross-references. Covers the writer's judgment (what deserves a doc vs. a code comment), how to structure a page, and how to verify every claim against source and rebuild the docs afterward.
---

# Docs Writer

This skill tells you when, what, and how to write or rewrite project documentation. It encodes the patterns demonstrated in the `architecture.mdx` rewrite — the same judgment calls about abstraction level, diagram style, and structural organization.

This project keeps developer documentation under `.devagent/docs/docs/`. This skill covers that tree:

| Path                   | Audience               | What belongs                                                                                       |
| ---------------------- | ---------------------- | -------------------------------------------------------------------------------------------------- |
| `.devagent/docs/docs/` | **Project developers** | Architecture, crate contracts, CLI behavior, execution flow, design decisions, testing patterns    |

The dashboard is a Vue frontend at `crates/scryrs-dashboard/frontend/`, not a docs tree. If the user asks about dashboard UI, do not use this skill — switch to the `shadcn-vue` skill instead.

## Architecture Page Contract

`.devagent/docs/docs/architecture.mdx` embeds LikeC4 diagrams with `<likec4-view view-id="…">`. Each `view-id` MUST match a view defined in `.devagent/architecture/views.c4` (elements live in `model.c4`, the spec in `spec.c4`). Current view IDs: `workspace-topology`, `runtime-flow`, `roadmap-phases`, `guardrail-boundary`, `publishing-surfaces`. A `view-id` with no matching view is a doc bug — the embed renders empty rather than failing the build. After changing the `.c4` sources, update `architecture.mdx` and rebuild docs (`bun run build` runs the LikeC4 codegen first).

## When to Write or Rewrite a Doc

### Signals that a doc is needed

- **Code-drift** — the code does something materially different from what the doc describes (wrong algorithm, wrong topology, wrong interface, removed concept).
- **Outdated terminology** — the doc uses names that no longer match the codebase (e.g., calling something the "detector" when the crate is `scryrs-core` exposing `TraceQuery` and the hotspot scorer).
- **Missing concept** — a significant architectural pattern exists in the code but has no documentation entry (e.g., the Cargo feature composition model, the guardrail boundary, the adapter publishing surfaces, the `HotspotsReport` schema).
- **Wrong diagram** — a LikeC4 view or ASCII sketch suggests incorrect relationships (crates shown as nested when they're co-equal, a dependency drawn backwards, a missing layer).
- **Broken cross-references** — "See [X](./nonexistent.md)" leads to a 404 or a page that doesn't match the described topic.
- **Missing "why"** — the doc describes _what_ something does but never explains _why_ it exists, what problem it solves, or what alternatives were rejected. This is the most common deficiency.
- **Implementation details in prose** — the doc describes line-by-line logic, parameter types, or function signatures that belong in code comments or rustdoc. Move them out.
- **After an architecture change** — new crate added, new Cargo feature, new trait or cross-crate contract in `scryrs-types`, new LikeC4 view, or a directory restructure that changes how components are organized. If you had to read the code to understand the change, the docs need updating so the next person doesn't have to.
- **After a code review discovers confusion** — if a reviewer (human or agent) had to ask "how does X work?" or "why is Y structured like that?", capture the answer.

### Signals that don't need a doc

- **A function signature changed** — update the rustdoc (`///`) comment, not the prose doc.
- **A bug was fixed** — the fix is in the code; no doc needed unless the fix reveals a misunderstanding of the architecture that needs correcting.
- **A new utility function was added** — docs describe patterns, not individual helpers.
- **A config value changed** — unless it's a new concept (e.g., a new Cargo feature or runtime mode), a changed default doesn't warrant a doc update.

## What to Capture (and What to Leave Out)

### Belongs in docs

- **Architecture principles** — separation of concerns, abstraction layers, why things are split the way they are.
- **Crate and trait boundaries** — Rust traits, cross-crate contracts in `scryrs-types`, the Cargo feature matrix in `scryrs-cli/Cargo.toml`. Show the trait signature or feature set, list implementors/composers, explain what each is for.
- **Execution models** — command flow (`scryrs record` captures trace events → `.scryrs/scryrs.db` → `scryrs hotspots` scores → `HotspotsReport`), deterministic core vs. guardrail crates vs. adapters.
- **Non-obvious file layouts** — where the architecture `.c4` sources live, where docs pages live, how the LikeC4 webcomponent is generated. Show the directory tree.
- **Cross-package relationships** — who depends on what, how data flows between crates.
- **Design decisions** — why one approach was chosen over another. Reference ADRs (`decisions/*.md`) where they exist.
- **Source-of-truth rules** — when two files define overlapping behavior (workspace `Cargo.toml` vs. `scryrs-cli/Cargo.toml` feature matrix; `.devagent/architecture/*.c4` vs. the `architecture.mdx` embeds), which one wins, and what the current alignment is.
- **Concrete how-it-works for non-obvious mechanics** — things like the hotspot scoring weight table and tie-break, the guardrail boundary (policy before model use), or how adapters publish without depending on a docs framework. If a developer would have to read 3+ source files to understand it, it belongs in docs.

### Belongs in code comments / rustdoc, not prose

- **Function signatures and parameter types** — rustdoc is right there.
- **Step-by-step call graphs** — an "X calls Y which calls Z" text chain is fragile and valuable only when you're debugging.
- **Configuration fields and defaults** — unless the config structure itself is a concept worth documenting.
- **Internal implementation details** — how a specific algorithm works, what edge cases it handles.

## How to Structure a Doc Page

### 1. State the architecture problem immediately

The first paragraph after the heading must answer: _What does this crate/component do, and who does it talk to?_ One or two sentences. No throat-clearing.

**Bad:**

> scryrs is a Rust project with several crates that perform various functions. These crates are organized into a workspace that...

**Good:**

> scryrs is a Rust workspace split by product capability: the CLI composes feature-gated crates, the deterministic core owns trace and hotspot logic, guardrail crates bound risky behavior, and adapters publish knowledge to documentation systems. `crates/scryrs-cli` composes the pieces through Cargo features.

### 2. Show topology with a diagram

For any page that describes how components fit together, show a topology diagram.

**On the architecture page, diagrams are LikeC4 views, not ASCII.** Edit the source in `.devagent/architecture/*.c4` and embed the view by id:

```mdx
<likec4-view view-id="workspace-topology" browser="false"></likec4-view>
```

Add or change the view in `views.c4` (and its elements in `model.c4`) first, then reference it. Never hand-draw a topology the `.c4` model could render — the two will drift.

For smaller or secondary pages where a full LikeC4 view is overkill, an ASCII sketch is fine. Rules:

- **Show co-equal crates/processes flat, not nested** — siblings, not children
- **Show what depends on or talks to what**
- **Label crate/source paths** so readers know where to look
- Keep it within 15-20 lines max. A diagram that takes 40 lines is too detailed.

```
┌──────────────────────────────────────────────────┐
│ scryrs-cli  (composition via Cargo features)       │
│  ├─ suite:       core · graph · curator             │
│  ├─ guardrails:  policy · sandbox · telemetry       │
│  └─ adapters:    markdown · rspress · harness        │
│        all depend on → scryrs-types (shared contracts)│
└──────────────────────────────────────────────────┘
```

### 3. Show the runtime flow with a second diagram

When there's an execution flow, show it separately from topology — topology shows _where things are_, flow shows _how things work_. On the architecture page this is the `runtime-flow` LikeC4 view; elsewhere an ASCII sketch works:

```
┌──────────────────────────────────────────────────┐
│ scryrs record   → capture trace events             │
│        ↓                                           │
│ .scryrs/scryrs.db   (TraceQuery store)             │
│        ↓                                           │
│ scryrs hotspots → deterministic scoring            │
│        ↓                                           │
│ HotspotsReport  → stdout + .scryrs/hotspots.json   │
└──────────────────────────────────────────────────┘
```

### 4. Explain the abstraction layers

Architecture docs should reveal the layer cake. For each layer, explain:

- **What it owns** (a product capability? a guardrail boundary? a publishing surface?)
- **How it's implemented** (crate, trait, file path)
- **Why it exists** (what problem it solves, what decoupling it provides)

Pattern: after the diagrams, write a paragraph that begins with the key insight:

> The key architectural separation: **suite crates own product capabilities**, **guardrail crates bound unsafe or privacy-sensitive behavior**, and **adapters are publishing surfaces**. `crates/scryrs-cli` composes them through Cargo features; `crates/scryrs-types` keeps cross-crate contracts small and explicit.

This is the "aha" sentence — the one thing a developer needs to grasp before they can navigate the code.

### 5. Use tables with column schemas that match the subject

Don't default to "Name / Description / Notes." Choose columns that answer the questions a reader actually has:

| Crate | Role | Why it exists |
| ----- | ---- | ------------- |

For each row:

- **Identity** — the crate or module name (`crates/scryrs-core`)
- **Role** — what capability it owns (one clear sentence)
- **Why it exists** — what problem it solves or what boundary it enforces
- **Composition** — where relevant, which Cargo feature pulls it in (`default`, `guardrails`, `full`)

### 6. Subsections for non-obvious mechanisms

After the main architecture description, add subsections for anything a developer would have to reverse-engineer from code:

- **Scoring rules** — "Hotspot scoring uses a documented weight table and a six-key tie-break, emitting a versioned `HotspotsReport` (schemaVersion 1.0.0)..."
- **Source-of-truth rules** — "The workspace `Cargo.toml` defines membership; `scryrs-cli/Cargo.toml` is authoritative for the product feature matrix..."
- **Scaffold notes** — "Graph, curator, and adapter crates are scaffold-level (sentinel descriptor tests only), pending implementation..."

### 7. Pin file paths

Every significant file or directory mentioned should include its path from the repo root. Readers use these to navigate. Format: `` `crates/scryrs-core/src/lib.rs` ``

### 8. Cross-reference other doc pages

Every page should end with a "Related Pages" section:

- [Link](../page.md) — one-line description of what that page covers

Include only the 2-4 most immediately relevant pages. Don't dump the entire sidebar.

### 9. Add historical notes when naming or structure is legacy

When the code has evolved but old names persist in config, env vars, or conventions, call it out:

> **Note:** The `scryrs-graph`, `scryrs-curator`, and adapter crates exist as scaffolds with sentinel tests only — their directories and descriptors are intentional placeholders, not abandoned code. See [Architecture](./architecture.mdx).

This prevents future readers from being misled by stale naming.

## Writing Style Principles

### No throat-clearing

Every paragraph earns its place. If you can delete a paragraph and lose nothing, delete it. This includes intro sentences like "In this section, we will discuss..." and transitions like "Now that we understand X, let's move on to Y."

### Explain the "why" before the "what"

For any architecture choice, start with the problem it solves, then describe the solution. Readers need the problem context to evaluate whether the solution is right for their use case.

### One concept per paragraph

If a paragraph tries to explain both the suite crates and the guardrail boundary, split it. Each paragraph makes one point. Readers (especially LLMs) rely on paragraph boundaries for chunking.

### Use code blocks for code-adjacent content

File paths, directory trees, trait signatures, Cargo feature sets, and config structures all go in ` ``` ` blocks. Inline backticks are for short references (a single path, a single type name).

### Don't repeat what's in the code

If the code is clear — the trait is defined, the types are documented — the prose doc should say "X implements Y (see `path/to/file.rs`)" and explain _why_ Y exists, not restate the rustdoc.

### Be honest about unfinished work

Doc pages that describe an aspirational state without acknowledging current gaps create more confusion than omission. If a component is "planned but not yet implemented" or is scaffold-only, say so explicitly. Use the same format:

> Graph and curator crates are scaffold-level (sentinel tests only); deterministic hotspot scoring is the only production path today.

### Write for a skim-reader

Not every reader will read the full page. Use:

- **Bold** for the first mention of each key term
- Bullet lists for parallel items (crates, file locations, design principles)
- Numbered lists for sequential steps (consumption workflow, build path)
- Short paragraphs (3-5 lines max)

## Workflow: How to Write or Rewrite a Doc

### Step 1 — Assess what's stale

Read the current doc page. For every claim, cross-reference against the source code. Ask:

- Does this crate/trait still exist at this path?
- Does this Cargo feature still pull in this crate?
- Does this `view-id` still resolve to a view in `views.c4`?
- Is this diagram still an accurate representation of the topology?
- Are these cross-references valid?

Flag each discrepancy. Don't fix yet — just catalog.

### Step 2 — Identify what's missing

Read the relevant source code crates. Look for:

- Types defined but undocumented (new traits, new structs with important roles)
- Patterns used across multiple crates (Cargo feature composition, the guardrail boundary, adapter contracts)
- Directory structures that are non-obvious (generated code, the LikeC4 sources and webcomponent, mirror directories)

Ask: "If I had to explain this code to another developer without showing them the source, what would I need to tell them?" That's your missing content.

### Step 3 — Structure the rewrite

Using the principles above, sketch the page structure:

1. Architecture problem (one sentence)
2. Topology diagram (LikeC4 view on the architecture page)
3. Runtime flow diagram
4. Abstraction layer explanation
5. Table of components
6. Subsections for non-obvious mechanisms
7. Execution / verification details
8. Related pages

### Step 4 — Write the diagrams first

Diagrams force you to understand the topology and flow before you write prose. If you can't draw the diagram correctly, you don't understand the architecture well enough to write about it. On the architecture page, that means getting the `.c4` model right; iterate on the view until it's accurate — the prose will follow naturally.

### Step 5 — Write the "key insight" paragraph

This is the paragraph that begins "The key architectural separation..." or "The central design principle..." Write it immediately after the diagrams. Everything else in the page should support or elaborate on this insight.

### Step 6 — Fill in the rest

Write each section in order. Cross-reference related pages as you go. Pin file paths as you write.

### Step 7 — Verify every claim

Reread the doc. For each claim that describes behavior, confirm it against source code. If you wrote "scoring uses a weight table," check the table is real and current. If you wrote "`scryrs hotspots` reads `.scryrs/scryrs.db` via `TraceQuery`," confirm that path and type exist in `scryrs-core`.

### Step 8 — Rebuild and verify

Run `bun run build` in `.devagent/docs/`. This runs the LikeC4 codegen (regenerating `docs/public/likec4-webcomponent.js` from `.devagent/architecture/*.c4`) and then `rspress build`. Confirm:

- No dead links or build errors.
- `doc_build/llms.txt` and `doc_build/llms-full.txt` are generated (Rspress emits these because `rspress.config.ts` sets `llms: true`).
- Every `<likec4-view view-id="…">` in `architecture.mdx` resolves — a `view-id` with no matching view in `views.c4` produces an empty embed, not a build error, so check the view ids against `views.c4`.

## Related Pages

- [Project Docs Skill](../project-docs/SKILL.md) — how to _consume_ docs (ingestion workflow, llms.txt, golden rule of index-first)
