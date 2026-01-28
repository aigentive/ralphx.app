@streams/refactor/backlog.md @.claude/rules/stream-refactor.md @.claude/rules/code-quality-standards.md

# Refactor Stream

## Phase 0: Recovery Check (ALWAYS FIRST)

```
1. Run: git status --porcelain streams/refactor/ src/ src-tauri/
2. Uncommitted changes exist?
   → YES: Run git diff to review changes
          Check streams/refactor/activity.md for last entry
          If work looks complete → Commit with refactor(scope):
          If work incomplete → Try to complete it, then commit
          Then proceed to normal workflow
   → NO: Proceed to normal workflow
```

---

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

### Commit Lock Protocol (see .claude/rules/commit-lock.md)
```
1. Check .commit-lock:
   → NOT EXISTS? Create: echo "refactor $(date -u +%Y-%m-%dT%H:%M:%S)" > .commit-lock
   → EXISTS? Read content, check if stale (>30s). If stale, delete and acquire.
            If not stale: sleep 3, re-read content (lock may change hands), loop.

2. Commit your files: git add <file1> <file2> ... && git commit

3. Release lock: rm -f .commit-lock
```

### Commit Steps
1. Acquire lock (create `.commit-lock`)
2. `git add <file1> <file2> ...` — only your files
3. Commit with prefix: `refactor(scope):`
4. Release lock (delete `.commit-lock`)

## IDLE Signal (ONLY when truly empty)

Count unchecked `[ ]` items in backlog.md that are NOT struck through with `~~text~~`.

**ONLY if count = 0**, output: `<promise>IDLE</promise>`

Otherwise, just complete the task and stop normally (next iteration picks next item).

---

Full workflow in: `.claude/rules/stream-refactor.md`
LOC limits in: `.claude/rules/code-quality-standards.md`
