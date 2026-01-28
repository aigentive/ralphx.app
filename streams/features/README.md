# Features Stream

Primary stream for shipping new functionality. Uses **Opus** model.

## Purpose

- Execute PRD tasks from active phases
- Fix P0 critical gaps (orphaned implementations, missing wiring)

## Priority Order

1. **P0 items** in `backlog.md` → Fix first, no exceptions
2. **PRD tasks** → Find first task with `passes: false`

## Workflow

```
1. Check backlog.md for P0 items
   → P0 exists? → Fix it → Mark [x] → Commit → STOP

2. Read specs/manifest.json → find active phase
3. Read phase PRD → find first failing task
4. Execute task following PRD steps
5. Run linters (npm run lint, cargo clippy)
6. Update PRD: set passes: true
7. Commit → STOP
```

## Watched Files

- `streams/features/backlog.md` - P0 items from verify stream
- `specs/manifest.json` - Phase status changes

## Output Signals

- `<promise>COMPLETE</promise>` - All phases done
- `<promise>IDLE</promise>` - No P0s and no active phase tasks

## Files

- `PROMPT.md` - Stream prompt sent to Claude
- `backlog.md` - P0 critical items (populated by verify stream)
- `activity.md` - Activity log

## Related

- Rules: `.claude/rules/stream-features.md`
- Gap verification: `.claude/rules/gap-verification.md`
