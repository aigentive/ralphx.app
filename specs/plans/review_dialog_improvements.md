# Review Dialog Improvements Plan

## Summary
Fix data discrepancies and visual issues in the ReviewDetailModal component.

## Tasks

### 1. Commit Sorting - Chronological Order
**Dependencies:** None
**Atomic Commit:** `fix(diff): reverse commit order to chronological`

**File:** `src/hooks/useGitDiff.ts:116`
**Change:** Add `.reverse()` after mapping commits
```typescript
// Current (line 116):
setCommits(commitInfos.map(toCommit));

// Fixed:
setCommits(commitInfos.map(toCommit).reverse());
```

### 2. Changes Tab - Show Full Git Diff
**Dependencies:** None
**Atomic Commit:** `fix(diff): use git diff for file changes instead of activity events`

**File:** `src-tauri/src/application/diff_service.rs:61-133`

**Problem:** Current implementation filters by activity events (Write/Edit tool calls only)

**Fix:** Replace activity-event filtering with direct git diff:
```rust
pub async fn get_task_file_changes(
    &self,
    _task_id: &TaskId,
    project_path: &str,
    base_branch: &str,
) -> AppResult<Vec<FileChange>> {
    // Use git diff --name-status to get all changed files
    let output = Command::new("git")
        .args(["diff", "--name-status", base_branch])
        .current_dir(project_path)
        .output()
        .map_err(|e| AppError::GitError(e.to_string()))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut changes = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 2 {
            let status = match parts[0] {
                "A" => FileChangeStatus::Added,
                "D" => FileChangeStatus::Deleted,
                _ => FileChangeStatus::Modified,
            };
            changes.push(FileChange {
                path: parts[1].to_string(),
                status,
                additions: 0,
                deletions: 0,
            });
        }
    }

    changes.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(changes)
}
```

### 3. Remove Duplicate Task Title
**Dependencies:** None
**Atomic Commit:** `fix(reviews): remove duplicate task title from sidebar`

**File:** `src/components/reviews/ReviewDetailModal.tsx`

**Problem:** Title in header (line 461) AND in TaskContextSection (lines 73-81)

**Fix:** Remove title from TaskContextSection, keep only in header:
```tsx
// Lines 73-81 - Remove or comment out:
{/* Title removed - already in modal header */}
{/* <h3 data-testid="modal-task-title" ...>{title}</h3> */}
```

Or simplify TaskContextSection to only show priority, category, description.

### 4. AI Review Summary - Render as Markdown
**Dependencies:** None
**Atomic Commit:** `feat(reviews): render AI review summary as markdown`

**File:** `src/components/reviews/ReviewDetailModal.tsx:168-171`

**Current:**
```tsx
<p className="text-[12px] text-white/60" ...>
  {latestApproved.notes}
</p>
```

**Fix:** Import and use ReactMarkdown:
```tsx
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

// In AIReviewSummary (line 168-171):
<div className="text-[12px] text-white/60 prose prose-sm prose-invert max-w-none">
  <ReactMarkdown remarkPlugins={[remarkGfm]}>
    {latestApproved.notes}
  </ReactMarkdown>
</div>
```

### 5. Review History - Use Summary Field
**Dependencies:** None
**Atomic Commit:** `fix(reviews): use summary field in review history`

**File:** `src/components/reviews/ReviewDetailModal.tsx:267-271`

**Current:**
```tsx
{entry.notes && (
  <p className="text-[11px] text-white/40 truncate mt-0.5">
    {entry.notes}
  </p>
)}
```

**Fix:** Use summary field (short plain text designed for display):
```tsx
{entry.summary && (
  <p className="text-[11px] text-white/40 truncate mt-0.5">
    {entry.summary}
  </p>
)}
```

### 6. Dialog Size - Near Full Screen
**Dependencies:** None
**Atomic Commit:** `feat(reviews): increase dialog size to near full screen`

**File:** `src/components/reviews/ReviewDetailModal.tsx`

**Changes:**
| Location | Current | New |
|----------|---------|-----|
| Line 439 | `w-[90vw] h-[85vh]` | `w-[95vw] h-[95vh]` |
| Line 481 | `w-[300px]` | `w-[400px]` |

```tsx
// Line 439:
"max-w-[95vw] w-[95vw] h-[95vh]"

// Line 481:
"w-[400px] shrink-0 flex flex-col border-r overflow-hidden"
```

## Critical Files

| File | Changes |
|------|---------|
| `src/hooks/useGitDiff.ts` | Line 116: reverse commits |
| `src-tauri/src/application/diff_service.rs` | Lines 61-133: git diff instead of activity events |
| `src/components/reviews/ReviewDetailModal.tsx` | Layout, markdown, title, summary field |

## Verification

1. Start dev server: `npm run tauri dev`
2. Open a task in `review_passed` state
3. Click "Review Code" → dialog opens
4. **Check commits:** Should be oldest first (chronological)
5. **Check changes:** Should show all modified files (matching commits total)
6. **Check title:** Only in header, not repeated in sidebar
7. **Check AI summary:** Renders markdown headings, lists, etc.
8. **Check review history:** Shows clean summary excerpts
9. **Check dialog size:** Large, fills most of screen with 400px sidebar

## Dependencies

- `react-markdown` and `remark-gfm` already in package.json (used elsewhere)
- No new dependencies needed

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)

### Task Independence Analysis

All 6 tasks are **independent** (no blocking dependencies):
- Tasks 1 & 2: Different files, different layers (frontend hook vs backend service)
- Tasks 3, 4, 5, 6: Same file but independent changes (no shared code paths)

**Compilation Unit Validation:**
- Task 1: Adds `.reverse()` - no type changes, compiles independently ✅
- Task 2: Simplifies function body - same signature, compiles independently ✅
- Task 3: Removes JSX element - no references, compiles independently ✅
- Task 4: Adds markdown rendering - additive change, compiles independently ✅
- Task 5: Uses existing `summary` field - verify field exists in type ⚠️
- Task 6: CSS class changes - no code dependencies, compiles independently ✅

**Note for Task 5:** Verify `ReviewEntry` type has `summary` field before implementing. If missing, add the field to the type definition in the same task.
