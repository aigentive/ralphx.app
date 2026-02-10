---
description: Show project-wide task status across all phases
allowed-tools: Read, Bash, Grep
---

# Work Status

Display a summary of tasks remaining across all active and pending phases.

## Instructions

### Step 1: Read Manifest

Read `specs/manifest.json` to get:
1. The current phase number
2. All phases with their status (active, pending, complete)

### Step 2: Identify Relevant Phases

Collect all phases where `status` is `"active"` or `"pending"`.

### Step 3: Count Tasks in Each Phase

For each relevant phase:
1. Read the PRD file at the path specified in the manifest
2. Find the JSON task list (inside ```json code block under "## Task List")
3. Parse the tasks and count:
   - Total tasks in the phase
   - Tasks with `"passes": false` (remaining)
   - Tasks with `"passes": true` (completed)

### Step 4: Display Summary Table

Output the following format:

```markdown
## Project Task Summary

| Category | Count |
|----------|-------|
| **Active phase (N)** | X tasks remaining |
| **Upcoming phases** | Y phases pending |
| **Total tasks to do** | Z tasks |

## Phase Breakdown

| Phase | Name | Status | Total | Remaining |
|-------|------|--------|-------|-----------|
| N | Phase Name | active | X | Y |
| N+1 | Next Phase | pending | X | X |
| ... | ... | ... | ... | ... |
```

### Step 5: Show Active Phase Details (if exists)

If there's an active phase with remaining tasks, show task-level status:

```markdown
## Active Phase Tasks (Phase N: Name)

| Task | Description | Status |
|------|-------------|--------|
| 1 | Description | Done |
| 2 | Description | **Pending** |
| ... | ... | ... |
```

Use "Done" for `passes: true` and "**Pending**" (bold) for `passes: false`.

### Step 6: Show Next Action

At the end, suggest the next action:

- If active phase has remaining tasks: "**Next:** Task N in Phase X - {description}"
- If active phase is complete but pending phases exist: "**Next:** Activate Phase X with `/activate-prd`"
- If all phases complete: "**All phases complete!**"
