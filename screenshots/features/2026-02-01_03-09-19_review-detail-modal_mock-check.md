# Mock Parity Check - ReviewDetailModal

## Commands Found
- `api.tasks.get` → ✅ mock exists (src/api-mock/tasks.ts)
- `api.reviews.getByTaskId` → ✅ EXTENDED mock (returns review array)
- `api.reviews.getTaskStateHistory` → ✅ EXTENDED mock (returns review history)
- `api.reviews.approveTask` → ✅ mock exists (no-op in web mode)
- `api.reviews.requestTaskChanges` → ✅ mock exists (no-op in web mode)
- `useGitDiff` → ✅ Uses internal mock data (no Tauri invoke)

## Mock Extensions
- Added `getByTaskId` implementation to return AI review for hasAiReview check
- Added `getTaskStateHistory` implementation to return review history timeline (2 entries: AI approved + human requested changes)

## Web Mode Test
- URL: http://localhost:5173/kanban
- Opens via: Reviews panel → Click "Review" on review_passed task
- Renders: ✅ Yes (task details, AI summary, review history, DiffViewer, action buttons)

## Result: PASS
