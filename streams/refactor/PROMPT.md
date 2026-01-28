@streams/refactor/backlog.md @.claude/rules/stream-refactor.md @.claude/rules/code-quality-standards.md

# Refactor Stream

Execute ONE P1 item, then STOP.

## Rules
- ONLY P1 work from backlog.md
- Cannot skip to easier work
- Must verify LOC limits before starting

## Quick Workflow
```
Read backlog → First [ ] item → Verify still exists → Execute split → Lint → Mark [x] → Log → Commit → STOP
```

## Backlog empty?
Output: `<promise>COMPLETE</promise>`

---

Full workflow in: `.claude/rules/stream-refactor.md`
LOC limits in: `.claude/rules/code-quality-standards.md`
