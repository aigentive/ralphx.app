# Refactor Stream

Handles P1 large file splits and architectural refactors. Uses **Sonnet** model.

## Purpose

- Split files exceeding LOC limits
- Execute architectural refactoring from backlog
- Ensure codebase stays maintainable

## LOC Limits

| Type | Max Lines | Action |
|------|-----------|--------|
| Backend file | 500 | Refactor at 400 |
| Frontend component | 500 | Refactor at 400 |
| Custom hook | 300 | Extract logic |

## Workflow

```
1. Read backlog.md
2. Find first unchecked [ ] item (skip strikethrough)
3. Verify issue still exists (check LOC)
   → Fixed? → Mark ~~(stale)~~ → Pick next
4. Execute file split/refactor
5. Run linters
6. Mark [x] in backlog.md
7. Commit → STOP
```

## Extraction Patterns

**Backend:**
- `{module}_helpers.rs` - Helper functions
- `{module}_types.rs` - Structs/enums (>5)
- `{module}_validation.rs` - Validation logic

**Frontend:**
- `Component.utils.ts` - Utility functions
- `Component.types.ts` - Type definitions
- `useHook.ts` - Extracted hook logic

## Watched Files

- `streams/refactor/backlog.md`

## Output Signals

- `<promise>IDLE</promise>` - Backlog empty

## Files

- `PROMPT.md` - Stream prompt
- `backlog.md` - P1 items (populated by hygiene stream)
- `activity.md` - Activity log

## Related

- Rules: `.claude/rules/stream-refactor.md`
- LOC limits: `.claude/rules/code-quality-standards.md`
