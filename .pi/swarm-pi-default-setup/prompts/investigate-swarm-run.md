---
description: trace and diagnose a failed swarm agent run — from docker logs through gates, assignments, runs, and OpenCode sessions
---

# Swarm Run Forensics

Use this when a swarm task failed or produced unexpected results. You will trace the
full chain: task record → gate evaluation → agent assignments → agent runs →
OpenCode execution → evidence-backed verdict.

$ARGUMENTS

You will receive either:
- A `run_id` (agent run UUID) — trace that specific run
- A `task_id` (task UUID) — trace the full task lifecycle
- A container hostname + timestamp (e.g. `c9357a81-agent-swarm-reviewer-1 | 8:53PM`)
- A description of observed symptoms

---

## 0. Prerequisites

Before starting, confirm these are available:

- `docker` — to read container logs
- `psql` — for direct DB queries (optional but preferred; fall back to docker logs)
- The swarm project must be running locally

The project prefix is derived from the first 8 chars of the `.devagent/.env` UUID.
By default this is `swarm` (no prefix). Container patterns:
- Postgres:    `${PREFIX}-postgres`
- Manager:     `${PREFIX}-manager`
- Agents:      `${PREFIX}-agent-<def>-1` (e.g. `${PREFIX}-agent-swarm-worker-1`)

Recommended first step — use the convenience script:

```bash
scripts/inspect-task <PREFIX> <TASK_ID>
scripts/inspect-task <PREFIX> <TASK_ID> --deep   # includes OpenCode log file tails
```

This dumps 12 investigative sections in one shot. Use it as a starting point, then
follow the sections below for deeper analysis.

---

## 1. Collect Initial Evidence

### 1a. Identify the container prefix

```bash
cat .devagent/.env | grep PROJECT_ID | head -1
# ShortID = first 8 chars. If PROJECT_ID is unset, prefix is "swarm".
```

### 1b. Quick-reconstruct the timeline

```bash
# All log lines for a task, sorted chronologically
PREFIX=<prefix>
TASK_ID=<uuid>

echo "=== MANAGER ===" && docker logs ${PREFIX}-manager 2>&1 | grep "$TASK_ID" | head -50
echo "=== WORKER ===" && docker logs ${PREFIX}-agent-swarm-worker-1 2>&1 | grep "$TASK_ID" | head -50
echo "=== REVIEWER ===" && docker logs ${PREFIX}-agent-swarm-reviewer-1 2>&1 | grep "$TASK_ID" | head -50
echo "=== ARCHITECT ===" && docker logs ${PREFIX}-agent-swarm-architect-1 2>&1 | grep "$TASK_ID" | head -50
echo "=== LEAD-DEV ===" && docker logs ${PREFIX}-agent-swarm-lead-dev-1 2>&1 | grep "$TASK_ID" | head -50
```

### 1c. Query the database directly

```bash
PGPASSWORD=swarm docker exec -i ${PREFIX}-postgres psql -U swarm -d swarm -t -A
```

Useful queries:

```sql
-- Task state
SELECT id, status::text, title, assignee, branch, pr, created_at, updated_at
FROM tasks WHERE id = '<task_id>';

-- Gate instances (shows lifecycle transitions)
SELECT id, gate_status::text, state::text, on_pass::text, on_fail::text, created_at
FROM gate_instances WHERE task_id = '<task_id>'
ORDER BY created_at DESC;

-- Gate assignments (which agent produced what outcome)
SELECT agent_definition_id, outcome::text, is_required, completed_at
FROM gate_assignments WHERE task_id = '<task_id>'
ORDER BY created_at DESC;

-- Agent runs (all execution attempts)
SELECT id, agent_definition_id, outcome::text, trigger_status::text,
       claiming_agent_id, claimed_at, completed_at,
       round(extract(epoch from (completed_at - claimed_at))::numeric, 1) AS duration_sec
FROM agent_runs WHERE task_id = '<task_id>'
ORDER BY created_at DESC;
```

---

## 2. Trace the Failure Chain

Follow these questions in order. Each question tells you what to look for, where
to look, and how to interpret what you find.

