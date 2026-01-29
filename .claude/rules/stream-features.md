# Features Stream

## Overview

The features stream handles **PRD tasks and P0 gap fixes**. It is the primary stream for shipping new functionality.

**Focus:** Implement features from active phase PRDs and fix critical gaps (P0 items).

## Rules

1. **ONE task per iteration, then STOP**
2. **P0 items BLOCK all PRD work** — fix P0 first, no exceptions
3. **No quality improvement work** — that's other streams' job (refactor, polish)
4. **TDD mandatory** — tests FIRST
5. **Document patterns inline** — new architectural patterns go in src/CLAUDE.md or src-tauri/CLAUDE.md
6. **Only recover YOUR work** — see Recovery Check below

## Recovery Check (ALWAYS FIRST)

Before starting normal workflow, check for incomplete work from a previous iteration:

```
1. Run: git status --porcelain
   → No uncommitted changes? → Skip recovery, proceed to normal workflow

2. Identify uncommitted files and check ownership:
   a. Read streams/features/activity.md → scan ALL entries (not just last)
   b. Read streams/features/backlog.md → get all P0 items
   c. Read active PRD → get current task files
   d. For each uncommitted file, check if it CORRELATES:
      - File path matches an activity log entry (any entry, not just last)? → YES
      - File path matches a P0 backlog item? → YES
      - File path matches active PRD task? → YES
      - Files in streams/features/ or specs/phases/? → YES
      - None of the above? → NO, belongs to another stream

3. Correlated changes exist?
   → YES: Complete the work if needed, commit ONLY correlated files, then proceed
   → NO: Leave uncommitted files alone, proceed to normal workflow
```

**CRITICAL:** Only commit files that match your activity log, backlog, or active PRD. No match → not yours.

## Workflow

```
1. Check streams/features/backlog.md for P0 items
   → P0 EXISTS? → Fix it → Mark [x] → Commit → STOP

2. Read specs/manifest.json → find active phase (status: "active")

3. Read the phase PRD → find first task with "passes": false

4. Read FULL task (steps, acceptance_criteria, design_quality)

5. Execute task following PRD steps exactly

6. Run linters (ONLY for what you modified):
   - Modified src/ files? → npm run lint && npm run typecheck
   - Modified src-tauri/ files? → cargo clippy --all-targets --all-features -- -D warnings && cargo test
   - Do NOT run frontend linters for backend-only changes (and vice versa)

7. Log to streams/features/activity.md

8. Update PRD: set "passes": true

9. Commit: feat|fix|docs: [description]

10. STOP — one task per iteration
```

## P0 Rules (CANNOT BE BYPASSED)

**P0 items are phase gaps — bugs where code exists but isn't wired up.**

```
P0 EXISTS? → You MUST fix it. Period.
- NO scope matching ("too big for my task")
- NO stale marking ("looks fine to me")
- NO PRD deferral (P0 comes from COMPLETED phases, not active PRD)
- NO skipping to easier work
```

**Why so strict?** P0 items represent shipped bugs. The feature "works" in isolation but users can't access it. Every iteration that passes without fixing P0 is an iteration where the bug remains in production.

## P0 Item Format

Items in streams/features/backlog.md follow this format:

```markdown
- [ ] [Frontend/Backend] Description - file:line
```

When fixed:
```markdown
- [x] [Frontend/Backend] Description - file:line
```

## Phase Complete Detection

When all PRD tasks have `"passes": true`:

1. Run gap verification (see `.claude/rules/gap-verification.md`)
2. Gaps found? → Add to streams/features/backlog.md as P0 → Continue iterations
3. No gaps? → Update manifest.json:
   - Set current phase `"status": "complete"`
   - Set next phase `"status": "active"`
   - Update `"currentPhase": N+1`
4. Commit: `chore: complete phase N, activate phase N+1`

## All Phases Complete

When all phases in manifest.json have `"status": "complete"`:

Output: `<promise>COMPLETE</promise>`

## IDLE Detection

When **no work exists** (no P0 items AND no active phase with failing tasks):

Output: `<promise>IDLE</promise>`

This signals the fswatch wrapper to take over and wait for file changes.

## Signal Output Rules

**CRITICAL:** Completion signals must be output as a **standalone final statement**.

- Output the signal as your LAST message content
- Do NOT quote or mention the signal syntax elsewhere in your output
- When discussing signals in logs/activity, refer to them as "the IDLE signal" or "the COMPLETE signal" — never the actual `<promise>` tags

## Activity Log Format

Log entries go in `streams/features/activity.md`:

```markdown
### YYYY-MM-DD HH:MM:SS - [Task Title]
**What:**
- Bullet points describing work done

**Commands:**
- `relevant commands run`

**Result:** Success/Failed
```

## Reference

- Gap verification workflow: `.claude/rules/gap-verification.md`
- Code quality standards: `.claude/rules/code-quality-standards.md`
- Manifest and phases: `specs/manifest.json`
