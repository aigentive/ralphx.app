# Refactor Stream

## Overview

The refactor stream handles **P1 large file splits and architectural refactors only**. It ensures large, complex refactoring work gets done without being avoided in favor of easier tasks.

**Focus:** Split files exceeding LOC limits and execute architectural refactors from the P1 backlog.

## Rules

1. **ONE P1 item per iteration, then STOP**
2. **ONLY do P1 work from backlog.md** — cannot pick PRD tasks, P0s, P2s, or P3s
3. **Cannot skip to easier work** — there is no easier work in this stream
4. **Must verify LOC limits** — reference `.claude/rules/code-quality-standards.md` before starting
5. **Run linters after every change** — cargo clippy, npm run lint

## Workflow

```
1. Read streams/refactor/backlog.md

2. Find first unchecked [ ] item

3. Verify the issue still exists:
   - Read file:line referenced in the item
   - Check LOC against code-quality-standards.md limits
   - If genuinely fixed → mark ~~(stale)~~ → pick next item
   - If still exists → continue

4. Execute the file split/refactoring:
   - Follow extraction patterns from code-quality-standards.md
   - Backend: Extract to {module}_helpers.rs, {module}_types.rs, {module}_tests.rs
   - Frontend: Extract to Component.utils.ts, Component.types.ts, useHook.ts

5. Run linters:
   - cargo clippy --all-targets --all-features -- -D warnings
   - cargo test
   - npm run lint && npm run typecheck

6. Mark [x] in backlog.md

7. Log to streams/refactor/activity.md

8. Commit: refactor(scope): description

9. STOP — one item per iteration
```

## LOC Limits Reference

From `.claude/rules/code-quality-standards.md`:

### Backend (src-tauri/)
| Condition | Max Lines | Action |
|-----------|-----------|--------|
| **Any file** | **500** | Refactor at 400 lines |
| Helper functions | 100 | Extract to `{module}_helpers.rs` |
| >5 structs/enums | N/A | Extract to `{module}_types.rs` |
| Service method | 50 | Extract helper |

### Frontend (src/)
| File Type | Max Lines | Trigger |
|-----------|-----------|---------|
| Component | 500 | Refactor at 400 |
| Custom Hook | 300 | |

**Key Principle:** "Well-organized" is not an excuse for exceeding limits. A file exceeding LOC limits needs extraction, period.

## P1 Item Format

Items in streams/refactor/backlog.md follow this format:

```markdown
- [ ] Split [filename] ([current] LOC) - extract [description] - file:line
```

When completed:
```markdown
- [x] Split [filename] ([current] LOC → [new] LOC) - extracted [description] - file:line
```

When verified as no longer needed:
```markdown
- [ ] ~~Split [filename] - [description]~~ (stale - now under limit)
```

## Cannot Skip Rules

**This stream exists to prevent scope avoidance.** Large refactors are uncomfortable but necessary.

```
P1 item exists in backlog? → You MUST work on it. Period.
- NO claiming it's "too big" (that's the point)
- NO picking a different item (work in order)
- NO doing PRD work instead (that's features stream)
- NO doing P2/P3 work instead (that's polish stream)
```

**Why so strict?** Without a dedicated stream, large refactors get perpetually skipped. Files grow to 2000+ LOC because "there's always something more urgent." This stream guarantees progress on structural debt.

## Backlog Empty Detection

When streams/refactor/backlog.md has no unchecked `[ ]` items:

Output: `<promise>IDLE</promise>`

This signals the fswatch wrapper to take over and wait for file changes (hygiene stream refills backlog).

## Activity Log Format

Log entries go in `streams/refactor/activity.md`:

```markdown
### YYYY-MM-DD HH:MM:SS - [Split/Refactor Description]
**What:**
- Original file: [path] ([N] LOC)
- Extracted to: [new files]
- New size: [M] LOC

**Commands:**
- `wc -l [files]`
- `cargo clippy --all-targets --all-features -- -D warnings`

**Result:** Success/Failed
```

## Reference

- LOC limits and extraction patterns: `.claude/rules/code-quality-standards.md`
- Hygiene refills this backlog: `.claude/rules/stream-hygiene.md`
