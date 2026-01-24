# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

Ralph Wiggum Loop Starter is an autonomous development loop for Claude Code. It runs Claude iteratively with fresh context windows until all tasks in a PRD are complete.

## Architecture

The system has three phases:

1. **PRD Creation** (`/create-prd`): Interactive wizard that gathers requirements and generates `specs/prd.md` with a JSON task list
2. **Autonomous Loop** (`ralph.sh`): Bash script that repeatedly invokes Claude with `PROMPT.md` until completion
3. **Task Execution**: Each iteration, Claude finds one task with `"passes": false`, completes it, marks it `true`, logs progress to `logs/activity.md`, and commits

The loop terminates when Claude outputs `<promise>COMPLETE</promise>` (all tasks pass) or max iterations is reached.

## Key Files

| File | Purpose |
|------|---------|
| `ralph.sh` | Main loop - invokes `claude -p` with stream-json output, parses for completion signal |
| `PROMPT.md` | Template prompt fed each iteration - references `@specs/prd.md` and `@logs/activity.md` |
| `specs/prd.md` | Generated PRD with embedded JSON task list (created by `/create-prd`) |
| `specs/plan.md` | Implementation plan document |
| `logs/activity.md` | Append-only log of completed work across iterations |
| `.claude/settings.json` | Pre-configured permissions for autonomous operation |
| `.claude/commands/create-prd.md` | PRD creation wizard command definition |

## Running the Loop

```bash
# First create your PRD interactively
/create-prd

# Then run the autonomous loop (20 iterations max)
./ralph.sh 20
```

Requirements: `claude` CLI, `jq`, bash

## Task List Format

Tasks in `specs/prd.md` follow this structure:

```json
[
  {
    "category": "setup|feature|integration|styling|testing",
    "description": "Task description",
    "steps": ["Step 1", "Step 2"],
    "passes": false
  }
]
```

Only modify the `passes` field when completing tasks.

## Iteration Behavior

Each iteration should:
1. Read `logs/activity.md` to understand current state
2. Find next task with `"passes": false`
3. Complete all steps for that task
4. Update `"passes": true` in `specs/prd.md`
5. Append dated entry to `logs/activity.md`
6. Git commit the changes

## Output

- `logs/iteration_N.json` - Raw Claude stream-json output per iteration (gitignored)
- `logs/activity.md` - Human-readable progress log (tracked)
- `screenshots/` - Visual verification (if agent-browser is used)
