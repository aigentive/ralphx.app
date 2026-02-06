# Mock Parity Check - Plan Branch Mock + Project Settings Toggle

## Commands Found
- `api.projects.changeGitMode` -> mock exists (mockProjectsApi)
- `api.projects.update` -> mock exists (mockProjectsApi)
- `getGitDefaultBranch` -> mock exists (mockGetGitDefaultBranch)
- `api.planBranches.updateProjectSetting` -> mock exists (mockPlanBranchApi)
- `api.planBranches.getByPlan` -> mock exists (mockPlanBranchApi)
- `api.planBranches.getByProject` -> mock exists (mockPlanBranchApi)
- `api.planBranches.enable` -> mock exists (mockPlanBranchApi)
- `api.planBranches.disable` -> mock exists (mockPlanBranchApi)

## Web Mode Test
- URL: http://localhost:5173/settings
- Renders: Expected (Settings page with Git section including Feature Branches toggle)

## Result: PASS