### Q1: Where is the task now?

> `status` from the task record. If `NeedsWork` → a gate ended it there.
> If still `Review` or `ReadyForDev` → still in progress (or stuck).

| Status | Meaning |
|---|---|
| ReadyForDev | Worker is planning/implementing |
| Review | Gate review in progress (architect + lead-dev + reviewer) |
| NeedsWork | A gate failed — task needs investigation and rework |
| Done | Completed successfully |

### Q2: Which gate failed?

> `gate_status = 'failed'` in gate_instances. The `state` column tells you
> *which* gate failed (e.g. `ReadyForDev`, `Review`, `NeedsWork`).

Each gate has a policy that determines how it fails:

| Gate | Policy | Agents | Fails when |
|---|---|---|---|
| ReadyForDev | ExclusiveSuccess | worker | Worker != finished |
| Review | AllApproved | architect + lead-dev + reviewer | Any agent != approved |
| NeedsWork | ExclusiveSuccess | worker | Worker != finished |

**Critical insight for Review gate**: `AllApproved` is **fail-fast**. The moment
*one* agent reports a non-approved outcome (e.g. the reviewer fails with
`outcome=failed`), the gate terminates immediately — even if other agents are
still running. Their work is discarded.

### Q3: Which agent caused the failure?

> `outcome` in gate_assignments. The assignment with a failing outcome and
> `is_required = true` is the culprit.

Outcome meanings:

| Outcome | Meaning | Look for |
|---|---|---|
| `approved` | Agent explicitly approved | Normal review pass |
| `finished` | Agent completed work | Worker plan/implement done |
| `needs_work` | Agent requested changes | Review found issues |
| `failed` | Agent execution errored | Bug in harness, branch, or OpenCode |

### Q4: Did the agent error before or during OpenCode?

> Compare `claimed_at` vs `completed_at` in agent_runs.

**Same-minute failure** (duration < 60s): The agent errored *before* OpenCode
started. Common causes:

- `prepareTaskBranch` / `prepareReviewBranch` failed (git checkout/branch issue)
  — check agent container logs for "branch preparation failed"
- `generateTaskPrompt` failed (prompt provider missing or template error)
  — check agent logs for "prompt" or "template"
- Runner validation failed (command file not found)
  — check agent logs for "opencode command not found"
- Docker socket unavailable (Docker-in-Docker)
  — check for exit code 243

> **Docker-in-Docker clue**: If the architect has exit 243, it likely tried a
> `docker run` inside the container. The `--format json` output was then empty
> (container had no stdout), so `parseOutcomeFromJSON` returned nothing, and
> the harness warned "JSON output did not yield a parsed outcome."
>
> **Runtime boundary note**: Task-reactive agents run with hardcoded
> `SWARM_DIND_ENABLED=true` and use the internal daemon at
> `/var/run/dind/docker.sock`. Onboarding containers instead mount the host
> `/var/run/docker.sock` — DinD failure signatures (exit 243, daemon unreachable,
> socket not found) apply to task-reactive execution only, not onboarding.

**Long-running failure** (minutes): The agent errored *during* OpenCode execution.
Common causes:

- Model error / API failure — check OpenCode log files for error messages
- Tool call permission denied — check for `permission requested: ...; auto-rejecting`
- Timeout — check for "deadline exceeded" in agent logs
- Outcome artifact not written — check for "Outcome artifact missing" warning

### Q5: What happened inside OpenCode?

> Read the execution log files at `.devagent/logs/<task_id>-<suffix>-<timestamp>.log`
> inside the agent container. These contain the full OpenCode session output.

```bash
PREFIX=<prefix>
TASK_ID=<uuid>
AGENT=swarm-reviewer

# List log files for a task
docker exec ${PREFIX}-agent-${AGENT}-1 \
  ls -lat /home/devuser/workspace/project-source/.devagent/logs/ \
  | grep "$TASK_ID"

# Read the last N lines (outcome / error section)
docker exec ${PREFIX}-agent-${AGENT}-1 \
  tail -50 /home/devuser/workspace/project-source/.devagent/logs/<logfile>

# Or: get the full file
docker exec ${PREFIX}-agent-${AGENT}-1 \
  cat /home/devuser/workspace/project-source/.devagent/logs/<logfile> \
  > /tmp/swarm-openCode-session.log
```

