# Features Stream

**Required Context:** @.claude/rules/code-quality-standards.md | @.claude/rules/git-workflow.md | @.claude/rules/gap-verification.md | @.claude/rules/visual-verification.md

## Overview

The features stream handles **PRD tasks and P0 gap fixes**. It is the primary stream for shipping new functionality.

**Focus:** Implement features from active phase PRDs and fix critical gaps (P0 items).

## Rules

1. **ONE task per iteration, then STOP**
2. **P0 items BLOCK all PRD work** — fix P0 first, no exceptions
3. **No quality improvement work** — that's other streams' job (refactor, polish)
4. **TDD mandatory** — tests FIRST
5. **Document patterns inline** — new architectural patterns go in src/CLAUDE.md or src-tauri/CLAUDE.md
6. **Follow git workflow rules** — see @.claude/rules/git-workflow.md

## Recovery Check (ALWAYS FIRST)

Follow the Recovery Check in `git-workflow.md` with these ownership rules:

```
Ownership sources:
- streams/features/backlog.md (P0 items)
- Active PRD task files
- Files in streams/features/ or specs/phases/

Match if: File path appears in P0 item OR active PRD task OR is a features stream file
```

**If uncommitted files match, they are YOUR WORK. Complete and commit before proceeding.**

## Workflow

```
1. Check streams/features/backlog.md for P0 items
   → P0 EXISTS? → Fix it → Mark [x] → Commit → STOP

2. Read specs/manifest.json → find active phase (status: "active")
   → NO ACTIVE PHASE? → Output IDLE signal → END

3. Read the phase PRD → find first task with "passes": false
   → NO FAILING TASKS? → Run gap verification, then Output IDLE signal → END

4. Read FULL task (steps, acceptance_criteria, design_quality)

5. Execute task following PRD steps exactly

5.5. Visual Verification Check:
   - Did this task modify ANY file in: src/components/, src/views/, src/pages/, src/modals/, src/styles/, *.css, *.tsx?
   - YES → MUST complete steps 6.0-6.5 below (CANNOT skip)
   - NO (backend-only, tests-only, config-only) → Skip to step 7

6.0. Mock Layer Check (MANDATORY for UI tasks):
   a. Identify Tauri commands used by modified UI code
   b. Check src/api-mock/ has matching mock
   c. Missing? → Create minimal mock first
   d. Verify: `npm run dev:web` renders without undefined errors

6.5. Browser Verification (MANDATORY for UI tasks):
   a. Ensure dev server running: `curl -s http://localhost:5173 > /dev/null || npm run dev:web &`
   b. Use agent-browser skill to:
      - Navigate to the affected view
      - Interact with the modified component
      - Take screenshot: screenshots/features/YYYY-MM-DD_HH-MM-SS_[task-name].png
   c. AI-judge: Does it match PRD acceptance criteria?
   d. Visual issues? → Fix before proceeding
   e. Record screenshot path for activity log

6.9. Visual Verification Checkpoint (UI tasks only):
   - Screenshot file exists at recorded path?
   - YES → Proceed to step 7
   - NO → STOP. Cannot mark task complete without visual evidence.

7. Run linters (ONLY for what you modified):
   - Modified src/ files? → npm run lint && npm run typecheck
   - Modified src-tauri/ files? → cargo clippy --all-targets --all-features -- -D warnings && cargo test
   - Do NOT run frontend linters for backend-only changes (and vice versa)

8. Log to streams/features/activity.md

9. Update PRD: set "passes": true

10. Commit: feat|fix|docs: [description]

11. STOP — end iteration (do NOT check for IDLE here, just end)
```

**IMPORTANT:** IDLE detection happens ONLY at steps 2-3 (start of iteration). After completing a task (step 11), just end — the next iteration will check for more work.

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

1. Run code gap verification (see @.claude/rules/gap-verification.md)
   → Code gaps found? → Add P0s → Continue iterations

2. Run visual gap verification (see @.claude/rules/visual-gap-verification.md)
   → Visual gaps found? → Add P0s → Continue iterations
   → Modal testability issues? → Add P1s (non-blocking)

3. No gaps? → Update manifest.json:
   - Set current phase `"status": "complete"`
   - Set next phase `"status": "active"`
   - Update `"currentPhase": N+1`
4. Commit: `chore: complete phase N, activate phase N+1`

## All Phases Complete

When all phases in manifest.json have `"status": "complete"`:

Output: `<promise>COMPLETE</promise>`

## IDLE Detection

**When:** At the START of an iteration (steps 2-3), NOT after completing a task.

**Condition:** No work exists (no P0 items AND no active phase with failing tasks)

**Action:** Output `<promise>IDLE</promise>`

This signals the fswatch wrapper to take over and wait for file changes.

**NEVER output IDLE after completing a task.** Just end the iteration — the next iteration will find the next task.

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

**Visual Verification:** (REQUIRED for UI tasks, "N/A - backend only" for non-UI)
- Screenshot: screenshots/features/[filename].png
- Mock status: Ready | Extended [description]
- Browser test: Passed | Failed [reason]

**Result:** Success/Failed
```

## Reference

- Code gap verification: @.claude/rules/gap-verification.md
- Visual gap verification: @.claude/rules/visual-gap-verification.md
- Code quality standards: @.claude/rules/code-quality-standards.md
- Manifest and phases: `specs/manifest.json`
