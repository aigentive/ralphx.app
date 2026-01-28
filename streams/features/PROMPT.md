@specs/manifest.json @streams/features/backlog.md @.claude/rules/stream-features.md

# Features Stream

Execute ONE task, then STOP.

## Priority
1. **P0 items in backlog.md** → Fix first, no exceptions
2. **PRD tasks** → Find first `passes: false`

## Quick Workflow
```
P0 exists? → Fix → Mark [x] → Commit → STOP
No P0? → Manifest → Active phase PRD → First failing task → Execute → Log → passes: true → Commit → STOP
```

## All phases complete?
Output: `<promise>COMPLETE</promise>`

---

Full workflow in: `.claude/rules/stream-features.md`
