---
description: investigate a bug and identify likely affected areas without full root cause analysis
agent: plan
swarm: true
agent_types:
  - swarm-worker
---

Investigate the bug described below and produce a grounded triage analysis.

$ARGUMENTS

## Investigation Workflow

### 1. Understand the Report
- If `$ARGUMENTS` references an issue number, PR, commit hash, log excerpt, or stack trace, start from that artifact.
- If `$ARGUMENTS` is a plain description, treat it as the bug report.
- Identify the likely affected component, package, module, command, API, or UI flow before inspecting code.
- Do not perform a full reproduction unless the report already contains enough direct evidence to reason from.

### 2. Find Relevant Code Areas
- Search the codebase for likely entry points, handlers, commands, services, state transitions, or config paths.
- Map the broad code areas that may be involved.
- Reference specific files and line ranges when available.
- Prefer identifying likely ownership boundaries and interaction points over tracing every function.

### 3. Form Hypotheses
- List plausible causes based on inspected code.
- Separate evidence-backed observations from speculation.
- Assign a confidence level to each hypothesis: high, medium, or low.
- Do not claim a definitive root cause unless the evidence is direct and obvious.

### 4. Assess Expected Behavior
- Describe what the system likely does today.
- Describe what the user likely expected instead.
- Note where behavior may diverge, including config, state, timing, environment, API contract, or data-shape assumptions.

### 5. Recommend Next Investigation Steps
- Suggest focused checks, logs, tests, manual steps, or files to inspect next.
- Keep recommendations scoped to confirming or ruling out the hypotheses.
- Avoid prescribing a full fix unless the likely change is obvious.

## Output Format

```markdown
## Investigation Summary
[One paragraph summarizing the suspected area and current expectation.]

## Likely Affected Areas
- [file/path or module] — [why it may be involved]

## Evidence
- [Concrete observation with file:line reference]
- [Concrete observation with file:line reference]

## Hypotheses
- Confidence: [high/medium/low]
  Area: [component/module]
  Explanation: [why this may cause the bug]
  Evidence: [file:line references or report details]

## Expected Behavior Analysis
[What appears to happen vs. what likely should happen.]

## Recommended Next Steps
- [Focused reproduction, log check, test, or code inspection step]
```

## Rules
- Do not make changes to the codebase. This is investigation only.
- Do not perform full root cause analysis unless the cause is immediately evident from inspected code.
- Do not require full reproduction; note whether reproduction would be useful as a follow-up.
- Back concrete findings with file:line references.
- Clearly label uncertainty and confidence.
- If the bug description is insufficient to proceed, list concise missing information.
