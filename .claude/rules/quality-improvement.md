# Quality Improvement Loop

## Overview

Every code task requires a `refactor:` commit. Use `logs/code-quality.md` to track issues.

## Workflow

```
1. Read logs/code-quality.md
2. P0 EXISTS? → MUST pick P0. No exceptions. No scope matching. No deferral.
3. No P0? → Pick by task scope: small=P3, medium=P2, large=P1
4. VERIFY (P1/P2/P3 only — P0 is NEVER deferred):
   a. Issue still exists? (read file:line)
   b. NOT in active PRD? (cross-reference with current phase task list)
   c. CRITICAL: Check against documented LOC limits (see below)
   ├── Valid & not in PRD? → Execute → Mark [x] → Commit
   ├── Stale (issue genuinely fixed)? → Strikethrough ~~text~~ (stale) → Pick next
   └── In PRD? → Strikethrough ~~text~~ (PRD) → Pick next
5. No valid items at current scope? → ESCALATE: P3→P2→P1→Explore
6. ALL items exhausted/marked? → Run DEFERRED VALIDATION (below)
7. Still nothing? → Launch Explore agent → Replenish → Pick ONE → Execute
```

## P0 Rules (CANNOT BE BYPASSED)

**P0 items are phase gaps — bugs where code exists but isn't wired up.**

```
P0 EXISTS? → You MUST fix it. Period.
- NO scope matching ("too big for my task")
- NO stale marking ("looks fine to me")
- NO PRD deferral (P0 comes from COMPLETED phases, not active PRD)
- NO deferred validation (P0 cannot be validated away)
```

**Why so strict?** P0 items represent shipped bugs. The feature "works" in isolation but users can't access it. Every iteration that passes without fixing P0 is an iteration where the bug remains in production.

## Deferred Validation Protocol

**When:** After all active P1/P2/P3 items are exhausted, before launching Explore agent.

**Applies to:** P1/P2/P3 strikethrough items EXCEPT `(excluded)`.

**Does NOT apply to:** P0 items. P0 cannot be deferred or strikethrough'd.

**Process:**
```
1. Pick 2-3 deferred items (oldest first, skip excluded)
2. Read file:line to verify current state
3. Issue still exists?
   → YES: Unmark, make active, pick this one for current task
   → NO:  Increment counter
          (reason) → (reason:1) → (reason:2) → move to archive
```

**Counter progression:**
- `(stale)` or `(PRD:N)` — first deferral
- `(stale:1)` or `(PRD:N:1)` — validated once, still resolved
- `(stale:2)` or `(PRD:N:2)` — validated twice, move to `logs/code-quality-archive.md`

**3 total checks** (original mark + 2 revalidations) before archiving as confirmed.

## Cleanup Protocol

**When:** `[x]` count exceeds 10 per section.

**Process:**
1. Count completed items per section (Frontend P1/P2/P3, Backend P1/P2/P3)
2. If count > 10, move oldest `[x]` items to `logs/code-quality-archive.md`
3. Archive format: include date and original section

## CRITICAL: LOC Limits Verification

**Before marking ANY LOC/extraction item as stale, you MUST verify against documented limits.**

**Reference:** `.claude/rules/code-quality-standards.md` (single source of truth)

Quick summary:
- **Backend:** 500 lines max (refactor at 400)
- **Frontend component:** 500 lines max (refactor at 400)
- **Frontend hook:** 300 lines max
- **Plugin files:** 100-150 lines max

**A file exceeding these limits is NOT "well-organized" — it is a valid extraction target.**

**"Stale" means the issue was FIXED, not that you subjectively think the code is fine.**

## CRITICAL: No Skipping Allowed

**Quality improvement is MANDATORY. "Nothing to do" is NOT an excuse.**

If your scope tier is exhausted:
- P3 exhausted → Pick from P2
- P2 exhausted → Pick from P1
- P1 exhausted → Launch Explore agent to find new issues