Key patterns in OpenCode log files:

| Pattern | Meaning |
|---|---|
| `"type":"tool_use"` + `"status":"error"` | A tool call failed |
| `permission requested: ...; auto-rejecting` | OS permission was denied (e.g. `.devagent/tmp/`) |
| `exit 243` | Docker-in-Docker failure (no socket) |
| `exit 124` | `timeout` command killed the process |
| `No outcome artifact written` | Agent didn't call `report_work_outcome` tool |
| `outcome artifact not found: <run_id>` | Recovery prompt failed or was ignored |

### Q6: Was the gate evaluation raced?

> Check the **timeline** (chronological order) of events. Specifically:
> 1. When did each agent report its outcome?
> 2. When did the gate transition?
> 3. Did any agent report *after* the gate had already resolved?

Race condition pattern:

```
T+0:  Reviewer claims run → fails instantly (outcome=failed)
T+1:  Gate resolves → task → NeedsWork
T+2:  Architect reports outcome=failed OR finishes
       → This outcome is DISCARDED (gate already resolved)
       → Manager logs: "gate already resolved"
```

This is **wasted compute**. The architect and lead-dev are still running while
the gate terminates. Their work is thrown away. This happens because the
Review gate's `AllApproved` policy does not wait for pending agents.

---

## 3. Form Hypotheses

Combine evidence from Q1-Q6 into a hypothesis table.

| Confidence | Hypothesis | Evidence |
|---|---|---|
| high | Reviewer failed before OpenCode | Same-minute claim→fail, no OpenCode session ID in logs |
| high | Architect output unparseable | 125KB plain-text output, no JSON, Docker exit 243 |
| medium | Gate race wasted compute | Agent B's run completed after gate had already resolved |
| low | Prompt provider missing for reviewer | Not directly confirmed but plausible |

---

## 4. Expected vs Actual Behavior

Compare what the system did against what it should have done.

| Aspect | Expected | Actual |
|---|---|---|
| Reviewer executes OpenCode review | Agent starts OpenCode with PR diff | Fails before OpenCode (git/prompt error) |
| All 3 review agents complete | architect + lead-dev + reviewer all finish | Gate terminates early on first failure |
| Architect writes outcome artifact | JSON outcome via `report_work_outcome` tool | 125KB plain text, no artifact |
| Gate waits for pending agents | AllApproved collects all results | Fail-fast terminates immediately |

---

## 5. Produce a Verdict

Use this template:

```markdown
## Investigation Summary
[One paragraph: what was investigated, what was found.]

## Evidence Collected
| Source | Key Finding | File/Log Reference |
|---|---|---|
| agent_runs | Reviewer failed in 12s (same-minute) | run_id=<uuid>, outcome=failed |
| docker logs | No OpenCode session between claim and fail | `<prefix>-agent-swarm-reviewer-1` |
| docker logs | Docker exit 243 during architect run | `<prefix>-agent-swarm-architect-1` |
| OpenCode log | `.devagent/tmp/` permission auto-rejected | `<logfile>` line <line> |

## Root Cause
[What broke and at what layer: runner, harness, model, gate, artifact.]

## Contributing Factors
- [e.g., AllApproved fail-fast wastes concurrent agent work]
- [e.g., classifyRunError always returns Transient, unused by gate]

## Recommendations
1. [Backend fix: e.g., fix prepareReviewBranch for missing branches]
2. [Gate fix: e.g., wait for pending agents before terminating]
3. [Diagnostic fix: e.g., add logging for prepareTaskBranch failures]
```

## Rules
- Every claim must cite evidence (log line, DB row, file:line).
- Label confidence: high / medium / low.
- Do not fix code — this is investigation only.
- Hypotheses are cheaper than certainty; label gaps.
- If evidence is insufficient, state what's missing.
