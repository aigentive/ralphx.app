@specs/manifest.json @streams/features/backlog.md @.claude/rules/stream-verify.md @.claude/rules/gap-verification.md

# Verify Stream

## Phase 0: Recovery Check (ALWAYS FIRST)

```
1. Run: git status --porcelain streams/verify/ streams/features/backlog.md
2. Uncommitted changes exist?
   → YES: Run git diff to review changes
          Check streams/verify/activity.md for last entry
          If work looks complete → Commit with chore(verify):
          If work incomplete → Try to complete it, then commit
          Then proceed to normal workflow
   → NO: Proceed to normal workflow
```

---

Scan for gaps, output P0 items, then STOP.

## Rules
- Scan completed phases for gaps
- Output P0 items to streams/features/backlog.md
- Do NOT fix anything (that's features' job)

## Quick Workflow
```
Read manifest → Completed phases → For each: check WIRING, API, STATE, EVENTS → Gaps found? → Append P0s to features/backlog.md → Log → Commit (only your files) → STOP
```

## Git Commit Rules (CRITICAL - parallel streams)

**NEVER use `git add .` or `git add -A`** — other streams have uncommitted changes!

### Commit Lock Protocol (see .claude/rules/commit-lock.md)
```
1. Check .commit-lock:
   → NOT EXISTS? Create: echo "verify $(date -u +%Y-%m-%dT%H:%M:%S)" > .commit-lock
   → EXISTS? Read content, check if stale (>2min). If stale, delete and acquire.
            If not stale: sleep 5, re-read content (lock may change hands), loop.

2. Commit your files: git add <file1> <file2> ... && git commit

3. Release lock: rm -f .commit-lock
```

### Commit Steps
1. Acquire lock (create `.commit-lock`)
2. Only commit: `streams/features/backlog.md`, `streams/verify/activity.md`
3. Commit with prefix: `chore(verify):`
4. Release lock (delete `.commit-lock`)

## No gaps found?
Output: `<promise>COMPLETE</promise>`

---

Full workflow in: `.claude/rules/stream-verify.md`
Verification checks in: `.claude/rules/gap-verification.md`
