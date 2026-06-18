---
description: gather review feedback and planning inputs
agent: plan
swarm: true
agent_types:
  - swarm-worker
---

$ARGUMENTS

<context>
You are gathering planning inputs in response to a code review.
Task request content (including review feedback and task comments) is provided via $ARGUMENTS in the prompt body above.
Your output will be consumed by another LLM, so optimize for unambiguous structure over human readability.
</context>

<task>
Review the current branch changes and any provided task comments, then produce structured planning inputs for the next planning step.
</task>

<scope_discipline>
- Treat the task request content provided via $ARGUMENTS above, the task ID, and current branch diff as the complete scope.
- Do not pull in unrelated tasks, experiments, evaluations, historical repo work, or broader process concerns unless they are directly needed to understand this task's review feedback.
- Ground every observation in the current task comments, current diff, or repository files inspected specifically for this task.
- If an observation cannot be tied back to this task, exclude it.
</scope_discipline>

<requirements>
<requirement>Run `git diff` against the PR base to understand the current branch changes.</requirement>
<requirement>Locate the `### Task Comments` section in the task request content above and aggregate all substantive durable task comments across review rounds before writing any diff-based observations.</requirement>
<requirement>Read durable task comments from the task request content above. These are the primary review feedback items.</requirement>
<requirement>Call out repeated or previously unresolved feedback explicitly so downstream execution can prioritize recurring defects first.</requirement>
<requirement>Preserve durable task comment metadata for every task-comment-derived item: author, source, created_at, and agent_run_id when present.</requirement>
<requirement>Treat all substantive durable task comments as primary review inputs. Do not privilege `Source=review` over other gate-agent outputs (architect, lead-dev, reviewer).</requirement>
<requirement>Keep `task_comment`, `review_context`, and `diff` findings separate. Do not collapse them into one combined source.</requirement>
<requirement>Do not mistake the original task request, task metadata, or task description for durable review feedback.</requirement>
<requirement>If no durable review/task comments are present, state that explicitly in the output.</requirement>
<requirement>Inspect the codebase around each feedback item to ground the plan in the actual repository.</requirement>
<requirement>Summarize the current branch diff, the review comments, and the concrete repository areas that the planner must consider.</requirement>
<requirement>Identify constraints, risks, assumptions, and any missing information the planner should resolve.</requirement>
<requirement>Do not author the final phased implementation plan.</requirement>
<requirement>Do not save files or implement code changes.</requirement>
</requirements>

<output_format>
Return structured planning inputs using this XML schema:

<planning_inputs>
  <summary>
    <goal>...</goal>
    <diff_summary>...</diff_summary>
    <durable_task_comment_status>present | absent</durable_task_comment_status>
  </summary>

  <review_feedback>
    <!-- Aggregate all substantive durable task comments. -->
    <item>
      <source>task_comment | review_context | diff</source>
      <source_metadata>
        <author>...</author>
        <comment_source>review | agent_run | other | none</comment_source>
        <agent_run_id>...</agent_run_id>
        <created_at>...</created_at>
      </source_metadata>
      <request>...</request>
      <impacted_code>...</impacted_code>
      <notes>...</notes>
    </item>
  </review_feedback>

  <relevant_code_areas>
    <code_area>
      <path>...</path>
      <reason>...</reason>
    </code_area>
  </relevant_code_areas>

  <planner_constraints>
    <constraint>...</constraint>
  </planner_constraints>

  <assumptions>
    <assumption>...</assumption>
  </assumptions>

  <open_questions>
    <question>...</question>
  </open_questions>

  <recommended_validation>
    <step>...</step>
  </recommended_validation>
</planning_inputs>
</output_format>
