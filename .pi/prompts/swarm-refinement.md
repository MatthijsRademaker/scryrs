---
description: refine architecture approach and implementation strategy
swarm: true
agent_types:
  - swarm-architect
  - swarm-lead-dev
---

Refine the task proposal through architecture-scope assessment and technical approach review.

## Scope Discipline

- Treat the current task prompt, task ID, injected task context/comments, and repository files inspected for this task as the complete refinement scope.
- Do not pull in unrelated tasks, experiments, evaluations, historical repo work, or broader backlog/process concerns unless they are directly needed to refine this task.
- Ground every risk, recommendation, and architectural assessment in the current task materials or inspected repository evidence.
- If an observation cannot be tied back to this task, exclude it.

Responsibilities:
- Assess the task's scope relative to existing architecture boundaries, data flow, and dependency direction
- Evaluate the proposed or implied technical approach for correctness and maintainability
- Identify architectural drift, circular dependencies, or boundary violations
- Identify risks, edge cases, and missing implementation considerations
- Recommend specific technical improvements and architecture doc updates
- Identify documentation gaps and recommend updates

Refinement process:
1. Understand the task context from the prompt
2. Inspect the codebase for relevant components, conventions, and patterns
3. Compare the task proposal against the existing system structure
4. Identify risks, edge cases, and missing considerations
5. Produce structured output

Output format (MUST use this exact structure):

```json
{
  "outcome": "finished",
  "architecture_assessment": "structural alignment with existing system",
  "technical_approach": "assessment of the proposed implementation approach",
  "risks": ["risk one", "risk two"],
  "recommendations": ["specific suggestion one", "specific suggestion two"],
  "changes_made": ["list of architecture docs updated"]
}
```

- **MUST always report `outcome: "finished"`**. Refinement agents produce structured implementation guidance regardless of the proposal quality. The spec-writer is the sole authority for rejecting insufficiently-refined tasks.
- **NEVER report `needs_work` or `approved`** — these cause the `all_finished` gate to fail and route the task to Backlog, bypassing the bounded `PreparingForDev → Refinement` retry loop.
- Produce honest, critical assessments in the structured fields — do not soften findings. The `risks` and `architecture_assessment` fields are where concerns are communicated.
- After producing the JSON output, call the `report_refinement_outcome` tool with `"finished"`. This writes the structured outcome artifact required by the swarm runtime.
