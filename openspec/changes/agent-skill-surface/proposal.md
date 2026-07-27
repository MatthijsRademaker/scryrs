## Why

scryrs's product loop is `Observe → Detect → Promote → Route` (`vision.md`). The first three arrows are built. The fourth has no consumer.

Every read-side surface — `scryrs hotspots`, `scryrs graph`, `scryrs route`, `scryrs route explain`, `scryrs route bundle` — emits JSON to stdout and waits for a human to type a command. Nothing tells an agent that these commands exist, when to prefer them over grepping, or how to read their output. The knowledge scryrs extracts from an agent's behavior never returns to an agent.

The evidence that this is real, not theoretical: this repository dogfoods scryrs and has `.scryrs/scryrs.db` (380 KB, 30 sessions) and `.scryrs/hotspots.json` (65 KB) — but **no `.scryrs/graph.json`, no `.scryrs/routes.json`, no `.scryrs/proposals/`, no `.scryrs/accepted/`**. scryrs has observed itself for a month and has never once routed itself. The read path has never been run here, because nothing in an agent's context ever suggests running it.

There is a second, sharper gap. `ProposalTargetType::Skill` already exists (`crates/scryrs-types/src/lib.rs:565`) and the curator already generates `skill` proposals for hotspot entries with failure outcomes (`crates/scryrs-curator/src/lib.rs:122`) — 13 entries qualify in the current store. But:

1. Nothing publishes them. `scryrs publish markdown` and `scryrs publish rspress` are the only adapters; both target docs trees, neither produces an agent-loadable skill.
2. The generated content is a stub, not a skill. It is four lines of metadata:
   ```
   # Skill: <subject>

   **Subject Kind**: file
   **Failure Count**: 2
   **Score**: 8
   ```
   No frontmatter, no `name`, no `description`, no routing trigger, no body. Publishing that as a `SKILL.md` would produce an artifact no agent could route to and no human would want.

So scryrs can already detect "agents keep failing on this subject" and can already file it as a skill proposal — and then drops it on the floor. Closing that loop turns scryrs from an analysis tool into a system whose output improves the agents that feed it.

## What Changes

This change delivers two surfaces. Part A is the consumer that makes existing retrieval reachable. Part B is the producer that closes the loop.

### Part A — A shipped, curated agent context skill

- Add a scryrs-authored skill (working name `scryrs-context`) whose job is to route an agent to scryrs retrieval **before** it starts grepping: query `scryrs route bundle` / `route explain` for a bounded context-loading plan, read the resulting `loadTarget` references, and fall back to search only when retrieval returns nothing.
- The skill instructs; it does not inject. The agent decides to invoke it. No hook, no prompt injection, no modification of agent-visible output — the observer-first boundary in `trace-hook-contract` is preserved exactly.
- The skill carries accurate CLI references for the read-side surface: `route bundle` (bounded planning), `route explain` (unbounded diagnostic ranking), `hotspots`, `graph`, `doctor`, and the prerequisite ordering between them (`graph` before `route`, `route` before `explain`/`bundle`).
- The skill teaches the `RouteHintItem` / `RouteBundleDocument` contract that consumers need in order to act on output: `loadTarget` kinds (`file`, `doc_page`, `non_loadable`), what `non_loadable` means and why `search` / `symbol` / `domain_term` / `doc_group` routes carry it, and how `rank` differs from explain `relevance`.
- The skill states its own preconditions honestly and tells the agent what to do when they are unmet — no `.scryrs/routes.json`, an empty hints array, or a stale manifest are all expected states, not errors to route around silently.
- `scryrs init --agent <NAME>` installs the skill into the harness's skill directory, alongside the trace hook. Installation stays idempotent and config-free, matching the current `init` contract.
- The skill is a curated artifact under `.pi/skills/CONVENTION.md`: `curated: true`, `sources` globs covering every path it makes claims about, `last-verified`, `verified-commit`. It must pass `scripts/skill-lint`. A skill that ships stale CLI references is worse than no skill, because a light-tier model cannot detect the drift.

### Part B — Accepted skill proposals publish as real skills

- Upgrade the curator's `skill` proposal content from the current metadata stub into a valid `SKILL.md` draft: YAML frontmatter with `name` and a routing `description`, plus a body grounded in the evidence that triggered it (the subject, the failure pattern, the co-occurring files, the evidence row ids).
- Add `scryrs publish skill <PATH> --output <DIR>` as a third publishing adapter, in a new `crates/scryrs-adapter-skill` crate, following the shape of `scryrs-adapter-markdown`: read `.scryrs/accepted/` only, write deterministically, never delete stale output, emit a JSON summary.
- Published output is one directory per skill containing a `SKILL.md` whose frontmatter is valid and whose `name` matches its directory — the structural contract a harness needs to load it.
- Publishing stays explicit. `scryrs proposals accept` does not publish, exactly as it does not today.

### The review gate is a pull request

- In the autonomous loop, the human gate is the PR, not an interactive review session. The loop runs `hotspots → graph → route → propose`, stages a decision, publishes, and opens a PR containing both the review-decision artifact and the generated `SKILL.md`. A human reviews the diff and merges or closes.
- The staged decision SHALL record the automating identity in `reviewer` (for example `scryrs-autonomous-loop`), never a human name. The artifact must not claim a human reviewed something no human has seen.
- Nothing reaches the default branch without the PR. `.scryrs/accepted/` inside a PR branch means "staged for review," and merge is the acceptance event.
- A generated skill lands as a proposal in the PR — it is never written directly into a live skill directory by the loop.

## Impact

- Agents gain a reason and a method to consume `route bundle` / `route explain`. The read path becomes reachable from inside a session instead of only from a human's shell.
- `scryrs init` grows a second responsibility (skill install) beyond hook install. The `init-installer` contract changes; `setup` remains the only command that writes `scryrs.json` and `.scryrs/`.
- `skill` proposals become publishable, so a third of the proposal kinds stop being dead ends.
- The curator's `skill` proposal content changes shape, which changes its deterministic content-addressed id. Existing `skill` proposal ids will not match — acceptable under AGENTS.md Rule 7, but it must be stated rather than discovered.
- A new workspace crate and a new Cargo feature, following the existing adapter feature split.
- `.scryrs/accepted/` acquires a second meaning in an automated context (staged-for-PR). This weakens the "review-first" framing and is a deliberate, documented trade-off, not an oversight.

## Dependency

This change is worth building only on a corpus that carries signal. `trace-signal-fidelity` fixes subject normalization, placeholder subjects, `FailedLookup` misclassification, and Claude Code outcome capture. Until it lands:

- Part A routes agents to a manifest built from evidence where 30 files are double-counted and 33% of subjects are absolute paths.
- Part B generates skills from `skill` proposals whose trigger — failure outcomes — is structurally impossible on Claude Code, because `PreToolUse` cannot observe an outcome.

Part A can be authored in parallel, since the CLI contract it documents is stable. Part B's generation quality depends directly on the signal fix and should follow it.
