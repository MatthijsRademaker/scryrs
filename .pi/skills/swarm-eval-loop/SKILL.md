---
name: swarm-eval-loop
description: Heuristics and mapping for the self-improving worker eval loop. Use when running swarm-self-improve-worker, analyzing experiment outputs, classifying worker execution failures, creating prompt variants, or promoting winning prompts into .opencode/. Do not load for general eval questions — only when driving the improvement loop.
---

# Swarm Eval Loop

This skill provides the stable heuristics, failure-to-surface mappings, stop conditions, and promotion rules for the `swarm-self-improve-worker` command. The command drives the workflow; this skill holds the reasoning you need at each step.

## Architecture

### Two-layer prompt system

| Layer | Path | When to change |
|---|---|---|
| Staging (experiment variants) | `evaluation/experiments/prompts/<variant>/` | Candidate prompt edits before validation |
| Live (production prompts) | `.opencode/commands/*.md`, `.opencode/agents/*.md` | After an experiment shows improvement |
| Scaffolding (source templates) | `src/cli/scaffolding/` | **Do not touch** — user reviews first |

### Target prompt surfaces (priority order)

| Priority | File | Controls |
|---|---|---|
| 1 (highest) | `.opencode/commands/opsx-apply.md` | Implementation scope, pre-edit inspection, verification, subagent delegation |
| 2 | `.opencode/commands/opsx-archive-auto.md` | Archive automation, delta spec sync, verification |
| 3 | `.opencode/commands/swarm-execute.md` | Context-driven rework execution |
| 4 (lowest) | `.opencode/agents/swarm-worker.md` | Cross-phase behavior: agency level, delegation rules, verification discipline |

**Rule**: Change phase-specific commands before changing the general worker prompt. Only touch `swarm-worker.md` when the same failure pattern repeats across multiple phases.

### Source-of-truth rule

For command-backed single-state workflows, `.opencode/commands/swarm-*.md` is the authoritative source for behavioral instructions and output schema. `src/agents/prompts/task-prompts/*.md` should stay context-only and should NOT duplicate process steps, JSON schemas, or `report_work_outcome` instructions. **Do not edit task prompt templates as part of this loop.**

## Failure-to-Surface Mapping

Use this table to decide which `.opencode` file needs changing based on what went wrong in a replay:

| Symptom | Likely surface | Change direction |
|---|---|---|
| Plan missed required task domains | `opsx-apply.md` | Strengthen implementation criteria: must cover all task requirements listed in the task request |
| Implementation was too verbose / wasted tokens | `opsx-apply.md` | Remove redundant sections, tighten output format |
| Worker made changes outside task scope | `opsx-apply.md` | Strengthen scope rules; require explicit "in scope / out of scope" check before editing |
| Worker skipped required files | `opsx-apply.md` | Require full file list derived from the OpenSpec change + task request before editing begins |
| Worker introduced mechanical errors (indentation, duplicate calls) | `opsx-apply.md` | Strengthen verification requirements: typecheck/lint during implementation |
| Worker touched unrelated config files | `opsx-apply.md` | Add explicit "do not touch files outside the scope" rule |
| Worker changed model config or opencode settings | `opsx-apply.md` | Add explicit "do not modify opencode.jsonc, agent configs, or provider settings" rule |
| Review failed to catch a weak implementation | `opsx-apply.md` | Require explicit per-requirement verification in implementation output |
| Good implementation but reviewer still rejected with valid reasons | `opsx-apply.md` | The worker didn't follow the OpenSpec change faithfully |
| Across multiple phases: worker delegates poorly or refuses to use subagents | `swarm-worker.md` | Adjust subagent delegation rules |
| Across multiple phases: worker overscopes consistently | `swarm-worker.md` | Add stronger scope boundary rules |

## Stop Conditions

The loop stops on ANY of these:

