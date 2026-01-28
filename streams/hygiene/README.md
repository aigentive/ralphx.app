# Hygiene Stream

Backlog maintenance and refilling. Uses **Sonnet** model.

## Purpose

- Archive completed items when count > 10
- Refill backlogs when active items < 3
- Validate strikethrough items periodically
- Keep backlogs healthy so other streams never starve

## Important

**This stream does NOT fix code.** It only maintains the task lists.

## Workflow

```
1. ARCHIVE: Check each backlog for >10 [x] items
   → Move oldest to streams/archive/completed.md

2. REFILL: Check backlog active item counts
   → refactor/backlog.md < 3? → Explore for P1 issues
   → polish/backlog.md < 3? → Explore for P2/P3 issues

3. VALIDATE: Check 2-3 strikethrough items
   → Issue still exists? → Unmark, make active
   → Issue gone? → Increment counter → Archive at :2

4. Commit → STOP
```

## Backlog Health Targets

| Backlog | Min Active | Max Completed |
|---------|------------|---------------|
| refactor | 3 | 10 |
| polish | 3 | 10 |

## Strikethrough Validation

Items marked as stale progress through validation:

```
(stale) → (stale:1) → (stale:2) → archive
```

Three total checks before confirmed archival.

## Watched Files

- `streams/refactor/backlog.md`
- `streams/polish/backlog.md`
- `streams/features/backlog.md`
- `streams/archive/completed.md`

**10-minute delay:** Unlike other streams (3s delay), hygiene uses a 10-minute
debounce to batch changes and prevent running after every commit.

## Output Signals

- `<promise>IDLE</promise>` - Nothing to maintain

## Explore Prompts

**For P1 (refactor):**
- Files exceeding LOC limits
- Large file split opportunities

**For P2/P3 (polish):**
- Type safety issues (`any` types)
- Console.log statements
- Dead code, lint warnings

## Files

- `PROMPT.md` - Stream prompt
- `activity.md` - Activity log

## Related

- Rules: `.claude/rules/stream-hygiene.md`
- LOC limits: `.claude/rules/code-quality-standards.md`
