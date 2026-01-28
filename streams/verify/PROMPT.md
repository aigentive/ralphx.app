@specs/manifest.json @streams/features/backlog.md @.claude/rules/stream-verify.md @.claude/rules/gap-verification.md

# Verify Stream

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

1. Only commit: `streams/features/backlog.md`, `streams/verify/activity.md`
2. `git add <file1> <file2>` — only files you modified
3. Commit with prefix: `chore(verify):`

## No gaps found?
Output: `<promise>COMPLETE</promise>`

---

Full workflow in: `.claude/rules/stream-verify.md`
Verification checks in: `.claude/rules/gap-verification.md`
