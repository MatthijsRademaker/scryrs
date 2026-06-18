---
description: gather evidence for a manual task intervention — what is still missing, what is the way forward
---

Inspect the repository state for a task that needs manual intervention and produce a structured
"what is still missing" report for a human operator.

$ARGUMENTS

## Scope

`$ARGUMENTS` must be a task ID (UUID or first 8 characters). If `$ARGUMENTS` is empty, print this
fatal message and stop:

```
manual-intervene: missing task ID. Usage: /manual-intervene <task-id>
```

Do not continue. Do not infer, guess, or search for a task ID.

## Evidence Retrieval Order

Gather evidence in this order, collecting from each source without overwriting earlier sources.
Report which sources contributed and which were unavailable.

### 1. Local Task Artifacts

Search for files matching the task ID or its first 8 characters in:

| Root | Expected match pattern |
|---|---|
| `.devagent/logs/` | `*<task-id>*` log files |
| `.agents/TODO/` | `<task-id>-*` plan and task files |
| `.agents/HANDOFFS/` | `<task-id>-*` handoff files |

For any artifact files found, extract:

- The task title and status from plan files under `.agents/TODO/`
- Saved prompt-context files under `.devagent/logs/` that contain `### Task Comments` — parse each
  comment from the saved markdown, preserving these fields when present: `Author`, `Source`,
  `Created At`, and `Agent Run ID`. WARNING: saved task context is bounded to 10 comments and
  32 KB total by `src/agents/taskcontext/context.go`, so artifact-derived comments may be
  incomplete.
- Handoff notes from `.agents/HANDOFFS/` files

If no local task artifacts are found, note this but continue to the next source.

### 2. Live Manager + Database (Best-Effort)

If the local swarm stack is running, retrieve live task data via HTTP API first, then
fall back to direct database queries when the API returns empty or inconclusive results:

**Manager HTTP endpoint discovery:**

The manager port is loaded from `.devagent/.env` key `MANAGER_PORT`, then overridden by the
`MANAGER_PORT` environment variable if set, falling back to `18080` (`src/shared/config/config.go:73`).
Discovery order:

1. Read `MANAGER_PORT=<value>` from `.devagent/.env`, stripping comments and whitespace.
2. If the `MANAGER_PORT` environment variable is set, it takes precedence over the `.env` file.
3. If neither source provides a value, default to `18080`.

Verify the port is listening before making requests:

```bash
nc -z localhost <PORT> 2>/dev/null && echo "reachable" || echo "unreachable"
```

**Retrieve task details when manager is reachable:**

```bash
curl -s http://localhost:<PORT>/api/v1/tasks/<TASK_ID>
```

This returns the task record with `id`, `title`, `status`, `branch`, `pr`, `assignee`, and timestamps.

**Retrieve task flow (gate state, agent runs, and comments in one call):**

```bash
curl -s http://localhost:<PORT>/api/v1/tasks/<TASK_ID>/flow
```

This returns a `TaskFlowResponse` with:
- `task_id` — the task ID
- `active_gate` — current gate state (`gate_status`, `state`, `on_pass`, `on_fail`, `required_agents`,
  `optional_agents`, `blocked_reason`, and `assignments` with per-agent outcomes)
- `agent_runs` — all agent execution attempts with `agent_definition_id`, `status`, `outcome`,
  `error` message, and `workflow_name`
- `comments` — all durable task comments with `author_id`, `body`, `source`, `agent_run_id`,
  `created_at`

**Retrieve task comments only (alternative when /flow is large):**

```bash
curl -s http://localhost:<PORT>/api/v1/tasks/<TASK_ID>/comments
```

**Direct database query fallback (when HTTP API returns "task not found" or empty stubs):**

The HTTP API may return `"task not found"` or empty flow stubs even when data exists in the
database (e.g. after a DB restore, schema drift, or partial cleanup). Fall back to direct DB
queries when the API is inconclusive.

Derive the container prefix from `.devagent/.env` `PROJECT_ID` (first 8 chars). If `PROJECT_ID`
is unset, the prefix defaults to `swarm`. Then query the Postgres container directly:

