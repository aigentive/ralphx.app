# Plan: Split src/lib/tauri.ts

## Problem
`src/lib/tauri.ts` is 1068 lines — over double the 500-line limit for frontend files.

## Current Structure Analysis

The file contains:
1. **Core utilities** (lines 55-82): `typedInvoke`, `typedInvokeWithTransform`
2. **Health/Project/Workflow schemas** (lines 87-110): Small, reusable
3. **QA Response Schemas** (lines 112-211): ~100 lines of snake_case API schemas
4. **Review Response Schemas** (lines 213-306): ~93 lines of snake_case API schemas + input types
5. **Execution Control Schemas** (lines 307-375): ~68 lines of schemas + transforms
6. **Task Injection Schemas** (lines 377-444): ~67 lines
7. **API object** (lines 447-1067): ~620 lines organized by domain:
   - `tasks` (~134 lines)
   - `projects` (~60 lines)
   - `workflows` (~33 lines)
   - `qa` (~56 lines)
   - `reviews` (~56 lines)
   - `fixTasks` (~24 lines)
   - `execution` (~48 lines)
   - `steps` (~137 lines)
   - `testData` (~44 lines)

## Established Pattern (from ideation)

The codebase follows a domain-split pattern in `src/api/`:
- `.schemas.ts` — Zod schemas for snake_case backend responses
- `.transforms.ts` — Functions to convert snake_case → camelCase
- `.types.ts` — camelCase TypeScript interfaces for frontend
- `.ts` — Main API file with invoke wrappers and `const xxxApi = {...}`

## Proposed Split

### 1. Keep `src/lib/tauri.ts` as core (~100 lines)
- `typedInvoke`, `typedInvokeWithTransform` utilities
- `HealthResponseSchema` (small, universal)
- Re-exports for backward compatibility
- Aggregate `api` object that merges all domain APIs

### 2. Create domain-specific API modules

| File | Contents | Est. Lines |
|------|----------|------------|
| `src/api/tasks.ts` | tasks + steps API | ~280 |
| `src/api/tasks.schemas.ts` | InjectTaskResponseSchema | ~30 |
| `src/api/tasks.transforms.ts` | transformInjectTaskResponse | ~20 |
| `src/api/projects.ts` | projects + workflows API | ~110 |
| `src/api/qa-api.ts` | qa API (avoid name collision with types/qa.ts) | ~80 |
| `src/api/qa-api.schemas.ts` | QA response schemas (snake_case) | ~100 |
| `src/api/reviews-api.ts` | reviews + fixTasks API | ~100 |
| `src/api/reviews-api.schemas.ts` | Review response schemas (snake_case) | ~90 |
| `src/api/execution.ts` | execution API | ~80 |
| `src/api/execution.schemas.ts` | ExecutionStatus schemas | ~50 |
| `src/api/execution.transforms.ts` | Transform functions | ~30 |
| `src/api/test-data.ts` | testData API | ~60 |

### 3. Backward Compatibility

**54 files** import from `@/lib/tauri`. Key patterns:
- Most import `api` directly: `import { api } from "@/lib/tauri"`
- Some also import types: `import { api, ReviewResponse, ExecutionStatusResponse } from "@/lib/tauri"`

**Strategy:** Re-export everything from tauri.ts to maintain all existing imports.

```typescript
// src/lib/tauri.ts (final ~150 lines)

// Core utilities (keep here)
export async function typedInvoke<T>(...) { ... }
export async function typedInvokeWithTransform<TRaw, TResult>(...) { ... }

// Health (universal, keep here)
export const HealthResponseSchema = z.object({ status: z.string() });
export type HealthResponse = z.infer<typeof HealthResponseSchema>;

// Re-export domain APIs + their types
export { tasksApi, stepsApi, type InjectTaskInput, type InjectTaskResponse } from "@/api/tasks";
export { projectsApi, workflowsApi, getGitBranches } from "@/api/projects";
export { qaApi, type TaskQAResponse, type QAResultsResponse, type UpdateQASettingsInput, ... } from "@/api/qa-api";
export { reviewsApi, fixTasksApi, type ReviewResponse, type ReviewNoteResponse, ... } from "@/api/reviews-api";
export { executionApi, type ExecutionStatusResponse, type ExecutionCommandResponse, ... } from "@/api/execution";
export { testDataApi } from "@/api/test-data";

// Aggregate object for backward compat
export const api = {
  health: { check: () => typedInvoke("health_check", {}, HealthResponseSchema) },
  tasks: tasksApi,
  projects: projectsApi,
  workflows: workflowsApi,
  qa: qaApi,
  reviews: reviewsApi,
  fixTasks: fixTasksApi,
  execution: executionApi,
  steps: stepsApi,
  testData: testDataApi,
} as const;
```

## File Naming

Use `-api` suffix for files that would collide with `src/types/`:
- `qa-api.ts` (not `qa.ts` — avoids collision with `types/qa.ts`)
- `reviews-api.ts` (not `review.ts` — avoids collision with `types/review.ts`)

## Implementation Tasks

### Task 1: Create execution API (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `refactor(api): extract execution API from tauri.ts`

Create `src/api/execution.ts`, `src/api/execution.schemas.ts`, `src/api/execution.transforms.ts`
- Extract `ExecutionStatusResponseSchema`, `ExecutionCommandResponseSchema`
- Extract `transformExecutionStatus`, `transformExecutionCommand`
- Extract `ExecutionStatusResponse`, `ExecutionCommandResponse` interfaces
- Create `executionApi` object with getStatus, pause, resume, stop