1. **`max_iterations` reached** — default 2, hard cap. Never exceed this without explicit user instruction.
2. **No score improvement** — `avg_total_score` of candidate ≤ `avg_total_score` of baseline (or previous iteration).
3. **Outcome agreement regression** — `outcome_agreement` for any task drops below baseline. A prompt fix must never make a previously matched task worse.
4. **New critical issues** — any task in the candidate run gains a `critical` severity issue that was not present in the baseline.
5. **All tasks become `equivalent` or better** — no "worse" classifications remain. The loop has succeeded.
6. **Errors or timeouts** — any task in the candidate run fails with `error` or `timeout`. A prompt edit must not break the execution path.

## Candidate Creation Rules

### Naming conventions

- Variant name: `worker-v<N>-<short-description>` (e.g. `worker-v3-scope-tighten`)
- Prompt files go under: `evaluation/experiments/prompts/<variant-name>/`
- Manifest goes at: `evaluation/experiments/manifests/<variant-name>.json`

### Manifest template

```json
{
  "name": "worker-v3-scope-tighten",
  "description": "<one-line description of what changed and why>",
  "prompt_variant": "worker-v3-scope-tighten",
  "workflow_mimic": "workflows/worker-opsx-apply.json",
  "taskflow_mimic": "taskflows/default.json",
  "opencode_config": "opencode/eval-default.opencode.jsonc",
  "prompt_overrides": {
    ".opencode/commands/swarm-execute.md": "prompts/worker-v3-scope-tighten/swarm-execute.md"
  }
}
```

Use the existing `v1-baseline` manifest as the baseline for comparison.

### File creation process

1. Copy the current live `.opencode` file to `evaluation/experiments/prompts/<variant>/<filename>`.
2. Apply the minimal edit to the copy — change only what addresses the failure.
3. Write the manifest with only the changed files in `prompt_overrides`.
4. Never override files that weren't changed.

### Minimal edit discipline

- Change the smallest number of lines needed.
- Never add instructions that duplicate what another prompt surface already says.
- Never add speculative rules for problems you haven't seen in the data.
- Prefer removing redundant instructions over adding new ones.
- Do not restructure the prompt beyond what the failure demands.

## Run and Compare

### Execute the experiment

```bash
scripts/eval-run --experiment <candidate-name> --workflow-task <task-id> --keep-workspace --mlflow-uri ""
```

Use `--workflow-task` to target only the tasks that were "worse" in the baseline. Use `--keep-workspace` so you can inspect the replay workspace if needed.

### Compare baseline vs candidate

Read both JSON output files (sorted by timestamp, most recent):

```
evaluation/outputs/experiment_<baseline>_<ts>.json
evaluation/outputs/experiment_<candidate>_<ts>.json
```

Compare per-task:
- `outcome_agreement` — must not drop below baseline
- `code_evaluation_agreement` — must not drop below baseline
- `issue_agreement` — higher is better
- `total` — weighted composite score
- `classification` (from the text report) — should move from "worse" toward "equivalent" or "potentially_better"

Also read the text reports:
```
evaluation/outputs/experiment_report_<baseline>_<ts>.txt
evaluation/outputs/experiment_report_<candidate>_<ts>.txt
```

### Decision table

| Candidate vs Baseline | Action |
|---|---|
| All worse-tasks now equivalent or better, no regressions | Promote to `.opencode/` |
| Some worse-tasks improved, no regressions, but some still worse | Iterate again (if under max_iterations) |
| Any task regressed (lower score, new critical issues, lower outcome_agreement) | Discard candidate, report regression |
| No change in scores | Stop — prompt change had no effect |
| Errors or timeouts introduced | Discard candidate, report breakage |

## Promotion Rules

When a candidate shows improvement and no regressions:

### How to promote

1. Copy the exact winning variant prompt file content into the corresponding live `.opencode` file.
2. Use `edit` tool for surgical changes rather than `write` (preserves file metadata).
3. Verify the live file diff makes sense before committing.

### What to promote

- Only the files that were in the variant's `prompt_overrides`.
- Do not promote unchanged companion files.
- Do not touch any files in `src/cli/scaffolding/`.

### What to report after promotion

After each successful promotion, output:

```
PROMOTED: .opencode/commands/<name>.md
  Baseline score: X.XXX
  Candidate score: X.XXX
  Reason: <why this prompt change improved the replay>
```

