# Code Quality Standards

> Single source of truth for file size limits and extraction triggers.

## File Size Limits

### Backend (src-tauri/)

| Condition | Max Lines | Action |
|-----------|-----------|--------|
| **Any file** | **500** | Refactor at 400 lines |
| Helper functions | 100 | Extract to `{module}_helpers.rs` |
| >5 structs/enums | N/A | Extract to `{module}_types.rs` |
| Service method | 50 | Extract helper |
| Validation | 30 | Extract to `{module}_validation.rs` |

### Frontend (src/)

| File Type | Max Lines | Trigger |
|-----------|-----------|---------|
| Component | 500 | Refactor at 400 |
| Custom Hook | 300 | |
| Presentational | 200 | Pure display only |

**Extraction Triggers:**
- >3 useState/useCallback in component → extract hook
- >4 props → composition pattern
- >3 conditional branches → extract sub-components
- Handler >10 lines → extract to hook

### Plugin (ralphx-plugin/)

| File Type | Max Lines |
|-----------|-----------|
| Component | 100 |
| Hook | 100 |
| Store | 150 |
| Skill | 150 |
| Agent | 100 |

## Key Principle

**"Well-organized" is not an excuse for exceeding limits.**

A file can be perfectly structured and still need extraction if it exceeds LOC limits. The limits exist to:
- Improve readability and navigation
- Enable focused testing
- Reduce cognitive load
- Facilitate parallel work

## Verification

When evaluating if a file needs extraction:
1. Check actual line count: `wc -l <file>`
2. Compare against limits above
3. **Exceeds limit = needs extraction (not optional)**

## References

This file is the canonical source. Referenced by:
- `.claude/rules/stream-refactor.md`
- `.claude/rules/stream-hygiene.md`
- `src-tauri/CLAUDE.md`
- `src/CLAUDE.md`
- `streams/refactor/backlog.md`
