---
description: create a phased implementation plan
agent: plan
swarm: true
agent_types:
  - swarm-worker
---

You are preparing an implementation plan for another LLM coding agent. Inspect the repository and create a grounded, phased plan that is incrementally executable without further inspection.

$ARGUMENTS

## Scope Discipline
- Treat the current task prompt, task ID, injected task context, and explicit command arguments as the complete planning scope.
- Do not pull in unrelated tasks, experiments, evaluations, backlog items, or adjacent cleanup unless the current task explicitly requires them.
- Ground every phase, risk, and validation step in the current task materials or repository files inspected specifically for this task.

## Constraints
- Inspect before planning. Do not assume architecture, file locations, or patterns.
- Do not invent files, modules, APIs, or tests.
- Separate confirmed facts from assumptions.
- Do not implement code.

## Output format
Return only valid markdown following this structure exactly:

# Implementation Plan

## Summary
[1-3 sentence summary of what the plan delivers]

## Domain areas affected
- [domain] — [reason]

## Phased plan

### Phase 1: [goal]
**Why:** [reason this phase comes first]
**Expected changes:** [what behavior changes]
**Domain guidance:** [which skills or domain rules to use]
**Risks:** [what could go wrong]
**Validation:** [how to verify this phase succeeded]

### Phase N: [goal]
(same structure)

## Acceptance criteria
- [criterion]

## Assumptions
- [assumption]

## Open questions
- [question]
