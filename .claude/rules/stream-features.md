# Features Stream

**Required Context:** @.claude/rules/code-quality-standards.md | @.claude/rules/git-workflow.md | @.claude/rules/gap-verification.md | @.claude/rules/visual-verification.md | @.claude/rules/commit-lock.md

> **Maintainer note:** This file optimizes for LLM context efficiency. Rules: (1) Tables > prose (2) One example max per concept (3) No redundant explanations (4) Use symbols: → = leads to, | = or, ❌/✅ = wrong/right (5) Before adding content, ask: "Can this be a single line?" If yes, make it one line.

## Overview

The features stream handles **PRD tasks and P0 gap fixes**. It is the primary stream for shipping new functionality.

**Focus:** Implement features from active phase PRDs and fix critical gaps (P0 items).

## Rules

| # | Rule |
|---|------|
| 1 | **ONE task per iteration, then STOP** |
| 2 | **P0 items BLOCK all PRD work** — fix P0 first, no exceptions |
| 3 | **No quality improvement work** — that's other streams' job |
| 4 | **TDD mandatory** — tests FIRST |
| 5 | **Document patterns inline** — new patterns go in src/CLAUDE.md or src-tauri/CLAUDE.md |
| 6 | **Follow git workflow rules** — see @.claude/rules/git-workflow.md |

## Recovery Check (ALWAYS FIRST)

Follow the Recovery Check in `git-workflow.md` with these ownership rules:

| Source | Match Condition |
|--------|-----------------|
| streams/features/backlog.md | File path in P0 item |
| Active PRD task files | File path in active task |
| streams/features/, specs/phases/ | Any file in these dirs |

**If uncommitted files match → YOUR WORK. Complete and commit before proceeding.**

## Workflow

### Phase: P0 Check (Step 1)

| Condition | Action |
|-----------|--------|
| P0 exists in backlog | Fix it |
| P0 tagged `[Visual/Mock]` | Re-run steps 6.0-6.9 after fix |
| P0 fixed | Mark [x] → Commit → STOP |

### Phase: Task Selection (Steps 2-4)

| Step | Check | If True | If False |
|------|-------|---------|----------|
| 2 | Active phase in manifest? | Continue | IDLE signal → END |
| 3 | Task with `"passes": false`? | Continue | Gap verification → IDLE |
| 4 | — | Read full task (steps, acceptance_criteria, design_quality) | — |

### Phase: Execution (Steps 5-11)

| Step | Action | Notes |
|------|--------|-------|
| 5 | Execute task per PRD | Follow steps exactly |
| 5.5-6.9 | Visual verification | See table below (UI tasks only) |
| 7 | Run linters | See linter table below |
| 8 | Log to activity.md | Include visual verification section |
| 9 | Update PRD | Set `"passes": true` |
| 10 | Commit | Use commit-lock protocol (see below) |
| 11 | STOP | End iteration |

### Step 5.5-6.9: Visual Verification (UI Tasks Only)

| Condition | Action |
|-----------|--------|
| Modified: src/components/, src/views/, src/pages/, src/modals/, *.tsx | MUST complete visual verification |
| Modified: backend-only, tests-only, config-only | Skip to step 7 |

**Execute per @.claude/rules/visual-verification.md, then checkpoint:**

| Checkpoint | Evidence Required | If Missing |
|------------|-------------------|------------|
| 6.0 Mock check | `screenshots/features/*_mock-check.md` | STOP → complete 6.0 |
| 6.5 Screenshot | `screenshots/features/*.png` | STOP → complete 6.5 |
| 6.5 PRD content | Screenshot shows required data (not empty) | STOP → log P0 `[Visual/Mock]` |

All pass? → Proceed to step 7.

### Step 7: Linters

| Modified | Command |
|----------|---------|
| src/ files | `npm run lint && npm run typecheck` |
| src-tauri/ files | `cargo clippy --all-targets --all-features -- -D warnings && cargo test` |

Do NOT run frontend linters for backend-only changes (and vice versa).

### Step 10: Commit (with Lock)

See @.claude/rules/commit-lock.md for full protocol.

| Requirement | Details |
|-------------|---------|
| Atomic Bash | wait lock → acquire → add → commit → release |
| Message | `feat:` \| `fix:` \| `docs:` + description |
| Co-author | `Co-Authored-By: Claude <MODEL> <noreply@anthropic.com>` — use your actual model name |

**IMPORTANT:** IDLE detection happens ONLY at steps 2-3 (start of iteration). After completing a task (step 11), just end — the next iteration will check for more work.

## P0 Rules (CANNOT BE BYPASSED)

**P0 items are phase gaps — bugs where code exists but isn't wired up.**

| Excuse | Response |
|--------|----------|
| "Too big for my task" | NO — fix it anyway |
| "Looks fine to me" | NO — verify, don't assume |
| "Defer to PRD" | NO — P0 is from COMPLETED phases |
| "Skip to easier work" | NO — P0 blocks everything |

**Why so strict?** P0 items represent shipped bugs. Every iteration without fixing P0 is an iteration where the bug remains in production.

## P0 Item Format

| Tag | When | Example |
|-----|------|---------|
| `[Frontend]` | UI wiring gap | `- [ ] [Frontend] Component not rendered - file:line` |
| `[Backend]` | API not called | `- [ ] [Backend] Command unreachable - file:line` |
| `[Visual/Mock]` | Mock missing, empty screenshot | `- [ ] [Visual/Mock] [Component]: Missing mock - file:line` |

Mark fixed: `- [x] ...`

## Phase Complete Detection

When all PRD tasks have `"passes": true`:

| Step | Action | If Gaps Found |
|------|--------|---------------|
| 1 | Run code gap verification (@.claude/rules/gap-verification.md) | Add P0s → Continue |
| 2 | Run visual gap verification (@.claude/rules/visual-gap-verification.md) | Add P0s → Continue |
| 3 | No gaps → Update manifest.json | Set current `"status": "complete"`, next `"status": "active"` |
| 4 | Commit | `chore: complete phase N, activate phase N+1` |

## All Phases Complete

When all phases in manifest.json have `"status": "complete"`:

Output: `<promise>COMPLETE</promise>`

## IDLE Detection

**When:** At the START of an iteration (steps 2-3), NOT after completing a task.

**Condition:** No work exists (no P0 items AND no active phase with failing tasks)

**Action:** Output `<promise>IDLE</promise>`

**NEVER output IDLE after completing a task.** Just end the iteration.

## Signal Output Rules

Output signals as standalone final statement. Never quote `<promise>` tags — refer to "the IDLE signal" or "the COMPLETE signal" in logs.

## Activity Log Format

Log entries go in `streams/features/activity.md`:

```markdown
### YYYY-MM-DD HH:MM:SS - [Task Title]
**What:**
- Bullet points describing work done

**Commands:**
- `relevant commands run`

**Visual Verification:** (REQUIRED for UI tasks, "N/A - backend only" for non-UI)
- Mock-check: screenshots/features/[filename]_mock-check.md
- Screenshot: screenshots/features/[filename].png
- PRD content check: ✅ Data visible | ❌ Empty/missing [logged P0]
- Browser test: Passed | Failed [reason]

**Result:** Success/Failed
```

## Reference

| Topic | Location |
|-------|----------|
| Visual verification details | @.claude/rules/visual-verification.md |
| Code gap verification | @.claude/rules/gap-verification.md |
| Visual gap verification | @.claude/rules/visual-gap-verification.md |
| Code quality standards | @.claude/rules/code-quality-standards.md |
| Commit lock protocol | @.claude/rules/commit-lock.md |
| Manifest and phases | `specs/manifest.json` |
