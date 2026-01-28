@streams/polish/backlog.md @.claude/rules/stream-polish.md

# Polish Stream

## Phase 0: Recovery Check (ALWAYS FIRST)

```
1. Run: git status --porcelain streams/polish/ src/ src-tauri/
2. Uncommitted changes exist?
   → YES: Run git diff to review changes
          Check streams/polish/activity.md for last entry
          If work looks complete → Commit with refactor(scope):
          If work incomplete → Try to complete it, then commit
          Then proceed to normal workflow
   → NO: Proceed to normal workflow
```

---

Execute ONE P2/P3 item, then STOP (no special output).

## Rules
- ONLY work from backlog.md
- Cannot skip to other work
- Verify issue exists before starting
- Skip items marked with `~~strikethrough~~` or `(excluded)`

## Quick Workflow
```
Read backlog → First [ ] item (not struck/excluded) → Verify still exists → Execute fix → Lint → Mark [x] → Log → Commit (only your files) → STOP
```

## Git Commit Rules (CRITICAL - parallel streams)

**NEVER use `git add .` or `git add -A`** — other streams have uncommitted changes!

### Commit Lock Protocol
```
1. BEFORE committing: Check .commit-lock file
   → EXISTS? sleep 5, then check again (loop until free)
   → NOT EXISTS? Create it: echo "polish $(date -u +%Y-%m-%dT%H:%M:%S)" > .commit-lock

2. Commit your files only: git add <file1> <file2> ... && git commit

3. AFTER committing (success or failure): rm -f .commit-lock
```

### Commit Steps
1. Acquire lock (create `.commit-lock`)
2. `git add <file1> <file2> ...` — only your files
3. Commit with prefix: `refactor(scope):`
4. Release lock (delete `.commit-lock`)

## IDLE Signal (ONLY when truly empty)

Count unchecked `[ ]` items in backlog.md that are NOT:
- Struck through with `~~text~~`
- Marked `(excluded)`
- Marked with `(PRD:*)`

**ONLY if count = 0**, output: `<promise>IDLE</promise>`

Otherwise, just complete the task and stop normally (next iteration picks next item).

---

Full workflow in: `.claude/rules/stream-polish.md`
