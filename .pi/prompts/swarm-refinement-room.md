---
description: run the Swarm Refinement Room controller for a backlog task
swarm: true
agent_types:
  - swarm-room-controller
---

Run the Swarm Refinement Room as the canonical refinement model for this task.

Runtime execution is owned by `swarm-room-controller`; Product Owner remains an escalation/product authority, not the task-reactive executor.

The room lifecycle is:

1. backlog task → exploration dossier
2. configured fresh-session agent rounds
3. consensus evaluation with required-agent blockers
4. controller-owned proposal publication
5. gate-owned `commit_artifacts` hook persists artifacts to the task branch
6. fresh worker apply handoff

Use the repository-local controller package rather than a separate preparing-for-dev spec-writer handoff. The controller owns the room ledger, artifact broker, canonical OpenSpec publication, and worker handoff. Git commit, push, and branch persistence are owned by the gate-level `commit_artifacts` hook, not by the room.

Expected command surface:

```bash
npm run room:controller -- init-room --task-id "$DEV_SWARM_TASK_ID" --task "<task description>" --json
npm run room:controller -- explore --ledger "<room-ledger.json>" --json
npm run room:controller -- run-round --ledger "<room-ledger.json>" --json
npm run room:controller -- reduce-round --ledger "<room-ledger.json>" --json
npm run room:controller -- finalize --ledger "<room-ledger.json>" --json
```

Return structured JSON with `outcome: "finished"` only after the room has published canonical OpenSpec artifacts and the handoff is ready, or with `outcome: "needs_work"` if the room reports blockers requiring Product Owner/user resolution.
