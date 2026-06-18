---
description: inspect a running or stalled refinement room controller — trace phases, ledger state, model output, and pi sessions
argument-hint: "<task-id> [container-name]"
---

# Swarm Refinement Room — Runtime Investigation

Use this when a refinement room controller is running, stalled, slow, or producing
unexpected results. You will inspect the container, the ledger file system, the
phase state machine, and the model session output — all live, without needing
the agent to finish.

$ARGUMENTS

You will receive either:
- A `task_id` (UUID) — trace that task's refinement room session
- A container hostname (e.g. `c9357a81-agent-swarm-room-controller-1`) — attach to that container

---

## 0. Prerequisites

```bash
# Identify the project prefix
cat .devagent/.env | grep PROJECT_ID | head -1
# ShortID = first 8 chars. If unset, prefix is "swarm".
PREFIX=<first 8 chars>

# Agent container naming pattern:
#   <PREFIX>-agent-swarm-room-controller-1
#   e.g. c9357a81-agent-swarm-room-controller-1
```

- `docker` — to exec into the container and read files
- `psql` — for DB queries (preferred, but can fall back to docker logs)

---

## 1. Is the Container Alive?

```bash
docker ps --format '{{.Names}} {{.Status}}' | grep room-controller
```

- If **running** → exec in and inspect live state
- If **stopped/killed** → `docker logs` may still work; if container fully removed, only DB traces remain
- If **not found** → the room controller may never have been started, or was cleaned up

### Container Specs

| Property | Value |
|---|---|
| Image | `swarm-worker:latest` (task-reactive) |
| Strategy | `RoomController` (Go `RoomControllerStrategy`) |
| Workspace | `/home/devuser/workspace/project-source/` |
| Ledger root | `.devagent/refinement-room/ledgers/` |
| Pi sessions | `{ledger}/pi-sessions/` |
| Outcome artifacts | `.devagent/tmp/` |
| Heartbeat interval | 30s |
| Stale timeout | 5 min after last heartbeat |

---

## 2. Collect DB State (always available)

The task and agent run records survive container death.

```bash
PGPASSWORD=swarm docker exec -i ${PREFIX}-postgres psql -U swarm -d swarm -t -A
```

### Task record

```sql
SELECT id, status::text, title, assignee, branch, created_at, updated_at
FROM tasks WHERE id = '<task-id>';
```

If status is `Backlog` and updated_at is recent → user moved it back manually after killing the container.

### Gate instances

```sql
SELECT id, gate_status::text, state::text, on_pass::text, on_fail::text, created_at
FROM gate_instances WHERE task_id = '<task-id>'
ORDER BY created_at DESC;
```

A `Refinement` gate with `gate_status=pending` means the room controller never reported its outcome.

### Agent runs

```sql
SELECT id, agent_definition_id, outcome::text, status, claiming_agent_id,
       claimed_at::text, completed_at::text,
       round(extract(epoch from (completed_at - claimed_at))::numeric, 1) AS duration_sec,
       left(error, 200) AS error_preview
FROM agent_runs WHERE task_id = '<task-id>'
ORDER BY created_at DESC;
```

Diagnostic patterns:

| Duration | Error | Meaning |
|---|---|---|
| < 60s | empty | Strategy failed before execution (preflight/node/npm missing) |
| < 60s | `stale run timed out` | Container killed, stale recovery fired 5 min later |
| Several minutes | empty (or long-running) | Strategy actively executing npm commands — container still running |
| Several minutes | `stale run timed out` | Container was killed mid-execution, stale recovery fired |
| NULL outcome + long duration | NULL | RUNNING — exec into container to inspect live state |

---

## 3. Inspect the Live Container

If the container is running, these are the most valuable commands.

### 3a. Follow real-time docker logs

```bash
docker logs -f ${PREFIX}-agent-swarm-room-controller-1
```

Each phase outputs JSON via `console.log()` (with `npm --silent` suppressing npm-level noise):

```
{"roomId":"...","phase":"exploring","dossier":{"areas":[...]}}
{"round":1,"validOutputs":3,"validationFailures":0}
{"phase":"deliberating","consensus":"pending","openBlockers":2}
```

The Go strategy also writes errors to stderr via `fmt.Fprintf(os.Stderr, ...)`:
```
[room-controller] room round produced no valid outputs
```

### 3b. List ledger files

```bash
docker exec ${PREFIX}-agent-swarm-room-controller-1 \
  find /home/devuser/workspace/project-source/.devagent/refinement-room/ledgers/<task-id> -type f
```

