# Hygiene Stream

## Overview

The hygiene stream handles **backlog maintenance, refilling via Explore, and archiving completed items**. It ensures backlogs stay healthy and streams never starve for work.

**Focus:** Maintain backlog health across all streams. Do NOT fix code.

## Rules

1. **ONE maintenance cycle per iteration, then STOP**
2. **Do NOT fix code** — that's other streams' job
3. **Do NOT pick items to work on** — only maintain the lists
4. **Archive completed items when count > 10**
5. **Refill backlogs when active items < 3**
6. **Validate strikethroughs periodically**

## Workflow

```
1. ARCHIVE: Check each backlog for >10 [x] items
   → Move excess to streams/archive/completed.md (oldest first)
   → Include date and original section

2. REFILL REFACTOR: Check streams/refactor/backlog.md
   → Active items < 3? → Run Explore agent for P1 issues → Append results

3. REFILL POLISH: Check streams/polish/backlog.md
   → Active items < 3? → Run Explore agent for P2/P3 issues → Append results

4. VALIDATE STRIKETHROUGHS: Pick 2-3 deferred items (oldest first)
   → Skip (excluded) items
   → Read file:line to verify current state
   → Issue still exists? → Unmark, make active
   → Issue gone? → Increment counter:
     (reason) → (reason:1) → (reason:2) → move to archive

5. Log to streams/hygiene/activity.md

6. Commit if changes made: chore(hygiene): backlog maintenance

7. STOP
```

## Archive Protocol

**When:** Any backlog has >10 completed `[x]` items.

**Process:**
1. Count `[x]` items in each backlog section
2. If count > 10, identify oldest completed items
3. Move to `streams/archive/completed.md` with format:

```markdown
## Archived YYYY-MM-DD

### From refactor/backlog.md (P1)
- [x] Split example.rs (800 LOC → 400 LOC) - src-tauri/src/example.rs

### From polish/backlog.md (P2/P3)
- [x] Remove console.log - src/App.tsx:45
```

## Refill Protocol

**When:** A backlog has fewer than 3 active `[ ]` items.

**Process:**
1. Count active `[ ]` items (not strikethrough, not completed)
2. If count < 3, launch Explore agent with appropriate prompt
3. Append discovered issues to the relevant backlog

### Explore Prompt for P1 (refactor/backlog.md)

```
Scan src/ and src-tauri/ for P1 code quality issues. Find ONLY:
- Files exceeding LOC limits (backend: 500, frontend component: 500, hook: 300)
- Large file split opportunities (>400 LOC is refactor trigger)
- Architectural extraction opportunities

LOC limits reference: .claude/rules/code-quality-standards.md

Output as markdown checklist:
- [ ] Split [filename] ([LOC] LOC) - extract [description] - file:line

Group by: Backend, Frontend
Maximum 10 items.
```

### Explore Prompt for P2/P3 (polish/backlog.md)

```
Scan src/ and src-tauri/ for P2/P3 code quality issues. Find:
- P2 (Medium): Type safety issues (any types), small extractions, error handling
- P3 (Low): Lint warnings, dead code, console.log statements, naming issues

Exclude: src/components/ui/* (shadcn/ui)

Output as markdown checklist:
- [ ] [P2/P3] [Frontend/Backend] Description - file:line

Group by: P2 - Medium Impact, P3 - Low Impact
Maximum 15 items total.
```

## Deferred Validation Protocol

**When:** After archive and refill, if time permits.

**Purpose:** Ensure strikethrough items are genuinely resolved, not just marked incorrectly.

**Process:**
1. Pick 2-3 oldest strikethrough items (skip `(excluded)`)
2. Read the file:line referenced in each item
3. For each item:

```
Issue still exists?
→ YES: Unmark the strikethrough, make active again
       - [ ] ~~text~~ (stale) becomes - [ ] text
       Pick this one for next stream iteration

→ NO:  Increment validation counter
       (stale) → (stale:1) → (stale:2) → move to archive
       (PRD:N) → (PRD:N:1) → (PRD:N:2) → move to archive
```

**Counter progression:**
- First mark: `(stale)` or `(PRD:N)`
- First validation (still resolved): `(stale:1)` or `(PRD:N:1)`
- Second validation (still resolved): `(stale:2)` or `(PRD:N:2)` → archive

Three total checks (original mark + 2 revalidations) before confirmed archival.

## Backlog Health Targets

| Backlog | Min Active | Max Completed | Action When Low | Action When High |
|---------|------------|---------------|-----------------|------------------|
| refactor | 3 | 10 | Explore P1 | Archive |
| polish | 3 | 10 | Explore P2/P3 | Archive |
| features | N/A | 10 | Verify produces | Archive |

Note: features/backlog.md is populated by the verify stream, not hygiene.

## Cannot Fix Code

**This stream exists ONLY for backlog maintenance.**

```
Hygiene stream does NOT:
- Fix any issues (that's refactor/polish streams)
- Pick items to work on (that's the stream that owns the backlog)
- Create new code (that's features stream)
- Verify gaps (that's verify stream)
```

**Why separate?** Combining maintenance with fixing creates scope creep. Hygiene ensures healthy backlogs exist; other streams consume them.

## IDLE Detection

If all conditions are met:
- All backlogs have < 10 `[x]` items (no archiving needed)
- All backlogs have >= 3 active `[ ]` items (no refill needed)
- No strikethrough items to validate

Output: `<promise>IDLE</promise>`

This signals the fswatch wrapper to take over and wait for backlog file changes.

## Activity Log Format

Log entries go in `streams/hygiene/activity.md`:

```markdown
### YYYY-MM-DD HH:MM:SS - Backlog Maintenance
**Archive:**
- Moved [N] items from refactor/backlog.md to archive
- Moved [M] items from polish/backlog.md to archive

**Refill:**
- Added [X] P1 items to refactor/backlog.md
- Added [Y] P2/P3 items to polish/backlog.md

**Validation:**
- Checked [Z] strikethrough items
- Reactivated: [list if any]
- Archived: [list if any]

**Result:** Maintenance complete | No changes needed
```

## Reference

- LOC limits: `.claude/rules/code-quality-standards.md`
- Verify stream (produces P0s): `.claude/rules/stream-verify.md`
- Refactor stream (consumes P1s): `.claude/rules/stream-refactor.md`
- Polish stream (consumes P2/P3s): `.claude/rules/stream-polish.md`
