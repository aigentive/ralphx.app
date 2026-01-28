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

### Commit Lock Protocol
```
1. BEFORE committing: Check .commit-lock file
   → EXISTS? Wait or skip commit this iteration
   → NOT EXISTS? Create it: echo "hygiene $(date -u +%Y-%m-%dT%H:%M:%S)" > .commit-lock

2. Commit your files only: git add <file1> <file2> ... && git commit

3. AFTER committing: rm -f .commit-lock
```

### Commit Steps
1. Acquire lock (create `.commit-lock`)
2. Only commit: `streams/*/backlog.md`, `streams/archive/*`, `streams/hygiene/activity.md`
3. Commit with prefix: `chore(hygiene):`
4. Release lock (delete `.commit-lock`)

## Nothing to maintain?
Output: `<promise>COMPLETE</promise>`

---

Full workflow in: `.claude/rules/stream-hygiene.md`