**The iteration does NOT complete until a `refactor:` commit exists.**

**Why verify?**
- Other tasks or parallel agents may have already fixed the issue
- Some issues may be planned work in the active PRD — don't duplicate effort

## Exclusions

**Do NOT scan or pick items from these paths:**
- `src/components/ui/*` — shadcn/ui components (upgraded externally)

If an item references an excluded path, mark it: `[ ] ~~text~~ (excluded)`

## Priority & Scope Matching

| Priority | When | Pick Order | Can Defer? |
|----------|------|------------|------------|
| **P0 - Critical** | Gaps from phase verification | **ALWAYS FIRST** | **NO — NEVER** |
| P1 - High | Architecture, major refactors | Large tasks (>150 LOC) | Yes (stale/PRD) |
| P2 - Medium | Error handling, extraction | Medium tasks (50-150 LOC) | Yes (stale/PRD) |
| P3 - Low | Lint, naming, cleanup | Small tasks (<50 LOC) | Yes (stale/PRD) |

**P0 blocks ALL other work.** You cannot pick P1/P2/P3 while P0 exists.

## Quality Targets

### Frontend (src/)
- Replace `any` with proper types
- Fix naming inconsistencies
- Add missing error handling
- Remove dead code
- Extract repeated logic into hooks/functions
- Fix lint warnings

### Backend (src-tauri/)
- Fix clippy warnings
- Improve error handling (domain-specific variants)
- Fix naming inconsistencies
- Remove dead code
- Extract repeated logic into helpers

## Explore Agent Prompt

When `logs/code-quality.md` is empty, use this prompt:

```
Scan src/ and src-tauri/ for code quality issues. Find:
- Type safety issues (any types, missing error handling)
- Dead code, unused imports
- Naming inconsistencies
- Clippy/lint warnings
- Extraction opportunities (repeated logic, large functions)

Output as markdown checklist:
- [ ] [P1/P2/P3] [Frontend/Backend] Description - file:line

Group by: Frontend P1, P2, P3 then Backend P1, P2, P3
```

## File Format: logs/code-quality.md

```markdown
## P0 - Critical (Phase Gaps)

> Items here BLOCK all other quality work. Fix before picking P1/P2/P3.

- [ ] [Frontend] Orphaned: View Registry not wired - src/components/tasks/TaskDetailOverlay.tsx:508

## Frontend (src/)

### P1 - High Impact
- [ ] Replace `any` in useChat hook - src/hooks/useChat.ts:45

### P2 - Medium Impact
- [ ] Extract validation logic - src/components/Form.tsx:120-150

### P3 - Low Impact
- [ ] Fix unused import - src/utils/helpers.ts:3

## Backend (src-tauri/)
[same structure: P1, P2, P3]

## Last Explored
**Date:** YYYY-MM-DD HH:MM
**Areas:** src/, src-tauri/
```

**P0 section is at the TOP** — agents see it first and cannot miss it.

## TODO Tracking (During Task Execution)

**When you add a TODO comment during task work, log it immediately.**

If you write any of these patterns:
- `// TODO:` or `/* TODO: */`
- `// FIXME:` or `/* FIXME: */`
- `# TODO:` (in scripts/config)

**Immediately add to `logs/code-quality.md`:**
```markdown
- [ ] [P2/P3] Implement TODO: [description] - file:line
```

**Priority assignment:**
- P2: Functional gaps (missing error handling, incomplete implementation)
- P3: Cleanup/optimization (refactoring, naming, performance)

**Why?** TODOs added during development are easily forgotten. Logging them ensures they're tracked and eventually addressed.

## Skip Conditions

Quality improvement NOT required for:
- Pure documentation changes
- Configuration-only changes

## Verification

Task is NOT complete until:
1. `refactor:` commit exists in git log
2. Completed item marked `[x]` in `logs/code-quality.md`