```bash
# Derive prefix
PREFIX=$(grep PROJECT_ID .devagent/.env | head -1 | cut -d= -f2 | cut -c1-8)
echo "prefix=$PREFIX"

# Confirm postgres is running
docker ps --format '{{.Names}}' | grep "${PREFIX}-postgres" || echo "postgres not running"

# Query task state
PGPASSWORD=swarm docker exec -i ${PREFIX}-postgres psql -U swarm -d swarm -t -A <<SQL
SELECT id, status::text, title, assignee, branch, pr, created_at, updated_at
FROM tasks WHERE id = '<task_id>';

SELECT id, gate_status::text, state::text, on_pass::text, on_fail::text, created_at
FROM gate_instances WHERE task_id = '<task_id>'
ORDER BY created_at DESC;

SELECT agent_definition_id, outcome::text, is_required, completed_at
FROM gate_assignments WHERE task_id = '<task_id>'
ORDER BY created_at DESC;

SELECT id, agent_definition_id, outcome::text, trigger_status::text,
       claiming_agent_id, claimed_at, completed_at
FROM agent_runs WHERE task_id = '<task_id>'
ORDER BY created_at DESC;
SQL
```

If the postgres container is not running, note this and continue with HTTP API results only.

**Retrieve task details via swarm-agent (Docker-only):**

```bash
swarm-agent task show <TASK_ID> --json
```

If `swarm-agent` is not installed, skip this path — it is Docker-only and unavailable in host
environments.

If the manager is unreachable, note this explicitly and continue with artifact-only evidence.

### 3. Branch Discovery and Checkout

Derive the working branch:
1. From the live task record (`branch` field), if the manager returned one.
2. From the task artifacts (plan files in `.agents/TODO/` or handoff notes).
3. Fallback: `feature/<task-id>` because the worker creates branches with the pattern
   `feature/<task-id>` (`src/agents/worker/agent.go:225-231`). The function truncates long
   IDs at 80 characters, so if the task ID produces a branch name longer than 80 characters,
   the actual branch on disk may differ from the simple `feature/<task-id>` fallback.

**Before switching branches**, check the worktree:

```bash
git status --porcelain
```

If there are uncommitted changes, **do not switch branches**. Report the dirty worktree condition
and continue with the current branch, noting that the working tree does not match the task branch.

If the worktree is clean, attempt to check out the task branch:

```bash
git checkout <task-branch>
```

If the checkout fails (branch does not exist locally):

- **Do not** run `git fetch`, `git pull`, `git checkout -b`, or any other git command that creates
  or fetches branches.
- Report the missing branch explicitly and continue producing an incomplete-but-honest report.
- Suggest that the operator run `git fetch origin && git checkout <task-branch>` to manually bring
  the branch in.

### 4. Diff Inspection

Once on a branch (or if stuck on the current branch due to dirty worktree / missing branch),
inspect code changes:

**Discover the base for comparison:**

The review base is typically `origin/main`. Derive it from whichever source is available:

1. From the live task record — check if the task has a `pr` field (PR number) and PR metadata
   indicating a `base_branch` against which the PR was opened.
2. From task artifacts — plan files under `.agents/TODO/` may reference a review base.
3. Fall back to `origin/main`, then `main`:

```bash
git merge-base HEAD origin/main 2>/dev/null || git merge-base HEAD main 2>/dev/null || echo "unknown"
```

**Collect diff evidence against the discovered base:**

Use three-dot diff to show only changes on the current branch since it diverged from the base:

```bash
BASE=$(git merge-base HEAD origin/main 2>/dev/null || git merge-base HEAD main 2>/dev/null)
if [ -n "$BASE" ]; then
  git diff --stat "$BASE"...HEAD
  git diff "$BASE"...HEAD
else
  echo "no merge base found"
fi
```

If no base can be determined and no PR exists, report the diff only as `git diff` against the
index (unstaged changes) and note the missing base comparison.

### 5. Optional Deeper Investigation

If the report so far is insufficient for a human operator to decide next steps, suggest these
follow-up surfaces:

- **`scripts/inspect-task <project-prefix> <task-id>`**: full task forensics (task record,
  gate instances, gate assignments, agent runs, manager/agent logs, task comments, timeline)
  — requires a running swarm stack and Docker access.
