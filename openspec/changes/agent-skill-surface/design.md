## Context

The loop today, with the break marked:

```
  agent tools ──▶ hook ──▶ TraceEvent ──▶ store          AUTONOMOUS
                                            │
        hotspots ─▶ graph ─▶ route ─▶ propose            human types it
                                            │
                          proposals accept                human reviews
                                            │
                    publish markdown|rspress              human types it
                                            │
                                            ▼
                                      docs tree
                                            │
                                      ╳ nothing reads it back
```

This change adds the missing return arrow twice:

```
  Part A (consume):   agent ──▶ scryrs route bundle ──▶ loads the right files
                        ▲                                      │
                        └──────── skill instructs ─────────────┘

  Part B (produce):   skill proposal ──▶ publish skill ──▶ SKILL.md ──▶ PR ──▶ merge
                                                                         │
                                                        agent's future context ◀┘
```

Relevant existing surface, verified at commit `d7d903c`:

- `scryrs route bundle <PATH> --query <TEXT> --limit <N>` — bounded context plan, `RouteBundleDocument`, read-only over `.scryrs/routes.json`.
- `scryrs route explain <PATH> --query <TEXT>` — unbounded diagnostic ranking, `RouteHintDocument`.
- `RouteHintItem` carries `routeId`, `target` (stable graph node id), optional `loadTarget` (`file` | `doc_page` | `non_loadable`), `label`, `rank`, evidence citations, and a template-derived `reason`.
- Publishing adapters live in `crates/scryrs-adapter-markdown` and `crates/scryrs-adapter-rspress`, both behind Cargo features, both dispatched from `crates/scryrs-cli/src/publish.rs`, both reading `.scryrs/accepted/` only.
- `.pi/skills/CONVENTION.md` defines the curated-skill contract; `scripts/skill-lint` enforces a path-accuracy floor.
- CI is `.github/workflows/ci.yml`, `hook-ci.yml`, `release.yml`.

## Goals / Non-Goals

**Goals:**
- An agent in a session has a reason and a method to consume scryrs retrieval before it searches.
- Accepted `skill` proposals become loadable skill artifacts.
- Generated skill content is good enough that a human wants to merge it.
- The human gate is a PR diff.
- The observer-first boundary is untouched.

**Non-Goals:**
- No prompt injection, no `UserPromptSubmit` hook, no automatic context loading. Recorded as a spike in `roadmap.mdx`; explicitly out of scope here.
- No MCP server. See D1.
- No semantic ranking or embedding retrieval — retrieval stays deterministic manifest matching (Phase 8 boundary).
- No LLM in the generation path. `scryrs-curator-llm` stays library-only and off the deterministic path (Phase 9 boundary).
- No auto-merge. The PR is a gate, and a gate a human never opens is not a gate.
- No changes to `route explain` / `route bundle` ranking, schema, or output.

## Decisions

### D1. A skill, not an MCP server

| | Skill + CLI references | MCP server |
|---|---|---|
| Context cost | progressive — loads when triggered | tool definitions resident every session |
| Process model | binary already on `PATH` | server lifecycle, transport, port |
| Output shape | CLI already emits single-line JSON, and `--help-json` self-describes | re-wraps what is already machine-readable |
| Repo precedent | `.pi/skills/CONVENTION.md` + `scripts/skill-lint` exist | zero MCP anywhere in the tree (verified) |
| Drift detection | `verified-commit` + `sources` globs | none |
| Harness reach | one `SKILL.md` read by Claude Code, Pi, and others | per-client configuration |

The decisive point is that MCP's value is making an unaddressable service addressable. scryrs is a binary on `PATH` emitting deterministic single-line JSON with a `--help-json` self-description. An MCP server would be a re-encoding tax over a contract that is already machine-readable.

There is also a boundary argument worth stating rather than drifting past. `hooks/pi/index.ts:17`: *"This hook never registers scryrs as an agent-callable tool, never modifies agent-visible tool results, and fails open."* A skill preserves that — the agent chooses to shell out and scryrs stays an observer. An MCP server would make scryrs an agent-callable tool surface. Not fatal, and a separate artifact from the hook, but a line to cross deliberately if ever.

### D2. The skill instructs; it does not inject

An injecting hook (`UserPromptSubmit` adding route hints unprompted) is more autonomous and more reliable — it does not depend on a model choosing to invoke anything. It is also exactly the "runtime optimizer" direction that `roadmap.mdx` defers until the observer chain is complete and stable, and it edges toward modifying agent semantics, which `trace-hook-contract` forbids.

