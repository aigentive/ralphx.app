# Mock Parity Check - PlanningView Reopen/Reset Header Actions

## Commands Found
- `reopen_ideation_session` → ✅ mock exists at src/api-mock/ideation.ts:135
- `apply_proposals_to_kanban` → ✅ mock exists (used by useResetAndReaccept chained call)

## Web Mode Test
- URL: http://localhost:5173/ (ideation view with accepted/archived session)
- Renders: ✅ Yes — PlanningView renders, hooks use mocked API

## Result: PASS
