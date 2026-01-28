@specs/manifest.json @streams/features/backlog.md @.claude/rules/stream-features.md

# Features Stream

## Phase 0: Recovery Check (ALWAYS FIRST)

```
1. Read streams/features/activity.md → get LAST entry (task name, files mentioned)
2. No recent entry or entry marked complete? → Skip recovery, proceed to normal workflow
3. Entry exists and looks incomplete? → Run: git status --porcelain
4. Check if uncommitted changes CORRELATE to the logged task:
   - Files in streams/features/ or specs/phases/ → YES, correlates
   - Files in src/ or src-tauri/ mentioned in activity log → YES, correlates
   - Files in src/ or src-tauri/ NOT mentioned → NO, other stream's work
5. Correlated changes exist? → Complete if needed, commit ONLY correlated files, proceed
   No correlated changes? → Proceed to normal workflow
```

**CRITICAL:** Only commit files that correlate to YOUR activity log. Leave other uncommitted
files alone - they belong to other streams.

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

### Commit Lock Protocol
```
1. BEFORE committing: Check .commit-lock file
   → EXISTS? Wait or skip commit this iteration
   → NOT EXISTS? Create it: echo "features $(date -u +%Y-%m-%dT%H:%M:%S)" > .commit-lock

2. Commit your files only: git add <file1> <file2> ... && git commit

3. AFTER committing: rm -f .commit-lock
```

### Commit Steps
1. Acquire lock (create `.commit-lock`)
2. `git add <file1> <file2> ...` — only your files
3. Commit with prefix: `feat:` | `fix:` | `docs:`
4. Release lock (delete `.commit-lock`)

## All phases complete?
Output: `<promise>COMPLETE</promise>`

---

Full workflow in: `.claude/rules/stream-features.md`