Decision: instruct. The cost is honest and should not be minimized — **a skill that instructs can be ignored.** If the retrieval surface is genuinely useful, invocation rates will show it; if agents ignore it, that is information about the surface, not a reason to force it. The injection path is recorded as a spike in `roadmap.mdx` and can be revisited with real invocation data.

### D3. The skill must be curated, or it will lie

`.pi/skills/CONVENTION.md` states the reason directly: curated skills are "consumed as ground truth by lighter-tier swarm agents," and "a light model cannot detect a stale skill — it will confidently follow it into files that no longer exist."

A skill full of CLI references is the highest-drift-risk skill in the repository: every flag, subcommand, and output field is a claim that a CLI change can falsify. So the skill takes the full curated contract — `curated: true`, `sources` globs covering `crates/scryrs-cli/**` and the route/hint spec surface, `last-verified`, `verified-commit` — and must pass `scripts/skill-lint`.

Corollary: the skill must be reachable by `/curate-skills` refresh passes, and any change to the route CLI surface must be treated as invalidating it.

### D4. The skill teaches the contract, not just the commands

A list of commands is not enough to act on. An agent that runs `route bundle` and receives `"loadTarget": {"kind": "non_loadable"}` needs to know that this is a correct, expected result for `search` / `symbol` / `domain_term` / `doc_group` routes — not a bug, not something to retry, and not something to work around by grepping the target string.

So the skill covers:
- **Prerequisite ordering.** `graph` writes `.scryrs/graph.json`; `route` consumes it and writes `.scryrs/routes.json`; `explain` and `bundle` are read-only over that manifest. Running `bundle` first fails, and the skill must say so rather than letting the agent discover it.
- **bundle vs. explain.** Bundle is the bounded planning surface; explain is the unbounded diagnostic ranking surface. An agent loading context wants bundle. Getting this backwards means either an unbounded read or a truncated diagnosis.
- **`loadTarget` kinds.** `file` → repository-relative path, read it. `doc_page` → `project-docs/<slug>`. `non_loadable` → there is nothing to read; the route is a concept, not a location.
- **`rank` vs. `relevance`.** `rank` is the manifest ordinal. `relevance` is explain's packed score and is absent from plain route projection. An agent that treats them as interchangeable will mis-order its reading.
- **Unmet preconditions as normal states.** No manifest, an empty `hints` array, or a manifest older than the working tree are all expected. The skill states what each means and what to do — which, for an empty result, is: fall back to search, and that fallback is itself the signal scryrs wants to capture.

### D5. Skill install belongs in `init`, and only the skill

`scryrs init --agent <NAME>` already installs the trace hook per harness and is documented as idempotent and config-free: "it never writes `scryrs.json` or `.scryrs/`." Installing a skill fits that contract — a skill is a static file, no configuration, no network, idempotent to rewrite.

`setup` stays the only command that writes `scryrs.json` and the `.scryrs/` scaffold. That separation was the entire point of splitting `init` and `setup`, and this change does not blur it.

Open sub-question for task work: whether the skill installs into a shared location or a harness-specific one, and how a locally-modified installed skill is handled. The hook precedent (AGENTS.md "Pi Hook Source Ownership") is a canonical source in `hooks/` plus a gitignored installed copy that agents must not edit. The skill should follow the same ownership model: canonical source in the repository, installed copy explicitly non-canonical.

### D6. Generated skill content must be a real SKILL.md, or Part B is pointless

The current generated content (`scryrs-curator/src/lib.rs:134`):

```
# Skill: crates/scryrs-cli/src/init.rs

**Subject Kind**: file
**Failure Count**: 2
**Score**: 8
```

No harness can route to this — no `name`, no `description`, no frontmatter. And no reviewer would merge it, because it says nothing a `git log` would not.

The generated draft must carry:
- **Frontmatter** with `name` (directory-matching, kebab-case) and a `description` written as a routing trigger — the field a model actually reads to decide whether to load the skill.
- **A body grounded in the evidence**: the subject, the failure pattern that triggered generation, the co-occurring files from the same sessions, and the evidence row ids so a reviewer can verify the claim.
- **Provenance** marking it machine-generated from a proposal id, so it is never mistaken for a hand-curated skill.

Accepted consequence: content-addressed proposal ids change, so existing `skill` proposal ids will not match. Fine under Rule 7, but stated here rather than discovered during review.

Honest limitation: a deterministic generator can produce a well-formed, evidence-grounded skill. It cannot produce a *good* skill — the actual recipe knowledge ("when you hit this, do that") is not in the trace data. So the generated artifact is best understood as **a well-formed stub with the evidence pre-assembled for a human to write into**. That is genuinely valuable — it puts the right file in front of the right person with the citations already gathered — but the proposal must not oversell it as an authored skill. This is also precisely the bounded drafting use `scryrs-curator-llm` exists for, and a reasonable later thread.

