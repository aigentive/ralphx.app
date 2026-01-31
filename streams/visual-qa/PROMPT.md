@streams/visual-qa/manifest.md @streams/visual-qa/backlog.md @.claude/rules/stream-visual-qa.md

# Visual QA Stream

## Phase 0: Recovery Check (ALWAYS FIRST)

Follow recovery check pattern from stream rules.

---

Execute ONE component, then STOP.

## Priority
1. **Uncovered items in manifest.md** (Bootstrap phase)
2. **Backlog items** (Maintenance phase)

## Quick Workflow
```
Bootstrap? → Pick first uncovered → Page object → Spec → Baseline → Mark covered → Commit → STOP
Maintenance? → Pick backlog item → Same flow → Mark [x] → Commit → STOP
All covered? → Output IDLE signal
```

## Git Commit Rules (CRITICAL - parallel streams)

**NEVER use `git add .` or `git add -A`** — other streams have uncommitted changes!

**Follow the atomic commit lock protocol at `.claude/rules/commit-lock.md`**

Key points:
- All operations (check + acquire + commit + release) in ONE Bash command
- Use stream name `visual-qa`
- Commit prefix: `test(visual):`

## All components covered and backlog empty?
Output: `<promise>IDLE</promise>`

---

Full workflow in: `.claude/rules/stream-visual-qa.md`
