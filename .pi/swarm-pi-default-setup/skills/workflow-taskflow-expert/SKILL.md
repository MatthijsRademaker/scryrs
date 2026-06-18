---
name: workflow-taskflow-expert
description: Workflow and taskflow domain expert for this repository. Covers WorkflowDefinition, WorkflowState, TaskFlowDefinition, GateSpec, GatePolicy, EvaluateGate, ScheduleTx, action kinds, input modes, session modes, and validation in src/manager/flowcontroller/, src/shared/types/workflow.go, src/shared/types/taskflow.go, src/agents/worker/workflow_engine.go, and src/agents/worker/workflow_validation.go. Use when adding gates, modifying workflow state execution, or debugging flow-controller scheduling.
---

# Workflow & Taskflow Expert

Grounded, repo-specific reference for the workflow/taskflow domain in this repository.

## Key Files

| File | Role |
|---|---|
| `src/shared/types/workflow.go` | Defines `WorkflowDefinition`, `WorkflowState`, `WorkflowInputMode`, `WorkflowSessionMode`, `WorkflowCapture`, `WorkflowValidation`, `WorkflowActionKind`, and semantic validation (`ValidateWorkflowSemantics`). |
| `src/shared/types/taskflow.go` | Defines `TaskFlowDefinition`, `GateSpec`, `GatePolicy` (`all_finished`, `all_approved`, `exclusive_success`), `GateInstance`, `GateAssignment`, `AgentRunOutcome`, `RunStatus`, `GateHookKind`, and `GateTransition`. |
| `src/manager/flowcontroller/controller.go` | Contains `ScheduleTx` — the entry point that checks if a target status has a gate in the active task flow, creates gate instances and assignments inside a transaction. Also `EvaluateGate` which determines gate pass/fail based on policy. |
| `src/agents/worker/workflow_engine.go` | Agent-side workflow execution engine. Implements `executeWorkflow`, state execution loop, session management, outcome parsing, artifact capture, and the on-success transition. |
| `src/agents/worker/workflow_validation.go` | Validates workflow definitions before execution. Runs semantic checks and reports issues. |
| `src/manager/api/handlers.go` | API handlers that trigger workflow/taskflow operations via the flow controller. |
| `src/manager/persistence/postgres_taskflow.go` | Postgres persistence for task flow definitions, gate instances, and gate assignments. |
| `src/manager/grpc/manager.go` | gRPC service definitions that expose workflow and taskflow operations to agents. |

## Domain Model Summary

### Workflow

- `WorkflowDefinition`: serializable workflow with `Name`, `Version`, `InitialState`, and `States`.
- `WorkflowState`: single node with `ID`, `ActionKind`, `Command`, `Input` (prompt construction), `Session`, `Captures`, `Validation`, `OnSuccess` (next state ID).
- Action kinds: `command`, `fetch_pr_context`, `github_pr_comment`, `merge_pr`, `status_transition`, `emit_event`, `task_comment`, `create_task`, `load_project_context`, `deduplicate_task_proposal`, `record_review_metadata`.
- Input modes: `task_prompt`, `literal`, `artifact`, `file`, `composed`.
- Session modes: `fresh`, `continue_state`, `continue_latest`.
- Validation modes: `marker` (check output for string marker), `file_change` (check for file modifications within scope).

### TaskFlow

- `TaskFlowDefinition`: flow with `Gates[]`, each keyed by `TaskStatus`.
- `GateSpec`: defines required + optional agents, policy, on_pass/on_fail transitions, hooks.
- `GatePolicy`: `all_finished` (all must finish), `all_approved` (all must approve), `exclusive_success` (first to succeed wins).
- `GateInstance`: live instance created when a task enters a gated status. State machine: `pending → blocked/passed/failed/overridden`.
- `GateAssignment`: individual agent assignment within a gate, linked to an `AgentRun` via `AgentRunID`. Has business `Outcome` (finished/approved/needs_work/skipped/failed) separate from `RunStatus`.

### Flow Controller

- `ScheduleTx`: called during task status transitions. Looks up gate for target status, creates gate instance + assignments + agent_runs in a transaction.
- `EvaluateGate`: called when an agent run completes. Checks all required assignments against the gate's policy to determine pass/fail, then applies on_pass/on_fail transitions and hooks.

## Important Rules

- `GatePolicyAllApproved`: any `needs_work` outcome causes immediate fail. Only `approved` outcomes count as success.
- `GatePolicyAllFinished`: any `failed` outcome causes fail. `finished` and `approved` both count as success.
- `GatePolicyExclusiveSuccess`: first required assignment to succeed triggers pass; remaining assignments are skipped.
- Sticky gates: when `Sticky: true`, the same agent assignments are reused across re-evaluations.
- On-pass hooks: `create_pr` and `merge_pr` are executed by the worker after gate passes.

## Architecture Flow

```
Task status change → ScheduleTx (flowcontroller)
  → creates GateInstance + GateAssignments + AgentRuns
  → agents pick up runs → execute workflows
  → agent completes → EvaluateGate (flowcontroller)
    → check policy against outcomes
    → if pass: apply on_pass transition, execute hooks
    → if fail: apply on_fail transition
```