### D7. The PR is the gate, and the reviewer field must not lie

`publish` reads `.scryrs/accepted/` only. An autonomous loop therefore has to write an accepted artifact before it can publish, which means something must "accept" before a human has looked.

The resolution is to keep the artifact honest about who acted:

```
  loop: hotspots → graph → route → propose
             │
             ├─ accept with reviewer = "scryrs-autonomous-loop"   ← staged, not human-approved
             ├─ publish skill → SKILL.md
             └─ open PR containing BOTH artifacts
                        │
                        ▼
              human reviews the diff
                   merge = acceptance
                   close = rejection
```

Rules that make this safe:
- `reviewer` records the automating identity, never a human name. The artifact says "an automation staged this," which is true.
- Both the review-decision artifact and the generated `SKILL.md` land in the same PR, so a reviewer sees the claim and the output together.
- Nothing reaches the default branch without the PR. `.scryrs/accepted/` on a PR branch means staged-for-review; merge is the acceptance event.
- The loop never writes into a live skill directory. It writes into the repository as a proposed diff.
- No auto-merge. A gate nobody opens is not a gate.

**Stated cost:** this weakens the review-first framing that the proposal-review contract was built around. `.scryrs/accepted/` now means "human-accepted" in the interactive path and "automation-staged" in the loop path — one directory, two meanings, disambiguated only by the `reviewer` field. That ambiguity is the price of reusing the existing publish contract instead of forking it, and it is worth flagging to a reviewer explicitly. The alternative — a `--from-pending` publish path — avoids the ambiguity but forks the publish contract and lets pending artifacts reach a docs surface, which is worse.

### D8. A separate crate for the skill adapter

`scryrs-adapter-markdown` and `scryrs-adapter-rspress` are separate crates behind separate Cargo features, dispatched from one `publish.rs`. A third adapter follows that shape: `crates/scryrs-adapter-skill`, its own feature, `publish_accepted_skill`, `SkillPublishSummary`, dispatched alongside the other two.

Consistency is the whole argument — the `add-adapter` skill documents this recipe and a reviewer will expect it followed.

## Risks / Trade-offs

- **An instructing skill can be ignored.** The core risk of D2. Mitigation is a sharp `description` and honest preconditions; measurement is invocation rate. If agents ignore it, that finding matters more than the skill does.
- **The skill is the highest-drift artifact in the repo.** Every CLI reference is falsifiable by a CLI change. Mitigated by the curated contract, `skill-lint`, and treating route-CLI changes as invalidating.
- **Generated skills may be low-value.** D6 is explicit that the generator produces evidence-assembled stubs, not authored skills. The PR gate is what keeps low-value output from landing — which means review load is real, and a loop that opens noisy PRs will be turned off.
- **`.scryrs/accepted/` carries two meanings.** D7, stated cost, not mitigated away.
- **Circularity worth naming:** the skill tells agents to run scryrs commands, and the hook records those commands as trace events. Verify that scryrs's own retrieval calls do not pollute the hotspot corpus they read from.
- **Part B on unfixed signal generates skills from noise.** Hence the `trace-signal-fidelity` dependency.

## Migration Plan

1. Author the Part A skill against the current CLI surface; verify every reference by running the command. Curated frontmatter, `skill-lint` green.
2. Wire skill install into `scryrs init --agent <NAME>`; settle the ownership model per D5.
3. Land `trace-signal-fidelity` before judging Part B output quality.
4. Upgrade curator `skill` proposal content to a valid `SKILL.md` draft (D6).
5. Add `crates/scryrs-adapter-skill` and `scryrs publish skill`, following `add-adapter`.
6. Wire the loop and the PR gate (D7); confirm `reviewer` is the automation identity and no auto-merge exists.
7. Dogfood: run the full loop on this repository and read the resulting PR as a reviewer would. If the PR is not worth merging, the generator is not done.

## Open Questions

- Shared or harness-specific skill install location, and what happens to a locally-modified installed copy? (D5)
- Does the skill tell agents to run `graph`/`route` themselves when the manifest is missing, or only report the precondition? Running them writes artifacts, which is a side effect inside a read-oriented skill.
- Should scryrs's own retrieval invocations be excluded from capture to avoid self-pollution?
- Who runs the loop — CI on a schedule, or a human-triggered command? Determines whether PR volume is bounded.
- Does `publish skill` output into `.claude/skills/` and `.pi/skills/` directly, or into a neutral staging directory a reviewer moves?
