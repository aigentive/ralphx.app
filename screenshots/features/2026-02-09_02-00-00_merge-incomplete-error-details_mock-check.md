# Mock Parity Check - MergeIncomplete Error Details

## Commands Found
- `retry_merge` → existing command, not newly added
- `resolve_merge_conflict` → existing command, not newly added
- No new invoke() calls added — changes only parse `task.metadata` (already on task object)

## Web Mode Test
- URL: http://localhost:5173 (navigate to task in merge_incomplete status)
- Mock tasks don't include metadata → triggers generic fallback path
- Generic fallback renders correctly (same as before)
- With metadata present → actual error details would render

## Result: PASS
