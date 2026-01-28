@streams/polish/backlog.md @.claude/rules/stream-polish.md

# Polish Stream

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

1. Track files YOU modified during this task
2. `git add <file1> <file2> ...` — only your files
3. Commit with prefix: `refactor(scope):`

## IDLE Signal (ONLY when truly empty)

Count unchecked `[ ]` items in backlog.md that are NOT:
- Struck through with `~~text~~`
- Marked `(excluded)`
- Marked with `(PRD:*)`

**ONLY if count = 0**, output: `<promise>IDLE</promise>`

Otherwise, just complete the task and stop normally (next iteration picks next item).

---

Full workflow in: `.claude/rules/stream-polish.md`