Expected structure:
```
ledgers/<task-id>/
├── room-ledger.json                 # Full ledger (state, config, round outputs)
├── pi-sessions/
│   ├── explore/                     # Pi SDK session files for exploration
│   │   ├── <session-id>.jsonl
│   │   └── ...
│   └── round-1/                     # Pi SDK session files for round 1
│       ├── <session-id>.jsonl
│       └── ...
├── config-snapshot.json             # Frozen room config used for this session
├── explore/
│   ├── prompt.md                    # Prompt sent to the model
│   ├── raw-output.txt               # RAW model response text (before parsing)
│   └── dossier.json                 # Parsed and validated exploration dossier
└── round-{N}/
    ├── input.json                   # Round input (task context, prior decisions)
    ├── prompt.md                    # Prompt sent to the model for this round
    ├── output.txt                   # RAW model response text for this round
    └── output.json                  # Parsed round output
```

### 3c. Inspect current phase — the ledger

```bash
docker exec ${PREFIX}-agent-swarm-room-controller-1 \
  cat /home/devuser/workspace/project-source/.devagent/refinement-room/ledgers/<task-id>/room-ledger.json \
  | python3 -m json.tool 2>/dev/null || \
docker exec ${PREFIX}-agent-swarm-room-controller-1 \
  cat /home/devuser/workspace/project-source/.devagent/refinement-room/ledgers/<task-id>/room-ledger.json
```

Key fields in the ledger:

```json
{
  "state": {
    "phase": "exploring",          // ← current phase
    "currentRound": 0,
    "unresolvedBlockers": [],
    "consensus": { "status": "pending" }
  },
  "explorationDossier": { ... },   // populated after explore phase
  "roundOutputs": [],              // populated during deliberation rounds
  "validationFailures": [],        // any parse/schema failures
  "sessionReferences": [           // all Pi sessions created so far
    { "agentId": "...", "sessionId": "...", "status": "completed" }
  ],
  "modelEscalations": []           // model upgrades across rounds
}
```

### 3d. Read raw model output (per phase)

```bash
# Exploration model output (the full LLM response before parsing):
docker exec ${PREFIX}-agent-swarm-room-controller-1 \
  cat /home/devuser/workspace/project-source/.devagent/refinement-room/ledgers/<task-id>/explore/raw-output.txt

# Round N model output:
docker exec ${PREFIX}-agent-swarm-room-controller-1 \
  cat /home/devuser/workspace/project-source/.devagent/refinement-room/ledgers/<task-id>/round-1/output.txt
```

### 3e. Read Pi SDK session files (full tool call trace)

```bash
# List explore sessions:
docker exec ${PREFIX}-agent-swarm-room-controller-1 \
  ls /home/devuser/workspace/project-source/.devagent/refinement-room/ledgers/<task-id>/pi-sessions/explore/

# Tail a specific session (shows tool calls, model responses, outcomes):
docker exec ${PREFIX}-agent-swarm-room-controller-1 \
  tail -100 /home/devuser/workspace/project-source/.devagent/refinement-room/ledgers/<task-id>/pi-sessions/explore/<session-id>.jsonl
```

### 3f. Check the room config

```bash
docker exec ${PREFIX}-agent-swarm-room-controller-1 \
  cat /home/devuser/workspace/project-source/.devagent/refinement-room.config.json
```

Shows lane config, max rounds, required agents, consensus policy.

---

## 4. Phase State Machine

The room controller cycles through phases via `npm run start -- {command}`:

```
 Go strategy                            npm                     Pi SDK
 ──────────                            ─────                   ─────
 CleanWorkspace()
 preflight()
 status --task-id <id> ─────────────►  init-room
                                     ● create room ledger
                                     ● generate exploration prompt
 explore --ledger <file> ───────────►  explore
                                     ● pi session (model call)
                                     ● write raw-output.txt + dossier.json
 status
 run-round --ledger <file> ─────────►  run-round  (1 per required agent)
                                     ● pi session per agent (model call)
                                     ● write output.txt + output.json
 status
 reduce-round --ledger <file> ──────►  reduce-round
                                     ● evaluate consensus
                                     ● identify blockers
 status
              ◄─── if openBlockers > 0 ───► return needs_work
              ◄─── if needs_resolution ───► return needs_work
 synthesize-proposal ───────────────►  synthesize-proposal
 finalize ──────────────────────────►  finalize (gate-owned commit_artifacts hook persists artifacts)
 handoff ───────────────────────────►  handoff
 return finished
```