Files:
- `src/api/execution.schemas.ts` (new, ~50 lines)
- `src/api/execution.transforms.ts` (new, ~30 lines)
- `src/api/execution.ts` (new, ~80 lines)

### Task 2: Create test-data API
**Dependencies:** Task 1
**Atomic Commit:** `refactor(api): extract test-data API from tauri.ts`

Create `src/api/test-data.ts`
- Extract `testData` object as `testDataApi`
- Uses only inline z.object schemas (no external schemas needed)

Files:
- `src/api/test-data.ts` (new, ~60 lines)

### Task 3: Create projects API (BLOCKING)
**Dependencies:** Task 2
**Atomic Commit:** `refactor(api): extract projects API from tauri.ts`

Create `src/api/projects.ts`
- Extract `projects` and `workflows` objects as `projectsApi`, `workflowsApi`
- Extract `getGitBranches` function
- Import transforms from `@/types/project`, `@/types/workflow`

Files:
- `src/api/projects.ts` (new, ~110 lines)

### Task 4: Create QA API (BLOCKING)
**Dependencies:** Task 3
**Atomic Commit:** `refactor(api): extract QA API from tauri.ts`

Create `src/api/qa-api.ts`, `src/api/qa-api.schemas.ts`
- Extract QA response schemas: `AcceptanceCriterionResponseSchema`, `QATestStepResponseSchema`, etc.
- Extract `TaskQAResponseSchema`, `QAResultsResponseSchema`
- Extract `UpdateQASettingsInput` interface
- Create `qaApi` object

Files:
- `src/api/qa-api.schemas.ts` (new, ~100 lines)
- `src/api/qa-api.ts` (new, ~80 lines)

### Task 5: Create reviews API (BLOCKING)
**Dependencies:** Task 4
**Atomic Commit:** `refactor(api): extract reviews API from tauri.ts`

Create `src/api/reviews-api.ts`, `src/api/reviews-api.schemas.ts`
- Extract review schemas: `ReviewResponseSchema`, `ReviewActionResponseSchema`, etc.
- Extract input types: `ApproveReviewInput`, `RequestChangesInput`, etc.
- Create `reviewsApi`, `fixTasksApi` objects

Files:
- `src/api/reviews-api.schemas.ts` (new, ~90 lines)
- `src/api/reviews-api.ts` (new, ~100 lines)

### Task 6: Create tasks API (BLOCKING)
**Dependencies:** Task 5
**Atomic Commit:** `refactor(api): extract tasks API from tauri.ts`

Create `src/api/tasks.ts`, `src/api/tasks.schemas.ts`, `src/api/tasks.transforms.ts`
- Extract `InjectTaskResponseSchemaRaw`, `InjectTaskResponse`, `InjectTaskInput`
- Extract `transformInjectTaskResponse`
- Create `tasksApi`, `stepsApi` objects (largest extraction)

Files:
- `src/api/tasks.schemas.ts` (new, ~30 lines)
- `src/api/tasks.transforms.ts` (new, ~20 lines)
- `src/api/tasks.ts` (new, ~280 lines)

### Task 7: Update tauri.ts with re-exports
**Dependencies:** Task 6
**Atomic Commit:** `refactor(lib): consolidate tauri.ts with domain re-exports`

Update `src/lib/tauri.ts`:
- Remove all extracted code
- Add re-exports from all domain API modules
- Keep `typedInvoke`, `typedInvokeWithTransform` utilities
- Keep `HealthResponseSchema` and health check
- Create aggregate `api` object that composes all domain APIs

Files:
- `src/lib/tauri.ts` (modify, 1068 → ~150 lines)

### Task 8: Verify extraction
**Dependencies:** Task 7
**Atomic Commit:** None (verification only)

Run verification commands:
- `npm run typecheck` — no type errors
- `npm run lint` — no lint errors
- `npm test` — all tests pass

## Critical Files

- `src/lib/tauri.ts` (1068 → ~150 lines)
- `src/api/tasks.ts` (new, ~280 lines)
- `src/api/projects.ts` (new, ~110 lines)
- `src/api/qa-api.ts` (new, ~80 lines)
- `src/api/reviews-api.ts` (new, ~100 lines)
- `src/api/execution.ts` (new, ~80 lines)
- `src/api/test-data.ts` (new, ~60 lines)

## Commit Lock Workflow (Parallel Agent Coordination)

Reference: `.claude/rules/commit-lock.md`

### Before Committing
```bash
# 1. Establish project root (works from any subdirectory)
PROJECT_ROOT="$(git rev-parse --show-toplevel)"

# 2. Check/acquire lock
if [ -f "$PROJECT_ROOT/.commit-lock" ]; then
  # Read lock content, wait 3s, retry up to 30s
  # If stale (same content >30s), delete and proceed
fi

# 3. Create lock
echo "<stream-name> $(date -u +%Y-%m-%dT%H:%M:%S)" > "$PROJECT_ROOT/.commit-lock"

# 4. Stage and commit
git -C "$PROJECT_ROOT" add <files>
git -C "$PROJECT_ROOT" commit -m "message"
```

### After Committing
```bash
# ALWAYS release lock (success or failure)
rm -f "$PROJECT_ROOT/.commit-lock"
```

### Lock Rules
1. Acquire lock BEFORE `git add`
2. Release lock AFTER commit (success OR failure)
3. Stale = same content + >30 sec old
4. Never force-delete active lock from another agent
