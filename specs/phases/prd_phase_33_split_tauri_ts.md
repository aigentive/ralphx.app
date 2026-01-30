# RalphX - Phase 33: Split src/lib/tauri.ts

## Overview

`src/lib/tauri.ts` is 1068 lines — over double the 500-line limit for frontend files. This phase extracts the monolithic file into domain-specific API modules following the established pattern in `src/api/`, maintaining full backward compatibility through re-exports.

The refactoring creates separate files for each API domain (execution, test-data, projects, QA, reviews, tasks) with their associated schemas and transforms, then consolidates `tauri.ts` as a thin re-export layer.

**Reference Plan:**
- `specs/plans/split_tauri_ts.md` - Detailed extraction plan with file mappings and implementation steps

## Goals

1. Reduce `src/lib/tauri.ts` from 1068 lines to ~150 lines
2. Create domain-specific API modules in `src/api/` following existing patterns
3. Maintain 100% backward compatibility for all 54+ importing files
4. Apply the snake_case boundary pattern consistently across all new modules

## Dependencies

### Phase 32 (API Serialization Convention) - Required

| Dependency | Why Needed |
|------------|------------|
| snake_case boundary pattern | All new API modules must follow the established serialization convention |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/split_tauri_ts.md`
2. Understand the architecture and component structure
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Write functional tests where appropriate
4. Run linters for modified code only (frontend: `npm run lint && npm run typecheck`)
5. Commit with descriptive message

---

## Git Workflow (Parallel Agent Coordination)

**Before each commit, follow the commit lock protocol:**

Reference: `.claude/rules/commit-lock.md`

1. Establish project root: `PROJECT_ROOT="$(git rev-parse --show-toplevel)"`
2. Acquire lock before `git add` (see commit-lock.md § Protocol)
3. Stage and commit using `git -C "$PROJECT_ROOT"`
4. Release lock after commit: `rm -f "$PROJECT_ROOT/.commit-lock"`

**Commit message conventions** (see `.claude/rules/git-workflow.md`):
- Refactor stream: `refactor(scope):`

**Task Execution Order:**
- Tasks with `"blockedBy": []` can start immediately
- Before starting a task, check `blockedBy` - all listed tasks must have `"passes": true`
- Execute tasks in ID order when dependencies are satisfied

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/split_tauri_ts.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "frontend",
    "description": "Create execution API module with schemas and transforms",
    "plan_section": "Task 1: Create execution API",
    "blocking": [2, 3, 4, 5, 6, 7],
    "blockedBy": [],
    "atomic_commit": "refactor(api): extract execution API from tauri.ts",
    "steps": [
      "Read specs/plans/split_tauri_ts.md section 'Task 1: Create execution API'",
      "Create src/api/execution.schemas.ts with ExecutionStatusResponseSchema, ExecutionCommandResponseSchema",
      "Create src/api/execution.transforms.ts with transformExecutionStatus, transformExecutionCommand",
      "Create src/api/execution.ts with executionApi object (getStatus, pause, resume, stop)",
      "Export ExecutionStatusResponse, ExecutionCommandResponse interfaces",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(api): extract execution API from tauri.ts"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "frontend",
    "description": "Create test-data API module",
    "plan_section": "Task 2: Create test-data API",
    "blocking": [7],
    "blockedBy": [1],
    "atomic_commit": "refactor(api): extract test-data API from tauri.ts",
    "steps": [
      "Read specs/plans/split_tauri_ts.md section 'Task 2: Create test-data API'",
      "Create src/api/test-data.ts with testDataApi object",
      "Include inline z.object schemas for seed responses",
      "Export testDataApi",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(api): extract test-data API from tauri.ts"
    ],
    "passes": false
  },
  {
    "id": 3,
    "category": "frontend",
    "description": "Create projects API module with workflows and git branches",
    "plan_section": "Task 3: Create projects API",
    "blocking": [7],
    "blockedBy": [1],
    "atomic_commit": "refactor(api): extract projects API from tauri.ts",
    "steps": [
      "Read specs/plans/split_tauri_ts.md section 'Task 3: Create projects API'",
      "Create src/api/projects.ts with projectsApi, workflowsApi objects",
      "Extract getGitBranches function",
      "Import transforms from @/types/project, @/types/workflow",
      "Export projectsApi, workflowsApi, getGitBranches",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(api): extract projects API from tauri.ts"
    ],
    "passes": false
  },
  {
    "id": 4,
    "category": "frontend",
    "description": "Create QA API module with schemas",
    "plan_section": "Task 4: Create QA API",
    "blocking": [7],
    "blockedBy": [1],
    "atomic_commit": "refactor(api): extract QA API from tauri.ts",
    "steps": [
      "Read specs/plans/split_tauri_ts.md section 'Task 4: Create QA API'",
      "Create src/api/qa-api.schemas.ts with all QA response schemas",
      "Create src/api/qa-api.ts with qaApi object",
      "Export UpdateQASettingsInput interface",
      "Export TaskQAResponse, QAResultsResponse, AcceptanceCriterionResponse types",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(api): extract QA API from tauri.ts"
    ],
    "passes": false
  },
  {
    "id": 5,
    "category": "frontend",
    "description": "Create reviews API module with schemas and input types",
    "plan_section": "Task 5: Create reviews API",
    "blocking": [7],
    "blockedBy": [1],
    "atomic_commit": "refactor(api): extract reviews API from tauri.ts",
    "steps": [
      "Read specs/plans/split_tauri_ts.md section 'Task 5: Create reviews API'",
      "Create src/api/reviews-api.schemas.ts with ReviewResponseSchema, ReviewActionResponseSchema, etc.",
      "Create src/api/reviews-api.ts with reviewsApi, fixTasksApi objects",
      "Export all input types: ApproveReviewInput, RequestChangesInput, RejectReviewInput, etc.",
      "Export ReviewResponse, ReviewNoteResponse, FixTaskAttemptsResponse types",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(api): extract reviews API from tauri.ts"
    ],
    "passes": false
  },
  {
    "id": 6,
    "category": "frontend",
    "description": "Create tasks API module with schemas and transforms (largest extraction)",
    "plan_section": "Task 6: Create tasks API",
    "blocking": [7],
    "blockedBy": [1],
    "atomic_commit": "refactor(api): extract tasks API from tauri.ts",
    "steps": [
      "Read specs/plans/split_tauri_ts.md section 'Task 6: Create tasks API'",
      "Create src/api/tasks.schemas.ts with InjectTaskResponseSchemaRaw",
      "Create src/api/tasks.transforms.ts with transformInjectTaskResponse",
      "Create src/api/tasks.ts with tasksApi, stepsApi objects (~280 lines)",
      "Export InjectTaskInput, InjectTaskResponse types",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(api): extract tasks API from tauri.ts"
    ],
    "passes": false
  },
  {
    "id": 7,
    "category": "frontend",
    "description": "Update tauri.ts with re-exports and aggregate api object",
    "plan_section": "Task 7: Update tauri.ts with re-exports",
    "blocking": [8],
    "blockedBy": [1, 2, 3, 4, 5, 6],
    "atomic_commit": "refactor(lib): consolidate tauri.ts with domain re-exports",
    "steps": [
      "Read specs/plans/split_tauri_ts.md section 'Task 7: Update tauri.ts with re-exports'",
      "Remove all extracted code from src/lib/tauri.ts",
      "Keep typedInvoke, typedInvokeWithTransform utilities",
      "Keep HealthResponseSchema and health check",
      "Add re-exports from all domain API modules",
      "Create aggregate api object that composes all domain APIs",
      "Verify all 54+ importing files still work (no import changes needed)",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(lib): consolidate tauri.ts with domain re-exports"
    ],
    "passes": false
  },
  {
    "id": 8,
    "category": "frontend",
    "description": "Verify extraction with full test suite",
    "plan_section": "Task 8: Verify extraction",
    "blocking": [],
    "blockedBy": [7],
    "atomic_commit": null,
    "steps": [
      "Run npm run typecheck - verify no type errors",
      "Run npm run lint - verify no lint errors",
      "Run npm test - verify all tests pass",
      "Verify src/lib/tauri.ts is under 200 lines",
      "Verify all new API files are under 300 lines each"
    ],
    "passes": false
  }
]
```

