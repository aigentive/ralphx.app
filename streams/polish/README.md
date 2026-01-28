# Polish Stream

Handles P2/P3 cleanup, type fixes, and lint fixes. Uses **Sonnet** model.

## Purpose

- P2 (Medium): Type safety, small extractions, error handling
- P3 (Low): Lint fixes, naming, dead code removal, console.log cleanup

## Scope Guidelines

| Priority | Impact | Typical LOC |
|----------|--------|-------------|
| P2 | Medium | 50-150 |
| P3 | Low | <50 |

## Workflow

```
1. Read backlog.md
2. Find first unchecked [ ] item
   → Skip: strikethrough ~~text~~, (excluded), (PRD:*)
3. Verify issue still exists
   → Fixed? → Mark ~~(stale)~~ → Pick next
4. Execute fix
5. Run linters
6. Mark [x] in backlog.md
7. Commit → STOP
```

## Item Categories

**P2 - Medium Impact:**
- Replace `any` types with proper types
- Extract small helper functions
- Add error handling
- React hook optimizations

**P3 - Low Impact:**
- Remove console.log/debug statements
- Fix lint warnings
- Remove dead code
- Fix naming inconsistencies

## Watched Files

- `streams/polish/backlog.md`

## Output Signals

- `<promise>IDLE</promise>` - Backlog empty (no active items)

## Files

- `PROMPT.md` - Stream prompt
- `backlog.md` - P2/P3 items (populated by hygiene stream)
- `activity.md` - Activity log

## Exclusions

- `src/components/ui/*` - shadcn/ui components (third-party)

## Related

- Rules: `.claude/rules/stream-polish.md`
- Quality standards: `.claude/rules/code-quality-standards.md`
