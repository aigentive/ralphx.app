# GitHub PR Mode User Guide

GitHub PR Mode changes how RalphX lands a completed **plan** on your base branch. Instead of merging the plan branch directly, RalphX creates and monitors a GitHub pull request for the plan branch and finishes the plan when that PR is merged.

---

## Quick Reference

| Question | Answer |
|----------|--------|
| Where do I enable it? | Settings → Repository → **GitHub PR Mode** |
| What changes when it is on? | RalphX creates a plan-level PR and waits for GitHub to merge it instead of merging the completed plan directly |
| What do I need? | A GitHub remote and an authenticated `gh` CLI |
| Does it affect every task? | No. It changes the **final plan merge** only |
| Do tasks inside the plan still merge normally? | Yes. Tasks still merge into the plan branch through RalphX's normal merge pipeline |
| Does it affect existing plans? | Yes. Turning it on retrofits active plans, and new plans inherit it automatically |
| What if I turn it off mid-plan? | RalphX closes the active PR, clears the PR metadata, and falls back to the direct-merge path |

---

## Table of Contents

1. [Overview](#overview)
2. [Before You Turn It On](#before-you-turn-it-on)
3. [Off vs On](#off-vs-on)
4. [Typical Workflow](#typical-workflow)
5. [What You See in the App](#what-you-see-in-the-app)
6. [Failure Modes](#failure-modes)
7. [Related Guides](#related-guides)

---

## Overview

GitHub PR Mode is a **plan delivery mode**, not a replacement for RalphX's normal execution pipeline.

What stays the same:

- RalphX still creates a plan branch for the plan.
- Individual tasks still execute on task branches.
- Individual tasks still merge into the plan branch through RalphX's normal merge pipeline.

What changes:

- The final **plan-merge task** no longer lands the plan branch directly on the base branch.
- RalphX creates or updates a GitHub PR for the plan branch.
- The plan-merge task waits for that GitHub PR to be merged.
- When GitHub merges the PR, RalphX detects it and completes the plan automatically.

If you want the full direct-merge pipeline instead, leave GitHub PR Mode disabled.

---

## Before You Turn It On

GitHub PR Mode only works when all of these are true:

- Your project remote is hosted on GitHub.
- The GitHub CLI (`gh`) is authenticated on the machine running RalphX.
- You are landing work through a **plan** that has a final plan-merge task.

If the remote is not GitHub, RalphX disables the toggle in Settings. If GitHub integration is unavailable at runtime, RalphX falls back to the normal direct-merge behavior.

---

## Off vs On

| Mode | Final plan delivery | Who completes the merge? | What the user does |
|------|----------------------|--------------------------|--------------------|
| **Off** | RalphX merges the plan branch directly into the base branch | RalphX | Wait for the plan-merge task to finish, or retry if it hits MergeIncomplete / MergeConflict |
| **On** | RalphX creates and monitors a GitHub PR for the plan branch | You (or your normal GitHub review process) | Review and merge the PR in GitHub; RalphX finishes the task automatically after GitHub merges it |

---

## Typical Workflow

1. Enable **GitHub PR Mode** in the Repository settings.
2. Accept a new plan into execution, or turn the setting on while a plan is already active.
3. RalphX creates the plan branch as usual.
4. As plan tasks execute, RalphX can create a **draft PR** for the plan branch early.
5. Plan tasks continue merging into the plan branch normally.
6. When the final **plan-merge task** starts, RalphX pushes the latest plan branch state and marks the PR ready for review.
7. The task enters a PR-waiting flow inside RalphX.
8. You open the PR in GitHub, review it, and merge it using your normal GitHub workflow.
9. RalphX detects the merged PR, records the merge, cleans up the plan branch/task branches, and marks the task **Merged**.

---

## What You See in the App

### Settings

In Settings → Repository, the toggle explains the intended behavior:

- “Create draft PRs when plans execute instead of merging directly”

### Task Cards

When the plan-merge task has an active PR, the Kanban card shows a **Review PR** indicator.

### Merge Detail View

For a PR-backed plan merge, the merge detail view changes meaning:

- The title becomes **Waiting for PR Merge**
- The subtitle explains that RalphX is monitoring GitHub PR status
- The detail panel shows the PR number, status, polling state, and an **Open in GitHub** action

### Completed Merge View

After GitHub merges the PR, the completed task detail shows:

- **Merged via PR #...**

---

## Failure Modes

| Situation | Result in RalphX | What to do |
|-----------|------------------|------------|
| PR closed without merging | Task moves to **MergeIncomplete** | Re-open or recreate the plan PR flow, then retry |
| PR operation fails (push, create, mark ready) | Task moves to **MergeIncomplete** | Fix the GitHub/auth/repo issue and retry |
| GitHub integration unavailable | RalphX falls back to the direct-merge path | Use the normal merge pipeline or restore GitHub integration |
| PR mode turned off mid-plan | RalphX closes the active PR and resumes direct merge handling | Let RalphX retry, or manually retry the merge task later |

If you turn PR mode on while a plan is already active, RalphX updates that active plan to use the PR-backed flow as well. If the plan is already waiting in the final merge stage, RalphX immediately retries that merge through GitHub PR mode.

---

## Related Guides

- [Project Settings & Configuration User Guide](configuration.md)
- [Merge Pipeline User Guide](merge.md)
