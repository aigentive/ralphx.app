# Polish Stream

## Overview

The polish stream handles **P2/P3 cleanup, type fixes, lint fixes, and small extractions**. It ensures incremental code quality improvements get done consistently.

**Focus:** Execute P2 (medium impact) and P3 (low impact) items from the backlog.

## Rules

1. **ONE P2/P3 item per iteration, then STOP**
2. **ONLY do work from backlog.md** — cannot pick PRD tasks, P0s, or P1s
3. **Cannot skip to other work** — work items in order
4. **Verify issue still exists** — check file:line before starting
5. **Run linters after every change** — cargo clippy, npm run lint

## Workflow

```
1. Read streams/polish/backlog.md

2. Find first unchecked [ ] item

3. Verify the issue still exists:
   - Read file:line referenced in the item
   - If genuinely fixed → mark ~~(stale)~~ → pick next item
   - If still exists → continue

4. Execute the cleanup/extraction:
   - P2: Error handling, small extractions, type fixes (50-150 LOC)
   - P3: Lint fixes, naming, cleanup, dead code removal (<50 LOC)

5. Run linters:
   - cargo clippy --all-targets --all-features -- -D warnings
   - cargo test
   - npm run lint && npm run typecheck

6. Mark [x] in backlog.md

7. Log to streams/polish/activity.md

8. Commit: refactor(scope): description

9. STOP — one item per iteration
```

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

When streams/polish/backlog.md has no unchecked `[ ]` items:

Output: `<promise>COMPLETE</promise>`

This signals the orchestrator to skip this stream until hygiene refills it.

## Activity Log Format

Log entries go in `streams/polish/activity.md`:

```markdown
### YYYY-MM-DD HH:MM:SS - [Item Description]
**What:**
- File: [path]
- Change: [description of fix]

**Commands:**
- `cargo clippy --all-targets --all-features -- -D warnings`
- `npm run lint && npm run typecheck`

**Result:** Success/Failed
```

## Reference

- Code quality standards: `.claude/rules/code-quality-standards.md`
- Hygiene refills this backlog: `.claude/rules/stream-hygiene.md`
