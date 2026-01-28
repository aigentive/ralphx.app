@specs/manifest.json @streams/features/backlog.md @.claude/rules/stream-features.md

# Features Stream

## Phase 0: Recovery Check (ALWAYS FIRST)

```
1. Run: git status --porcelain streams/features/ specs/phases/
2. Check streams/features/activity.md for last entry (task name, files mentioned)
3. Uncommitted changes exist AND activity log shows incomplete features work?
   → YES: Verify changes MATCH the logged task (PRD task or P0 item)
          Changes match? → Complete if needed, commit, proceed
          Changes DON'T match? → SKIP (other stream's work) → proceed to normal workflow
   → NO activity log entry or no uncommitted changes? → Proceed to normal workflow
```

**CRITICAL:** Do NOT touch uncommitted changes in src/ or src-tauri/ unless they are clearly
attributable to a PRD task or P0 item logged in YOUR activity file. Other streams may have
uncommitted work there.

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
