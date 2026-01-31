@streams/refactor/backlog.md @streams/polish/backlog.md @streams/archive/completed.md @.claude/rules/stream-hygiene.md

# Hygiene Stream

## Phase 0: Recovery Check (ALWAYS FIRST)

```
1. Run: git status --porcelain streams/hygiene/ streams/archive/ streams/*/backlog.md
2. Uncommitted changes exist in hygiene-owned files?
   → YES: Run git diff to review changes
          Check streams/hygiene/activity.md for last entry
          If work looks complete → Commit with chore(hygiene):
          If work incomplete → Try to complete it, then commit
          Then proceed to normal workflow
   → NO: Proceed to normal workflow
```

---

Maintain backlogs, then STOP.

## Rules
- Archive >10 completed items
- Refill <3 active items via Explore
- Validate strikethrough items periodically
- Do NOT fix code (that's other streams' job)

## Quick Workflow
```
Archive excess [x] items → Refill low backlogs via Explore → Validate 2-3 strikethroughs → Log → Commit (only your files) → STOP
```

## Git Commit Rules (CRITICAL - parallel streams)

**NEVER use `git add .` or `git add -A`** — other streams have uncommitted changes!

**Follow the atomic commit lock protocol at `.claude/rules/commit-lock.md`**

Key points:
- All operations (check + acquire + commit + release) in ONE Bash command
- Use stream name `hygiene`
- Only commit: `streams/*/backlog.md`, `streams/archive/*`, `streams/hygiene/activity.md`
- Commit prefix: `chore(hygiene):`

## Nothing to maintain?
Output: `<promise>COMPLETE</promise>`

---

Full workflow in: `.claude/rules/stream-hygiene.md`