## Iteration Loop (max_iterations=2)

```
Iteration 1:
  1. Run baseline (v1-baseline) on specified tasks
  2. Identify "worse" tasks and their failure patterns
  3. Classify each failure to a prompt surface
  4. Choose the highest-priority surface affecting the most tasks
  5. Write a minimal prompt edit
  6. Create experiment variant + manifest
  7. Run candidate experiment on the same tasks
  8. Compare baseline vs candidate

If improved and tasks still "worse" remain → Iteration 2:
  9. Re-classify remaining failures (some may have been fixed by iteration 1)
  10. Choose next-highest-priority surface
  11. Create a NEW variant (do not mutate the iteration-1 variant)
  12. Run against the same tasks
  13. Compare against iteration 1 (not baseline)

If iteration 2 also improves but tasks still "worse" → report and stop.
  Do not exceed max_iterations.
```

## Heuristics for Analyzing Eval Outputs

### Reading the JSON artifact

The machine-readable output at `evaluation/outputs/experiment_<name>_<ts>.json` contains:

- `scores[]` — per-task score breakdown (`outcome_agreement`, `code_evaluation_agreement`, `issue_agreement`, `total`)
- `results[]` — per-task details including `worker_output`, `reviewer_output`, `reviewer_parsed`, `code_diff`, `workspace_path`

### Reading the text report

`evaluation/outputs/experiment_report_<name>_<ts>.txt` contains classifications and inspection flags. Tasks marked `worse` need prompt changes. Tasks marked `potentially_better` need manual review but are not blocking.

### Classifying a failure from reviewer issues

Look at `reviewer_parsed.issues[]` in the JSON output. Key fields:

- `severity`: `critical` (must fix), `warning` (should fix), `info` (nice to have)
- `category`: `logic` (bug), `alignment` (scope/requirement mismatch), `style` (formatting), `security` (vulnerability)
- `file`: which file the issue is about
- `description`: what went wrong
- `suggestion`: how to fix it

**Alignment issues** (scope creep, missing requirements) → `opsx-apply.md` scope rules.
**Logic issues** (bugs, duplicates, broken code) → `opsx-apply.md` verification rules.
**Style issues** (formatting, indentation) → `opsx-apply.md` verification rules (add `go fmt` or equivalent after edits).
**Multiple issues across unrelated files** → `opsx-apply.md` scope discipline.

### Reading worker_output for root cause

The `worker_output` field in the JSON shows what the worker actually did. Look for:

- Did the worker read the full task request before editing? If not → strengthen pre-edit requirements.
- Did the worker check which files exist vs which need change? If not → strengthen inspection requirements.
- Did the worker run verification commands? If not and errors were introduced → strengthen verification requirements.
- Did the worker delegate to specialists? If not and domain-specific errors occurred → strengthen delegation rules.
- Did the worker touch files outside the plan? If yes → add explicit scope boundary rules.

## Verification After Promotion

After promoting a prompt change to `.opencode/`, verify the change is correct:

1. Re-run the same experiment with the baseline manifest (which now uses the updated live `.opencode` file).
2. Confirm the baseline task now passes or improves.
3. If other tasks regressed, the prompt change was too broad — revert and narrow.

## Files You May Read

| File | Why |
|---|---|
| `evaluation/outputs/experiment_<name>_<ts>.json` | Machine-readable scores, reviewer issues, code diffs |
| `evaluation/outputs/experiment_report_<name>_<ts>.txt` | Human-readable classification and inspection flags |
| `evaluation/experiments/manifests/v1-baseline.json` | Baseline manifest template |
| `evaluation/experiments/workflows/worker-opsx-apply.json` | Worker workflow state machine reference |
| `.opencode/commands/opsx-apply.md` | Plan phase prompt |
| `.opencode/commands/opsx-archive-auto.md` | Plan review/save prompt |
| `.opencode/commands/swarm-execute.md` | Plan execution prompt |
| `.opencode/agents/swarm-worker.md` | Worker system prompt |
| `evaluation/experiments/README.md` | Experiment framework docs |