- **`.opencode/commands/investigate-swarm-run.md`**: trace a specific agent run or task lifecycle
  failure when unexpected outcomes or instant failures are suspected.

## Output Format

Produce a structured markdown report with these sections. Every section that has data must include
a source-provenance tag: `[live]` for manager-derived data, `[artifact]` for artifact-derived data,
`[diff]` for git-diff observations, or `[none]` when no data is available.

```markdown
## Completeness: <full | partial | minimal>

<One sentence declaring whether the evidence is full (live API + DB + diff), partial (artifact-only or
missing branch), or minimal (no comments or artifacts found).>

---

## Task Snapshot

- **Task ID**: <id> `[source]`
- **Title**: <title> `[source]`
- **Status**: <status> `[source]`
- **Assignee**: <assignee> `[source]`
- **Branch**: <branch> `[source]`
- **PR**: <pr-url> `[source]`

## Evidence Sources

| Source | Available | Notes |
|---|---|---|
| Live manager (HTTP API) | yes / no | <port, swarm-agent status> |
| Direct DB query (Postgres) | yes / no | <whether DB queries returned rows> |
| Local artifacts (.devagent/logs/) | yes / no | <count files found> |
| Local artifacts (.agents/TODO/) | yes / no | <count files found> |
| Local artifacts (.agents/HANDOFFS/) | yes / no | <count files found> |
| Git diff | yes / no | <base used or reason unavailable> |

## Branch & Diff Findings

**Branch State**: <on task branch / on different branch / dirty worktree prevented checkout /
branch missing locally>

**Base**: <base ref or "could not determine">

**Diff Summary**:
```
<diff --stat output or description>
```

**Key Changed Areas**:
- <file or directory pattern> — <brief observation> `[diff]`

## Durable Task Comments

<!-- Each comment preserves provenance metadata. Separate live from artifact comments explicitly. -->

<If no comments exist in any source, write: "No durable task comments found.">

<For each live comment:>
- **Author**: <author_id> | **Source**: <source> | **Created**: <created_at> | **Agent Run**: <agent_run_id> `[live]`
  ```
  <comment body excerpt>
  ```

<For each artifact comment:>
- **Author**: <author_id> | **Source**: <source> | **Created**: <created_at> | **Agent Run**: <agent_run_id> `[artifact — may be stale]`
  ```
  <comment body excerpt>
  ```

## Artifact-Derived Historical Context

<!-- Content extracted from saved prompt-context files under .devagent/logs/ -->
<!-- All content in this section is marked as potentially stale. -->

<If no saved context artifacts exist: "No saved task-context artifacts found.">

<Otherwise, summarize relevant findings from task-context files, keeping each observation tagged
with the artifact file it came from. Do not duplicate comments already listed above.>

## Unresolved Gaps

<!-- These are concrete things that still need to be done, are blocked, or are uncertain. -->

<If none: "No unresolved gaps identified from available evidence.">

- [Gap description] — <source provenance> — <impact>

## Concrete Next Steps

<!-- Actionable, ordered steps the operator should follow. Each step must be concrete. -->

1. <actionable step>
```

## Rules

- This command is read-only. Do not write, commit, push, create PRs, modify task state, or change
  any file.
- The only git mutation allowed is `git checkout <task-branch>` when the worktree is clean and the
  branch exists locally.
- Do not auto-fetch branches, create branches, or perform any network-side-effecting operations.
- Every data point in the report must carry a source-provenance tag: `[live]`, `[db]`, `[artifact]`,
  `[diff]`, or `[none]`.
- Treat live manager comments and artifact-derived comments as separate sources. Never collapse
  them into one combined list.
- When the manager is unreachable, state it clearly and report artifact-derived data as partial
  evidence.
- When the HTTP API returns "task not found" or empty flow stubs, query the database directly
  before concluding the task is truly absent. The API may return empty results while the DB still
  holds the complete record.
- When a branch is missing or the worktree is dirty, report the condition and continue with an
  incomplete-but-honest report — do not abort.
- If both `swarm-agent` and the manager HTTP API are unavailable, do not treat this as an error.
  Produce the best report possible from local artifacts and git state.
- Back all code references and path claims with confirmed repository surfaces. Do not invent files,
  endpoints, or conventions.
