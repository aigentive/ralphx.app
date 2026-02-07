# Mock Parity Check - Fix stale toggle after feature branch enable

## Commands Found
- `api.planBranches.enable()` → ✅ mock exists (src/api-mock/plan-branch.ts:59)
- `api.planBranches.disable()` → ✅ mock exists (src/api-mock/plan-branch.ts:76)

## Web Mode Test
- URL: http://localhost:5173 (Graph page → Plan Group → Settings gear)
- Renders: ✅ Yes — no new invoke() calls added, only error handling refactored

## Result: PASS
