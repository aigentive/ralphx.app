# Mock Parity Check - PlanGroupSettings & Feature Branch Badge

## Commands Found
- `api.planBranches.getByPlan(planArtifactId)` → mock exists (returns null)
- `api.planBranches.enable({...})` → mock updated (returns mock PlanBranch)
- `api.planBranches.disable(planArtifactId)` → mock exists (returns void)

## Web Mode Test
- URL: http://localhost:5173/ (Task Graph view)
- Renders: N/A - web mode returns null for getByPlan, so feature branch badge does not appear (expected behavior - no branch = no badge)
- Settings gear only appears when projectId is set (correctly guarded)
- Enable toggle in mock returns valid PlanBranch object

## Result: PASS
