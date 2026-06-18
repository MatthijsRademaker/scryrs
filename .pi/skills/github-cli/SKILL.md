---
name: github-cli
description: Use the GitHub CLI (gh) to create, view, and manage pull requests and branches from automated workflows and pending actions.
---

# GitHub CLI

This skill provides the canonical contract for `gh` commands used by automated workflows, gate hooks, and pending actions within the swarm system.

## PR Creation (Pending Actions)

Pending actions of kind `create_pr` use `gh pr create` with the following contract:

```bash
gh pr create \
  --title "<title>" \
  --body "<body>" \        # or --fill if body is empty
  --head "<branch>" \
  --base "<base>"          # optional, defaults to repo default
```

**Important**: `gh pr create` does NOT support `--json`. It prints the PR URL to stdout on success. Never use `--json url` with `gh pr create`.

### Output Handling

On success, `gh pr create` prints the PR URL. Trim whitespace and verify the output starts with `https://`. If the output is not a URL, the PR may already exist for the branch.

### Fallback: Existing PR Lookup

If `gh pr create` returns a non-URL output or fails, look up an existing PR for the branch:

```bash
gh pr view --head "<branch>" --json url --jq .url
```

This command DOES support `--json` and returns the URL via `--jq .url`.

## Preconditions

Before calling `gh pr create`, the branch must be pushed with upstream tracking:

```bash
git push -u origin <branch>
```

## Base Branch Protection

Do not create PRs from base branches (`main`, `master`). PRs must originate from feature branches. If the resolved head branch matches a base branch, fail the action with a clear error.

## PR Merge (Pending Actions)

Pending actions of kind `merge_pr` use `gh pr merge` with the following contract:

```bash
gh pr merge <pr_url> --<method> [--delete-branch] [--auto]
```

| Flag | Source | Description |
|---|---|---|
| `--squash` / `--merge` / `--rebase` | `merge_method` config (default: `merge`) | Merge method to use |
| `--delete-branch` | `delete_branch` config (`"true"` → present) | Delete the head branch after merge |
| `--auto` | `auto` config (`"true"` → present) | Enable auto-merge (disables prompts) |

### Idempotency

If `gh pr merge` fails, the worker checks whether the PR is already merged:

```bash
gh pr view <pr_url> --json state --jq .state
```

If the state is `MERGED`, the action succeeds. Otherwise the failure propagates.

### Config Fields

The merge_pr config is enriched at the manager side with task-level data before dispatch:

| Field | Source | Description |
|---|---|---|
| `pr_url` | `task.PR` (auto-injected) | The pull request URL to merge |
| `branch` | `task.Branch` (auto-injected) | The feature branch name |
| `merge_method` | Gate hook definition (default: `squash`) | Merge strategy |
| `delete_branch` | Gate hook definition (default: `true`) | Delete branch after merge |
| `auto` | Gate hook definition (default: `true`) | Auto-merge mode |

## Repository Setup

The worker assumes the current working directory is a git repository with:
- `git` configured with valid credentials
- `gh` authenticated (`gh auth status` passes)
- A remote named `origin`