Max 10 state transitions (hard guard in Go code). After that, controller returns `failed` with `"exceeded 10 state transitions"`.

---

## 5. Common Failure Patterns

### Pattern A: Never leaves `exploring`

```json
{"state": {"phase": "exploring"}, "roundOutputs": []}
```
- The exploration prompt is being sent to the model but the response is slow or the Pi SDK session is hung
- Check `explore/raw-output.txt` — if empty, the model hasn't responded yet
- Check `pi-sessions/explore/` — if no `.jsonl` files, the session hasn't started; if present, inspect the last file

### Pattern B: Round produces no valid outputs

The Go strategy checks:
```go
if roomNumericValue(round.Payload, "validOutputs") == 0 {
    return roomControllerFailed("room round produced no valid outputs"), nil
}
```
- Check `round-{N}/output.txt` — was the model response valid JSON?
- Check `validationFailures` in the ledger — what schema errors occurred?
- Check `modelEscalations` — was the model downgraded?

### Pattern C: Stall between phases

- If `docker logs` shows no new JSON for minutes → the npm command is still running
- `ps aux` inside the container will show a running `node` process
- The Go strategy's `exec.CommandContext` uses `defaultRunCommandTimeout` (60 minutes from CLI)
- Individual npm commands can hang for an hour before the strategy times out

### Pattern D: Container killed, stale timeout

DB shows:
```
status=failed, error='stale run timed out', duration=27min
```
- User stopped the runtime via dashboard
- Heartbeats stopped → after 5 min, `RecoverStaleRuns` fired
- Ledger files inside the container are gone

### Pattern E: Preflight failure (same-minute claim→fail)

```go
func (s *RoomControllerStrategy) preflight() error {
    if _, err := exec.LookPath("node"); err != nil { ... }
    if _, err := exec.LookPath("npm"); err != nil { ... }
    if _, err := os.Stat(filepath.Join(s.packageDir, "package.json")); err != nil { ... }
}
```
- `node` or `npm` missing from container image
- `package.json` not found at the resolved package directory
- Check `docker logs` for `room-controller` error messages

---

## 6. Health Summary

Run this to get a one-shot status of a running room controller:

```bash
PREFIX=<prefix>
TASK_ID=<task-id>
CONTAINER=${PREFIX}-agent-swarm-room-controller-1

echo "=== DOCKER LOGS (last 20 lines) ==="
docker logs --tail=20 "$CONTAINER" 2>&1

echo ""
echo "=== CURRENT PHASE ==="
docker exec "$CONTAINER" \
  cat /home/devuser/workspace/project-source/.devagent/refinement-room/ledgers/$TASK_ID/room-ledger.json 2>/dev/null \
  | python3 -c "import sys,json; d=json.load(sys.stdin); print(f'Phase: {d[\"state\"][\"phase\"]}'); print(f'Round: {d[\"state\"][\"currentRound\"]}'); print(f'Blockers: {len(d[\"state\"][\"unresolvedBlockers\"])}'); print(f'Sessions: {len(d[\"sessionReferences\"])}'); print(f'Rounds completed: {len(d[\"roundOutputs\"])}')" 2>/dev/null \
  || echo "No ledger found — room not yet initialized"

echo ""
echo "=== RAW MODEL OUTPUT (last 30 lines of explore) ==="
docker exec "$CONTAINER" \
  tail -30 /home/devuser/workspace/project-source/.devagent/refinement-room/ledgers/$TASK_ID/explore/raw-output.txt 2>/dev/null \
  || echo "No exploration output yet"

echo ""
echo "=== PI SESSIONS ==="
docker exec "$CONTAINER" \
  find /home/devuser/workspace/project-source/.devagent/refinement-room/ledgers/$TASK_ID/pi-sessions -type f 2>/dev/null \
  | head -10 \
  || echo "No pi sessions yet"
```

---

## 7. Produce a Verdict

| Source | What to look for | Finding |
|---|---|---|
| `docker logs` | JSON phase output, error messages | Which phase, any errors |
| Ledger `room-ledger.json` | `state.phase`, `roundOutputs[]`, `validationFailures[]` | Current progress, failures |
| Ledger `explore/raw-output.txt` | Raw model response text | Model output quality, errors |
| Pi session files | Full tool call trace | Session health, tool errors |
| DB `agent_runs` | Duration, outcome, error | Was it killed, timed out, or completed |
| `ps aux` (inside container) | Running processes | Is the npm command still running, or hung |
