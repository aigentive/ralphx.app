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

## File Extraction Best Practices

When splitting a file into multiple files or a module folder:

1. **Atomic commits** - New files and deletion of original must be in the same commit. Never leave orphaned originals.

2. **No backup files** - Never create `.bak` files. Git is your backup. Use `git restore` to recover if needed.

3. **Verify deletion** - Before committing, confirm the original file no longer exists:
   ```bash
   ls <original_path>  # Should fail / not found
   ```

4. **One extraction = one commit** - Don't split the work across multiple commits with the same message.

5. **COPY, don't rewrite** - When extracting code to new files:
   - **Read the original implementation first** using `git show HEAD:<file_path>` or `Read`
   - **Copy the exact function signatures and implementations** - do NOT invent new APIs
   - **Verify types and repos exist** before using them (e.g., check AppState, entity fields)
   - **Verify compilation after each extraction** before proceeding to the next:
     - Backend: `cargo check`
     - Frontend: `npm run typecheck`

   **Bad:** Writing `task.steps.clone()` without verifying Task has a `steps` field
   **Bad:** Using `artifact_session_link_repo` without checking if it exists in AppState
   **Good:** Reading original implementation, copying exact logic, verifying it compiles

6. **Extraction validation** - Before committing any extraction:
   ```bash
   # Backend extractions
   cargo check 2>&1 | head -20

   # Frontend extractions
   npm run typecheck 2>&1 | head -20
   ```
   If errors exist, do NOT commit. Fix or revert.

## References

This file is the canonical source. Referenced by:
- `.claude/rules/stream-refactor.md`
- `.claude/rules/stream-hygiene.md`
- `src-tauri/CLAUDE.md`
- `src/CLAUDE.md`
- `streams/refactor/backlog.md`
