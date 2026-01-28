@specs/manifest.json @streams/features/backlog.md @.claude/rules/stream-features.md

# Features Stream

## Phase 0: Recovery Check (ALWAYS FIRST)

```
1. Run: git status --porcelain streams/features/ src/ src-tauri/ specs/phases/
2. Uncommitted changes exist?
   → YES: Run git diff to review changes
          Check streams/features/activity.md for last entry
          If work looks complete → Commit with appropriate prefix (feat:/fix:/docs:)
          If work incomplete → Try to complete it, then commit
          Then proceed to normal workflow
   → NO: Proceed to normal workflow
```

---

Execute ONE task, then STOP.

## Priority
1. **P0 items in backlog.md** → Fix first, no exceptions
2. **PRD tasks** → Find first `passes: false`

## Quick Workflow
```
P0 exists? → Fix → Mark [x] → Commit → STOP
No P0? → Manifest → Active phase PRD → First failing task → Execute → Log → passes: true → Commit → STOP
```

## Git Commit Rules (CRITICAL - parallel streams)

**NEVER use `git add .` or `git add -A`** — other streams have uncommitted changes!

1. Track files YOU modified during this task
2. `git add <file1> <file2> ...` — only your files
3. Commit with appropriate prefix: `feat:` | `fix:` | `docs:`

## All phases complete?
Output: `<promise>COMPLETE</promise>`

---

Full workflow in: `.claude/rules/stream-features.md`
