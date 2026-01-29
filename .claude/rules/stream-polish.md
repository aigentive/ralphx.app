# Polish Stream

## Overview

The polish stream handles **P2/P3 cleanup, type fixes, lint fixes, and small extractions**. It ensures incremental code quality improvements get done consistently.

**Focus:** Execute P2 (medium impact) and P3 (low impact) items from the backlog.

## Rules

1. **ONE P2/P3 item per iteration, then STOP**
2. **ONLY do work from backlog.md** — cannot pick PRD tasks, P0s, or P1s
3. **Cannot skip to other work** — work items in order
4. **Verify issue still exists** — check file:line before starting
5. **Run linters after every change** — only for what you modified (cargo clippy for Rust, npm lint for TS)
6. **Only recover YOUR work** — see Recovery Check below

## Recovery Check (ALWAYS FIRST)

Before starting normal workflow, check for incomplete work from a previous iteration:

```
1. Run: git status --porcelain
   → No uncommitted changes? → Skip recovery, proceed to normal workflow

2. Read streams/polish/backlog.md → get all P2/P3 items

3. For each uncommitted file, check if it matches a BACKLOG ITEM:
   - File path matches a backlog item path? → YOURS
   - No backlog match? → NOT yours, leave alone

4. Matched files exist?
   → YES: This is YOUR incomplete work. Complete it, commit matched files, proceed.
   → NO: Leave all uncommitted files alone, proceed to normal workflow.
```

## BACKLOG = OWNERSHIP

**If uncommitted files match a backlog item, they are YOUR WORK. Period.**

## Workflow

```
1. Read streams/polish/backlog.md

2. Find first unchecked [ ] item
   → NO UNCHECKED ITEMS? → Output IDLE signal → END

3. Verify the issue still exists:
   - Read file:line referenced in the item
   - If genuinely fixed → mark ~~(stale)~~ → pick next item
   - If still exists → continue

4. Execute the cleanup/extraction:
   - P2: Error handling, small extractions, type fixes (50-150 LOC)
   - P3: Lint fixes, naming, cleanup, dead code removal (<50 LOC)

5. Run linters (ONLY for what you modified):
   - Modified src/ files? → npm run lint && npm run typecheck
   - Modified src-tauri/ files? → cargo clippy --all-targets --all-features -- -D warnings && cargo test
   - Do NOT run frontend linters for backend-only changes (and vice versa)

6. Mark [x] in backlog.md

7. Log to streams/polish/activity.md

8. Commit: refactor(scope): description

9. STOP — end iteration (do NOT check for IDLE here, just end)
```

**IMPORTANT:** IDLE detection happens ONLY at step 2 (start of iteration). After completing an item (step 9), just end — the next iteration will find the next item.

## P2/P3 Item Categories

### P2 - Medium Impact (50-150 LOC)
- Type safety improvements (replace `any` with proper types)
- Small extractions (helper functions, utilities)
- Error handling improvements
- Dependency chain fixes
- React hook optimizations (useMemo, useCallback)

### P3 - Low Impact (<50 LOC)
- Lint warning fixes
- Naming inconsistencies
- Dead code removal
- Console.log cleanup
- Import organization
- Comment cleanup

## Item Format

Items in streams/polish/backlog.md follow this format:

```markdown
## P2 - Medium Impact
- [ ] [Frontend/Backend] Description - file:line

## P3 - Low Impact
- [ ] [Frontend/Backend] Description - file:line
```

When completed:
```markdown
- [x] [Frontend/Backend] Description - file:line
```

When verified as no longer needed:
```markdown
- [ ] ~~[Frontend/Backend] Description~~ (stale - issue already fixed)
```

## Cannot Skip Rules

**This stream exists to ensure consistent quality progress.**

```
Item exists in backlog? → You MUST work on it.
- NO claiming it's "not important enough"
- NO skipping to PRD work (that's features stream)
- NO skipping to P1 work (that's refactor stream)
- NO doing P0 work (that's features stream)
```

**Why so strict?** Without dedicated attention, P2/P3 items accumulate indefinitely. "Technical debt" becomes permanent. This stream guarantees steady progress on code quality.

## Backlog Empty Detection

**When:** At the START of an iteration (step 2), NOT after completing an item.

**Condition:** streams/polish/backlog.md has no unchecked `[ ]` items

**Action:** Output `<promise>IDLE</promise>`

This signals the fswatch wrapper to take over and wait for file changes (hygiene stream refills backlog).

**NEVER output IDLE after completing an item.** Just end the iteration — the next iteration will find the next item.

## Signal Output Rules

**CRITICAL:** Completion signals must be output as a **standalone final statement**.

- Output the signal as your LAST message content
- Do NOT quote or mention the signal syntax elsewhere in your output
- When discussing signals in logs/activity, refer to them as "the IDLE signal" — never the actual `<promise>` tags

## Activity Log Format

Log entries go in `streams/polish/activity.md`:

```markdown
### YYYY-MM-DD HH:MM:SS - [Item Description]
**What:**
- File: [path]
- Change: [description of fix]

**Commands:**
- (if src-tauri/) `cargo clippy --all-targets --all-features -- -D warnings`
- (if src/) `npm run lint && npm run typecheck`

**Result:** Success/Failed
```

## Reference

- Code quality standards: `.claude/rules/code-quality-standards.md`
- Hygiene refills this backlog: `.claude/rules/stream-hygiene.md`
