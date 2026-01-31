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

**Follow the atomic commit lock protocol at `.claude/rules/commit-lock.md`**

Key points:
- All operations (check + acquire + commit + release) in ONE Bash command
- Use stream name `polish`
- Commit prefix: `refactor(scope):`

## IDLE Signal (ONLY when truly empty)

Count unchecked `[ ]` items in backlog.md that are NOT:
- Struck through with `~~text~~`
- Marked `(excluded)`
- Marked with `(PRD:*)`

**ONLY if count = 0**, output: `<promise>IDLE</promise>`

Otherwise, just complete the task and stop normally (next iteration picks next item).

---

Full workflow in: `.claude/rules/stream-polish.md`
