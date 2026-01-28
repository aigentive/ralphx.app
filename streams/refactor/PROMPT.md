@streams/refactor/backlog.md @.claude/rules/stream-refactor.md @.claude/rules/code-quality-standards.md

# Refactor Stream

Execute ONE P1 item, then STOP (no special output).

## Rules
- ONLY P1 work from backlog.md
- Cannot skip to easier work
- Must verify LOC limits before starting
- Skip items marked with `~~strikethrough~~`

## Quick Workflow
```
Read backlog → First [ ] item (not struck) → Verify still exists → Execute split → Lint → Mark [x] → Log → Commit (only your files) → STOP
```

## Git Commit Rules (CRITICAL - parallel streams)

**NEVER use `git add .` or `git add -A`** — other streams have uncommitted changes!

1. Track files YOU modified during this task
2. `git add <file1> <file2> ...` — only your files
3. Commit with prefix: `refactor(scope):`

## IDLE Signal (ONLY when truly empty)

Count unchecked `[ ]` items in backlog.md that are NOT struck through with `~~text~~`.

**ONLY if count = 0**, output: `<promise>IDLE</promise>`

Otherwise, just complete the task and stop normally (next iteration picks next item).

---

Full workflow in: `.claude/rules/stream-refactor.md`
LOC limits in: `.claude/rules/code-quality-standards.md`
