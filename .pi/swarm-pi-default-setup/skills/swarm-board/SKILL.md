---
name: swarm-board
description: Interact with the swarm task board. Use when you need to query existing tasks, check backlog status, create new tasks, or understand the current project state.
---

# Swarm Task Board

This skill enables you to interact with the swarm task management system to query tasks, understand project state, and create new work items. All communication uses `swarm-agent` gRPC commands -- no HTTP or curl required.

## Quick Reference

| Action | Command |
|--------|---------|
| List all tasks | `swarm-agent task list` |
| List by status | `swarm-agent task list --status Backlog` |
| Get task details | `swarm-agent task show <id>` |
| Create task | `swarm-agent task create "<title>" -d "<description>" -p <priority>` |
| Update task | `swarm-agent task update <id> --status Review --branch feature/foo` |
| Complete task | `swarm-agent task complete <id>` |
| Delete task | `swarm-agent task delete <id>` |
| Claim task | `swarm-agent task claim [task-id]` |
| Claim review | `swarm-agent task claim-review` |
| Release task | `swarm-agent task release <id>` |
| Register agent | `swarm-agent agent register --type worker --id <id>` |
| Agent heartbeat | `swarm-agent agent heartbeat --agent-id <id>` |
| Get prompt | `swarm-agent prompt get --agent-type worker` |
| Get workflow | `swarm-agent workflow get --agent-type worker` |

Add `--json` to any command for machine-parseable output.

## Querying Tasks

### List All Tasks

```bash
swarm-agent task list
```

Filter by status, assignee, or priority:

```bash
swarm-agent task list --status Backlog
swarm-agent task list --priority high
swarm-agent task list --assignee <agent-id>
```

For JSON output:

```bash
swarm-agent task list --json
```

### Get Single Task

Supports full UUID or partial ID prefix:

```bash
swarm-agent task show 550e8400
swarm-agent task show 550e8400 --json
```

## Creating Tasks

### Create a New Task

```bash
swarm-agent task create "Add password reset functionality" \
  -d "Feature: Password Reset
  As a user
  I want to reset my password
  So that I can regain access to my account

  Scenario: Request password reset
    Given I am on the login page
    When I click \"Forgot Password\"
    And I enter my email address
    Then I should receive a reset email

Technical Notes:
- Use existing email service
- Token expires in 1 hour

Acceptance Criteria:
- [ ] Reset email sent within 30 seconds
- [ ] Token is single-use
- [ ] Old password invalidated after reset" \
  -p Medium

# With JSON output
swarm-agent task create "Add user auth" -d "Feature: ..." -p High --json
```

### Task Fields

| Field | Required | Values |
|-------|----------|--------|
| `title` | Yes | Short descriptive title |
| `description` | No | Gherkin format preferred |
| `priority` | No | `high`, `medium` (default), `low` |

## Updating Tasks

```bash
# Change status
swarm-agent task update <id> --status InProgress

# Set branch and PR
swarm-agent task update <id> --branch feature/login --pr https://github.com/...

# Update title and description
swarm-agent task update <id> --title "New title" --description "Updated description"

# Change priority
swarm-agent task update <id> --priority high
```

## Claiming, Completing, and Releasing

```bash
# Claim a specific task
swarm-agent task claim <task-id>

# Claim next available task (highest priority first)
swarm-agent task claim

# Claim next task needing review
swarm-agent task claim-review

# Complete a task (mark as Done)
swarm-agent task complete <task-id>

# Release a claimed task back to backlog
swarm-agent task release <task-id>
```

## Deleting Tasks

```bash
swarm-agent task delete <task-id>
```

## Agent Lifecycle

```bash
# Register with the manager
swarm-agent agent register --type worker --id my-agent --name "My Agent"

# Send a heartbeat
swarm-agent agent heartbeat --agent-id my-agent --status working --task-id <id>

# Unregister
swarm-agent agent unregister my-agent
```

## Prompts and Workflows

```bash
# Get the active system prompt for a worker
swarm-agent prompt get --agent-type worker

# Get the task prompt for a reviewer
swarm-agent prompt get --agent-type reviewer --prompt-type task

# Get the active workflow definition
swarm-agent workflow get --agent-type worker
```

## Gherkin Format

Tasks should use Gherkin format for clarity:

```gherkin
Feature: [Feature name]
  As a [user role]
  I want [goal]
  So that [benefit]

  Scenario: [Primary scenario]
    Given [precondition]
    When [action]
    Then [expected outcome]

Technical Notes:
- [Implementation detail 1]
- [Implementation detail 2]

Acceptance Criteria:
- [ ] [Criterion 1]
- [ ] [Criterion 2]
```

## Building Context

Before creating new tasks, gather context:

### 1. Check Current Backlog

```bash
swarm-agent task list --status Backlog
```

### 2. Review Completed Work

```bash
swarm-agent task list --status Done
```

### 3. Check Work In Progress

```bash
swarm-agent task list --status InProgress
```

### 4. Get All Tasks

```bash
swarm-agent task list --json
```

## Avoiding Duplicates

Before creating a task, check if similar work exists:

1. **Check backlog** for similar titles
2. **Check completed tasks** - don't recreate finished work
3. **Check in-progress tasks** - don't duplicate active work

Example deduplication check:
```bash
# List all tasks and search for keywords in the output
swarm-agent task list
```

## Task Lifecycle

```
Backlog → InProgress (claimed by worker)
InProgress → Review (PR created, task updated)
Review → Done (approved & merged)
Review → NeedsWork (changes requested)
NeedsWork → InProgress (worker addresses feedback)
```

Agent workflow:

```bash
# 1. Claim next task
swarm-agent task claim

# 2. Work on it, then create branch/PR
swarm-agent task update <id> --branch feature/my-change --status InProgress

# 3. Submit for review
swarm-agent task update <id> --pr https://github.com/... --status Review

# 4. Complete or release
swarm-agent task complete <id>
swarm-agent task release <id>
```

## Priority Guidelines

| Priority | Use When |
|----------|----------|
| **high** | Core functionality gaps, security issues, blocking bugs |
| **medium** | Feature enhancements, performance improvements |
| **low** | Technical debt, documentation, nice-to-have features |

## Best Practices

1. **Keep tasks focused** - 1-4 hours of work each
2. **Use Gherkin format** - Clear acceptance criteria
3. **Check for duplicates** - Query backlog before creating
4. **Set appropriate priority** - Don't make everything High
5. **Include technical notes** - Help workers understand context