**Task field definitions:**
- `id`: Sequential integer starting at 1
- `blocking`: Task IDs that cannot start until THIS task completes
- `blockedBy`: Task IDs that must complete before THIS task can start (inverse of blocking)
- `atomic_commit`: Commit message for this task

---

## Key Architecture Decisions

| Decision | Rationale |
|----------|-----------|
| **Domain-specific API files** | Follows established pattern in src/api/, enables parallel work |
| **Re-export for backward compat** | All 54+ files importing from @/lib/tauri continue working unchanged |
| **-api suffix for collision avoidance** | qa-api.ts and reviews-api.ts avoid collision with types/qa.ts, types/review.ts |
| **snake_case boundary pattern** | Schemas expect snake_case, transforms convert to camelCase |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Frontend - Run `npm run test`
- [ ] All existing tests pass without modification
- [ ] No new type errors introduced

### Build Verification (run only for modified code)
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`npm run build`)

### Manual Testing
- [ ] App starts successfully with existing dev server
- [ ] All API calls work (tasks, projects, workflows, QA, reviews, execution)

### Wiring Verification

**For each new API module, verify the full path:**

- [ ] `executionApi` methods (getStatus, pause, resume, stop) work
- [ ] `testDataApi` methods (seed, seedVisualAudit, clear) work
- [ ] `projectsApi` and `workflowsApi` methods work
- [ ] `qaApi` methods work
- [ ] `reviewsApi` and `fixTasksApi` methods work
- [ ] `tasksApi` and `stepsApi` methods work
- [ ] Aggregate `api` object provides all methods

**Common failure modes to check:**
- [ ] No missing exports in re-export statements
- [ ] No circular dependencies between API modules
- [ ] All type exports are properly forwarded

See `.claude/rules/gap-verification.md` for full verification workflow.
