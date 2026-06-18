---
description: self-improve worker execution prompts using the eval loop
agent: build
swarm: true
agent_types:
  - swarm-worker
---

<context>
You are running a self-improvement loop for the swarm worker execution flow.
Your goal: improve the quality of worker outputs by iterating on prompt files under `.opencode/`.
You operate through the evaluation framework at `evaluation/`. Never touch `src/cli/scaffolding/`.
</context>

<request>
$ARGUMENTS
</request>

<task>
Run a bounded improvement loop (max 2 iterations) to improve worker execution prompts.
Start from a baseline experiment, identify failure patterns in worker plan-review-execute flows,
create minimal prompt variants, validate them through the eval suite, and optionally promote
winning changes into `.opencode/`.
</task>

<pre_work>
1. Load the `swarm-eval-loop` skill. It contains the heuristics, failure-to-surface mappings,
   stop conditions, candidate creation rules, promotion rules, and output analysis guidance
   needed at every step.
2. Read `evaluation/experiments/README.md` to understand the experiment framework if this
   is your first time.
</pre_work>

<workflow>
<step number="1" label="Prepare baseline">
Run the baseline experiment on the specified tasks. If <request> specifies task IDs, use
`--workflow-task <task-id>` for each. If <request> specifies a dataset path, use
`--fixture <path>`. Default to `v1-baseline` as the baseline experiment name.

The eval run command is:
```
scripts/eval-run --experiment v1-baseline --workflow-task <task-id> --keep-workspace --mlflow-uri ""
```

If <request> contains no specific task IDs or dataset, default to the task that already shows
known worker-flow failures in the eval outputs. Start with at most 2-3 tasks.
</step>

<step number="2" label="Analyze baseline failures">
Read the most recent baseline outputs:
- `evaluation/outputs/experiment_v1-baseline_*.json` (machine-readable)
- `evaluation/outputs/experiment_report_v1-baseline_*.txt` (human-readable)

Use the skill's heuristics to:
- Identify all tasks classified as "worse"
- For each worse task, extract reviewer issues from `reviewer_parsed.issues[]`
- Read `worker_output` to understand root cause
- Map each failure to the specific `.opencode` prompt surface that controls it
- Choose the highest-priority surface affecting the most tasks that can be fixed without
  introducing risk to currently passing tasks
</step>

<step number="3" label="Create candidate variant (iteration 1)">
Create a candidate experiment. Follow the skill's candidate creation rules exactly:

1. Choose a variant name: `worker-v1-<short-description>` (kebab-case, max 40 chars).
2. Create `evaluation/experiments/prompts/<variant>/<filename>` by copying the current
   live `.opencode` file and applying the minimal edit.
3. Create `evaluation/experiments/manifests/<variant>.json` referencing only the changed files.

Rules:
- Only change the prompt surface(s) the failure analysis identified.
- Change the smallest number of lines that address the failure.
- Never add instructions that duplicate what another surface already says.
- Prefer removing redundant instructions over adding new ones.
- Do not touch any file in `src/cli/scaffolding/`.
</step>

<step number="4" label="Validate candidate (iteration 1)">
Run the candidate experiment on the same tasks:
```
scripts/eval-run --experiment <variant> --workflow-task <task-id> --keep-workspace --mlflow-uri ""
```

Compare baseline vs candidate using the skill's comparison rules:
- Read both JSON output files
- Compare per-task scores
- Check for regressions (any task with lower outcome_agreement, new critical issues, or errors)

Stop if:
- No improvement in avg_total_score
- Any regression detected
- All tasks are now "equivalent" or better
</step>

<step number="5" label="Promote or iterate (iteration 1 decision)">
If the candidate improved with no regressions and all worse-tasks became better:
  → Go to step 7 (Finalize).

If the candidate improved with no regressions but some tasks remain "worse":
  → Proceed to step 6 (Iteration 2).

If the candidate did not improve or introduced regressions:
  → Go to step 8 (Report failure).

If max_iterations (2) would be exceeded by further iteration:
  → Go to step 8 (Report outcome).
</step>

<step number="6" label="Iteration 2">
Re-classify remaining failures using the skill's heuristics. Some failures may have been
fixed by iteration 1's changes.

Choose the next-highest-priority prompt surface not yet modified in this session.
Create a NEW variant (e.g. `worker-v2-<description>`) — do not mutate the iteration-1 variant.

Repeat steps 3-4 with this new variant. Compare against iteration 1 results, not baseline.

If iteration 2 improves but tasks still worse → report and stop. Do not exceed max_iterations=2.
</step>

<step number="7" label="Finalize — promote winning prompts">
For each variant that showed improvement with no regressions:

1. Read the variant prompt file(s) from `evaluation/experiments/prompts/<variant>/`.
2. Apply the exact same changes to the corresponding live file(s) under `.opencode/`.
   Use `edit` for surgical changes — do not wholesale replace the file.
3. For each promoted file, output:
   ```
   PROMOTED: .opencode/commands/<filename>
     Baseline score: X.XXX
     Candidate score: X.XXX
     Reason: <why this change improved the replay>
   ```
4. Do NOT touch any file under `src/cli/scaffolding/`.

After all promotions, run a final verification: re-run the baseline experiment (which now
uses the updated `.opencode` files) on the same tasks to confirm the baseline now passes.
</step>

<step number="8" label="Report outcome">
Emit a final summary:

```
SELF-IMPROVE WORKER SUMMARY
  Iterations completed: N
  Tasks evaluated: N
  Prompt surfaces changed: <list of files>
  Baseline avg score: X.XXX
  Final avg score: X.XXX
  Tasks improved: N
  Tasks regressed: N
  Tasks unchanged: N
  Candidate variants created:
    evaluation/experiments/prompts/<variant>/
    evaluation/experiments/manifests/<variant>.json
  Files promoted to .opencode/: <list or "none">
  Remaining worse tasks requiring manual review: <list of task IDs or "none">
```
</step>
</workflow>

<constraints>
- max_iterations = 2. Never exceed.
- Target `.opencode/commands/*.md` and `.opencode/agents/*.md` only.
- Never touch `src/cli/scaffolding/`.
- Never touch `src/agents/prompts/task-prompts/`.
- Never run the full dataset without explicit instruction — default to 2-3 specific tasks.
- Never change the eval framework itself (Python code, scoring model, workflow mimic).
- Every prompt change must be traceable to a specific failure in the eval output.
- Do not start a new iteration without analyzing results from the previous one.
</constraints>

<output>
The final `SELF-IMPROVE WORKER SUMMARY` block as described in step 8, plus the individual
`PROMOTED:` blocks from step 7.
</output>
