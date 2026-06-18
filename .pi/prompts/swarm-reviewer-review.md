---
description: review a pull request and provide a merge recommendation
swarm: true
agent_types:
  - swarm-reviewer
---

Review the pull request associated with the current task and produce a merge recommendation.

## Scope Discipline

- Treat the current task prompt, task ID, injected task context/comments, and current branch/PR diff as the complete review scope.
- Do not pull in unrelated tasks, experiments, evaluations, historical repo work, or broader backlog/process concerns unless they are directly needed to assess this task.
- Ground every finding and verdict in the current task requirements, task comments, diff, PR metadata, or repository files inspected specifically for this review.
- If an observation cannot be tied back to this task, exclude it from the verdict.

## Pre-review checks

1. If the task has a PR URL (in metadata or description), extract the PR number.
2. Check merge status: `gh pr view <number> --json mergeable,mergeStateStatus`
   - If `mergeable` is `CONFLICTING` or `mergeStateStatus` contains `dirty` or `conflicting`:
     Report `outcome: "needs_work"` with the specific conflict state. Do NOT approve.
3. Fetch the PR diff: `gh pr diff <number>`

## Review criteria

- **Correctness**: Does the code correctly implement the task requirements?
- **Security**: Are there any security vulnerabilities, exposed secrets, or unsafe patterns?
- **Maintainability**: Does the code follow existing conventions and patterns?
- **Completeness**: Are tests included? Are edge cases handled?
- **Conflicts**: Are there merge conflicts (checked above)?

## Review process

1. Understand the task context from the prompt.
2. Extract every stated requirement, acceptance criterion, and user-facing goal from the task description.
3. Fetch the PR diff: `gh pr diff <number>`.
4. Map each requirement to the PR changes. For each requirement, locate the specific code or behavior that satisfies it.
5. Examine changed files for quality, security, and style issues.
6. Check for merge conflicts via `gh pr view --json mergeable,mergeStateStatus`.
7. Populate the `requirements_checked` array in the output — one entry per requirement.
8. Post a review comment on the PR via:
   ```
   gh pr comment <number> --body "## Review Result: <APPROVED|NEEDS_WORK> ..."
   ```
9. Produce the JSON output.

## Output format (MUST use this exact structure)

Your ENTIRE response must be a single valid JSON object. Do not write any text before or after the JSON. Do not use markdown fences. Do not include commentary, explanations, or summaries outside the JSON.

```json
{
  "outcome": "approved|needs_work",
  "feedback": "summary of the review assessment",
  "requirements_checked": [
    {
      "requirement": "requirement text from the task",
      "status": "satisfied|not_satisfied|not_applicable",
      "evidence": "file path and brief explanation of how this requirement is met or why it is not"
    }
  ],
  "issues": [
    {
      "severity": "info|warning|critical",
      "file": "path/to/file.go",
      "line": 42,
      "category": "security|quality|style|logic|test|alignment|performance",
      "description": "description of the issue",
      "suggestion": "how to fix the issue"
    }
  ]
}
```

- `"outcome": "approved"` is valid ONLY when every stated requirement has `status: "satisfied"` or `"not_applicable"`. Any `"not_satisfied"` requirement forces `"needs_work"`.
- Use `"outcome": "needs_work"` when changes are required before merging.
- Merge conflicts are always `"outcome": "needs_work"` with severity `critical`.
- Always include both the `requirements_checked` array and the `issues` array, even if empty.
- After producing the JSON output, call the `report_review_outcome` tool with `"approved"` or `"needs_work"`. This writes the structured outcome artifact required by the swarm runtime.
- Post a review comment on the PR BEFORE producing your final JSON output.
