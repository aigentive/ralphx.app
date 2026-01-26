# RalphX - Phase 19: Task Execution Experience

## Overview

This phase introduces deterministic progress tracking via **Task Steps** and enhances the execution UX with **reactive TaskCards** and a **full-screen TaskFullView** with integrated chat. Together, these features create a cohesive execution experience where users can see real progress on the Kanban board and dive deep into any task.

**Reference Plan:**
- `specs/plans/task-execution-experience.md` - Complete implementation plan with architecture, components, and detailed specifications

## Goals

1. Add Task Steps data model with status tracking (pending, in_progress, completed, skipped, failed, cancelled)
2. Create MCP tools for worker agent to update step progress during execution
3. Auto-import steps from proposal when creating tasks from ideation
4. Add reactive TaskCard visuals (pulsing animations, progress dots, duration badge)
5. Create full-screen TaskFullView with split layout (details + embedded chat)
6. Implement context-aware chat that switches based on task state (execution/review/discussion)
7. Show step progress on TaskCard ("3/7") and in TaskFullView (step list with status)

## Dependencies

### Phase 18 (Task CRUD, Archive & Search) - Required

| Dependency | Why Needed |
|------------|------------|
| TaskDetailModal with edit mode | TaskFullView builds on modal content |
| TaskCard with context menu | Add progress indicators to existing card |
| uiStore patterns | Modal/view management |
| Task entity with sourceProposalId | Steps imported from linked proposal |
| ExecutionChatService | Chat embedded in TaskFullView |
| ChatPanel component | Refactored for embedding |
| MCP tool infrastructure | New step tools follow same pattern |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/task-execution-experience.md`
2. Understand the architecture, data flow, and component structure
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Write functional tests where appropriate (unit tests, not manual/e2e)
4. Run `npm run lint && npm run typecheck` and `cargo clippy --all-targets --all-features -- -D warnings`
5. Commit with descriptive message

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/task-execution-experience.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "category": "backend",
    "description": "Create TaskStep entity and TaskStepStatus enum",
    "plan_section": "Data Model: TaskStep",
    "steps": [
      "Read specs/plans/task-execution-experience.md section 'Data Model: TaskStep'",
      "Create src-tauri/src/domain/entities/task_step.rs:",
      "  - Add TaskStepId newtype (like TaskId)",
      "  - Add TaskStepStatus enum: Pending, InProgress, Completed, Skipped, Failed, Cancelled",
      "  - Add #[serde(rename_all = \"snake_case\")] to enum",
      "  - Add TaskStep struct with fields: id, task_id, title, description, status, sort_order,",
      "    depends_on, created_by, completion_note, created_at, updated_at, started_at, completed_at",
      "  - Implement TaskStep::new(task_id, title, sort_order, created_by)",
      "  - Implement can_start(), is_terminal() helper methods",
      "  - Implement from_row() for SQLite deserialization",
      "Update src-tauri/src/domain/entities/mod.rs to export TaskStep, TaskStepId, TaskStepStatus",
      "Write unit tests for entity creation and status helpers",
      "Run cargo test",
      "Commit: feat(entities): add TaskStep entity and TaskStepStatus enum"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Create database migration for task_steps table",
    "plan_section": "Data Model: TaskStep - Database Schema",
    "steps": [
      "Read specs/plans/task-execution-experience.md section 'Database Schema'",
      "Add migration to src-tauri/src/infrastructure/sqlite/migrations.rs:",
      "  - CREATE TABLE task_steps with all fields",
      "  - Add FOREIGN KEY to tasks(id) ON DELETE CASCADE",
      "  - CREATE INDEX idx_task_steps_task_id ON task_steps(task_id)",
      "  - CREATE INDEX idx_task_steps_task_order ON task_steps(task_id, sort_order)",
      "Update SCHEMA_VERSION constant",
      "Run cargo test to verify migration applies",
      "Commit: feat(db): add task_steps table migration"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Create TaskStepRepository trait",
    "plan_section": "Data Model: TaskStep",
    "steps": [
      "Create src-tauri/src/domain/repositories/task_step_repository.rs:",
      "  - Define #[async_trait] TaskStepRepository trait",
      "  - Add create(&self, step: TaskStep) -> AppResult<TaskStep>",
      "  - Add get_by_id(&self, id: &TaskStepId) -> AppResult<Option<TaskStep>>",
      "  - Add get_by_task(&self, task_id: &TaskId) -> AppResult<Vec<TaskStep>>",
      "  - Add get_by_task_and_status(&self, task_id: &TaskId, status: TaskStepStatus) -> AppResult<Vec<TaskStep>>",
      "  - Add update(&self, step: &TaskStep) -> AppResult<()>",
      "  - Add delete(&self, id: &TaskStepId) -> AppResult<()>",
      "  - Add delete_by_task(&self, task_id: &TaskId) -> AppResult<()>",
      "  - Add count_by_status(&self, task_id: &TaskId) -> AppResult<HashMap<TaskStepStatus, u32>>",
      "  - Add bulk_create(&self, steps: Vec<TaskStep>) -> AppResult<Vec<TaskStep>>",
      "  - Add reorder(&self, task_id: &TaskId, step_ids: Vec<TaskStepId>) -> AppResult<()>",
      "Update src-tauri/src/domain/repositories/mod.rs to export trait",
      "Commit: feat(repository): add TaskStepRepository trait"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Implement SqliteTaskStepRepository",
    "plan_section": "Data Model: TaskStep",
    "steps": [
      "Create src-tauri/src/infrastructure/sqlite/sqlite_task_step_repo.rs:",
      "  - Implement TaskStepRepository for SqliteTaskStepRepository",
      "  - Use parameterized queries for all operations",
      "  - Order get_by_task results by sort_order ASC",
      "  - Implement bulk_create with transaction",
      "  - Implement reorder with transaction (update sort_order for each step)",
      "Update src-tauri/src/infrastructure/sqlite/mod.rs to export",
      "Write unit tests for all repository methods",
      "Run cargo test",
      "Commit: feat(sqlite): implement SqliteTaskStepRepository"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Implement MemoryTaskStepRepository for tests",
    "plan_section": "Data Model: TaskStep",
    "steps": [
      "Create src-tauri/src/infrastructure/memory/memory_task_step_repo.rs:",
      "  - Use Arc<Mutex<HashMap<TaskStepId, TaskStep>>> for storage",
      "  - Implement all TaskStepRepository trait methods",
      "  - Filter and sort in-memory for get_by_task",
      "Update src-tauri/src/infrastructure/memory/mod.rs to export",
      "Write unit tests to verify behavior matches SQLite implementation",
      "Run cargo test",
      "Commit: feat(memory): implement MemoryTaskStepRepository for tests"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Add TaskStepRepository to AppState",
    "plan_section": "Data Model: TaskStep",
    "steps": [
      "Update src-tauri/src/application/app_state.rs:",
      "  - Add task_step_repo: Arc<dyn TaskStepRepository> field",
      "  - Initialize SqliteTaskStepRepository in new_production()",
      "  - Initialize MemoryTaskStepRepository in new_test()",
      "Run cargo test to verify DI works",
      "Commit: feat(app_state): add TaskStepRepository to AppState"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Create StepProgressSummary struct",
    "plan_section": "Data Model: TaskStep - Progress Summary",
    "steps": [
      "Add to src-tauri/src/domain/entities/task_step.rs:",
      "  - StepProgressSummary struct with fields:",
      "    task_id: String, total: u32, completed: u32, in_progress: u32,",
      "    pending: u32, skipped: u32, failed: u32,",
      "    current_step: Option<TaskStep>, next_step: Option<TaskStep>,",
      "    percent_complete: f32",
      "  - Implement StepProgressSummary::from_steps(task_id: &TaskId, steps: &[TaskStep])",
      "  - Calculate percent_complete as (completed + skipped) / total * 100",
      "  - current_step = first InProgress step",
      "  - next_step = first Pending step",
      "Write unit tests for summary calculation",
      "Run cargo test",
      "Commit: feat(entities): add StepProgressSummary struct"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Create task step CRUD commands",
    "plan_section": "MCP Tools for Worker Agent",
    "steps": [
      "Create src-tauri/src/commands/task_step_commands.rs:",
      "  - create_task_step(task_id, title, description?, sort_order?) -> TaskStep",
      "  - get_task_steps(task_id) -> Vec<TaskStep>",
      "  - update_task_step(step_id, title?, description?, sort_order?) -> TaskStep",
      "  - delete_task_step(step_id) -> ()",
      "  - reorder_task_steps(task_id, step_ids: Vec<String>) -> Vec<TaskStep>",
      "  - get_step_progress(task_id) -> StepProgressSummary",
      "Register commands in src-tauri/src/lib.rs invoke_handler",
      "Update src-tauri/src/commands/mod.rs",
      "Write unit tests for each command",
      "Run cargo test",
      "Commit: feat(commands): add task step CRUD commands"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Create step status transition commands",
    "plan_section": "MCP Tools for Worker Agent",
    "steps": [
      "Add to src-tauri/src/commands/task_step_commands.rs:",
      "  - start_step(step_id) -> TaskStep",
      "    - Validates step is Pending",
      "    - Sets status to InProgress, started_at to now",
      "    - Emits 'step:updated' event",
      "  - complete_step(step_id, note?) -> TaskStep",
      "    - Validates step is InProgress",
      "    - Sets status to Completed, completed_at to now, completion_note",
      "    - Emits 'step:updated' event",
      "  - skip_step(step_id, reason) -> TaskStep",
      "    - Validates step is Pending or InProgress",
      "    - Sets status to Skipped, completed_at to now, completion_note = reason",
      "    - Emits 'step:updated' event",
      "  - fail_step(step_id, error) -> TaskStep",
      "    - Validates step is InProgress",
      "    - Sets status to Failed, completed_at to now, completion_note = error",
      "    - Emits 'step:updated' event",
      "Register commands in lib.rs",
      "Write unit tests for valid/invalid transitions",
      "Run cargo test",
      "Commit: feat(commands): add step status transition commands with events"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Add step HTTP endpoints for MCP",
    "plan_section": "MCP Tools for Worker Agent",
    "steps": [
      "Add to src-tauri/src/http_server.rs:",
      "  - GET /api/task_steps/:task_id -> Vec<StepResponse>",
      "  - POST /api/start_step { step_id } -> StepResponse",
      "  - POST /api/complete_step { step_id, note? } -> StepResponse",
      "  - POST /api/skip_step { step_id, reason } -> StepResponse",
      "  - POST /api/fail_step { step_id, error } -> StepResponse",
      "  - POST /api/add_step { task_id, title, description?, after_step_id? } -> StepResponse",
      "  - GET /api/step_progress/:task_id -> StepProgressSummary",
      "Add StepResponse struct with id, task_id, title, description, status, sort_order,",
      "  completion_note, started_at, completed_at",
      "Add request structs: StartStepRequest, CompleteStepRequest, SkipStepRequest,",
      "  FailStepRequest, AddStepRequest",
      "Run cargo test",
      "Commit: feat(http): add step endpoints for MCP"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Update worker agent tool scoping for steps",
    "plan_section": "MCP Tools for Worker Agent",
    "steps": [
      "Update tool scoping logic in http_server.rs or where RALPHX_AGENT_TYPE is checked:",
      "  - Worker agent gets: get_task_steps, start_step, complete_step, skip_step,",
      "    fail_step, add_step, get_step_progress",
      "Update src-tauri/CLAUDE.md with new tool scoping table",
      "Run cargo test",
      "Commit: feat(mcp): add step tools to worker agent scope"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Import steps from proposal when creating task",
    "plan_section": "Integration",
    "steps": [
      "Update src-tauri/src/application/apply_service.rs or task creation logic:",
      "  - When creating task from proposal (source_proposal_id is set):",
      "  - If proposal.steps is Some and non-empty:",
      "    - Parse JSON array of step titles",
      "    - Create TaskStep for each with created_by='proposal'",
      "    - Use bulk_create to insert all steps",
      "Write test: create task from proposal with steps -> steps exist on task",
      "Run cargo test",
      "Commit: feat(apply): import steps from proposal when creating task"
    ],
    "passes": true
  },
  {
    "category": "backend",
    "description": "Include steps in TaskContext for worker",
    "plan_section": "Worker Agent Instructions",
    "steps": [
      "Update src-tauri/src/domain/entities/task_context.rs (or wherever TaskContext is):",
      "  - Add steps: Vec<TaskStep> field",
      "  - Add step_progress: Option<StepProgressSummary> field",
      "Update get_task_context_impl in http_server.rs:",
      "  - Fetch steps via task_step_repo.get_by_task()",
      "  - Calculate StepProgressSummary",
      "  - Include in TaskContext response",
      "Add context_hint: 'Task has N steps defined - use get_task_steps to see them'",
      "Run cargo test",
      "Commit: feat(context): include steps in TaskContext for worker"
    ],
    "passes": true
  },
  {
    "category": "agent",
    "description": "Update worker agent prompt with step instructions",
    "plan_section": "Worker Agent Instructions",
    "steps": [
      "Update ralphx-plugin/agents/worker.md:",
      "  - Add '## Step Progress Tracking' section",
      "  - Instruct: At start, call get_task_steps(task_id)",
      "  - Instruct: Before each step, call start_step(step_id)",
      "  - Instruct: After each step, call complete_step(step_id, note?)",
      "  - Instruct: If step not needed, call skip_step(step_id, reason)",
      "  - Instruct: If step fails, call fail_step(step_id, error)",
      "  - Instruct: If no steps exist, create them using add_step",
      "  - Add example flow: get_task_steps -> start_step -> [work] -> complete_step",
      "Commit: docs(agent): add step progress instructions to worker prompt"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Add TaskStep types and schemas",
    "plan_section": "UI Components",
    "steps": [
      "Create src/types/task-step.ts:",
      "  - TaskStepStatusSchema = z.enum(['pending', 'in_progress', 'completed', 'skipped', 'failed', 'cancelled'])",
      "  - TaskStepSchema = z.object({ id, taskId, title, description, status, sortOrder,",
      "      dependsOn, createdBy, completionNote, createdAt, updatedAt, startedAt, completedAt })",
      "  - StepProgressSummarySchema = z.object({ taskId, total, completed, inProgress,",
      "      pending, skipped, failed, currentStep, nextStep, percentComplete })",
      "  - Export types: TaskStep, TaskStepStatus, StepProgressSummary",
      "Update src/types/index.ts to export",
      "Run npm run typecheck",
      "Commit: feat(types): add TaskStep types and schemas"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Add step API bindings",
    "plan_section": "UI Components",
    "steps": [
      "Update src/lib/tauri.ts, add api.steps namespace:",
      "  - getByTask: (taskId: string) => invoke('get_task_steps', { taskId })",
      "  - create: (taskId, data) => invoke('create_task_step', { taskId, ...data })",
      "  - update: (stepId, data) => invoke('update_task_step', { stepId, ...data })",
      "  - delete: (stepId) => invoke('delete_task_step', { stepId })",
      "  - reorder: (taskId, stepIds) => invoke('reorder_task_steps', { taskId, stepIds })",
      "  - getProgress: (taskId) => invoke('get_step_progress', { taskId })",
      "  - start: (stepId) => invoke('start_step', { stepId })",
      "  - complete: (stepId, note?) => invoke('complete_step', { stepId, note })",
      "  - skip: (stepId, reason) => invoke('skip_step', { stepId, reason })",
      "  - fail: (stepId, error) => invoke('fail_step', { stepId, error })",
      "Run npm run typecheck",
      "Commit: feat(api): add step API bindings"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Create useTaskSteps hook",
    "plan_section": "UI Components",
    "steps": [
      "Create src/hooks/useTaskSteps.ts:",
      "  - Export stepKeys query key factory: all, byTask(taskId), progress(taskId)",
      "  - useTaskSteps(taskId) hook using useQuery:",
      "    - queryKey: stepKeys.byTask(taskId)",
      "    - queryFn: api.steps.getByTask(taskId)",
      "    - staleTime: 30_000",
      "    - Return { data: TaskStep[], isLoading, isError }",
      "  - useStepProgress(taskId) hook using useQuery:",
      "    - queryKey: stepKeys.progress(taskId)",
      "    - queryFn: api.steps.getProgress(taskId)",
      "    - staleTime: 5_000",
      "    - refetchInterval: poll every 5s if inProgress > 0",
      "    - Return { data: StepProgressSummary, isLoading }",
      "Create useTaskSteps.test.ts with mock tests",
      "Run npm run lint && npm run typecheck && npm run test",
      "Commit: feat(hooks): add useTaskSteps and useStepProgress hooks"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Create useStepMutations hook",
    "plan_section": "UI Components",
    "steps": [
      "Create src/hooks/useStepMutations.ts:",
      "  - useStepMutations(taskId) hook returning mutations:",
      "    - create: useMutation for api.steps.create",
      "    - update: useMutation for api.steps.update",
      "    - delete: useMutation for api.steps.delete",
      "    - reorder: useMutation for api.steps.reorder",
      "  - Each mutation invalidates stepKeys.byTask(taskId) and stepKeys.progress(taskId)",
      "  - Show toast on success/error",
      "Create useStepMutations.test.ts",
      "Run npm run lint && npm run typecheck && npm run test",
      "Commit: feat(hooks): add useStepMutations hook"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Create useStepEvents hook for real-time updates",
    "plan_section": "Events",
    "steps": [
      "Create src/hooks/useStepEvents.ts:",
      "  - Listen for 'step:created', 'step:updated', 'step:deleted', 'steps:reordered' events",
      "  - On any event, extract task_id from payload",
      "  - Invalidate stepKeys.byTask(task_id) and stepKeys.progress(task_id)",
      "  - Use useEffect with cleanup to unlisten",
      "Add to EventProvider in src/providers/EventProvider.tsx",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(hooks): add useStepEvents for real-time step updates"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Create StepProgressBar component",
    "plan_section": "UI Components - StepProgressBar",
    "steps": [
      "Create src/components/tasks/StepProgressBar.tsx:",
      "  - Props: taskId: string, compact?: boolean",
      "  - Use useStepProgress(taskId) to fetch progress",
      "  - If loading or no data or total === 0, return null",
      "  - Render progress dots: map through total, color by status",
      "    - completed: bg-status-success",
      "    - skipped: bg-text-muted",
      "    - failed: bg-status-error",
      "    - in_progress: bg-accent-primary animate-pulse",
      "    - pending: bg-border-default",
      "  - If not compact, show text: '{completed + skipped}/{total}'",
      "Create StepProgressBar.test.tsx",
      "Run npm run lint && npm run typecheck && npm run test",
      "Commit: feat(components): add StepProgressBar component"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Create StepItem component",
    "plan_section": "UI Components - StepList",
    "steps": [
      "Create src/components/tasks/StepItem.tsx:",
      "  - Props: step: TaskStep, index: number, editable?: boolean, onUpdate?, onDelete?",
      "  - Status icon mapping: pending=Circle, in_progress=Loader2, completed=CheckCircle2,",
      "    skipped=MinusCircle, failed=XCircle",
      "  - Status color mapping with Tailwind classes",
      "  - Show step number, title, description (if exists), completion_note (if exists)",
      "  - If in_progress: add border-accent-primary and bg-accent-muted",
      "  - If completed: add opacity-75",
      "  - If skipped: add opacity-50 and line-through",
      "  - If editable and pending: show delete button",
      "Create StepItem.test.tsx",
      "Run npm run lint && npm run typecheck && npm run test",
      "Commit: feat(components): add StepItem component"
    ],
    "passes": true
  },
  {
    "category": "frontend",
    "description": "Create StepList component",
    "plan_section": "UI Components - StepList",
    "steps": [
      "Create src/components/tasks/StepList.tsx:",
      "  - Props: taskId: string, editable?: boolean",
      "  - Use useTaskSteps(taskId) to fetch steps",
      "  - Use useStepMutations(taskId) for mutations",
      "  - If loading: show Skeleton",
      "  - If no steps: show EmptyState with ListChecks icon",
      "  - Map steps to StepItem components",
      "  - Pass onUpdate and onDelete handlers if editable",
      "Create StepList.test.tsx",
      "Run npm run lint && npm run typecheck && npm run test",
      "Commit: feat(components): add StepList component"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Add execution state animations to CSS",
    "plan_section": "UI Components - Reactive TaskCard",
    "steps": [
      "Update src/styles/globals.css:",
      "  - Add @keyframes executing-pulse with box-shadow animation",
      "  - Add @keyframes attention-pulse with opacity animation",
      "  - Add .task-card-executing class with animation",
      "  - Add .task-card-attention class with animation",
      "  - Add CSS variables: --animation-executing-pulse, --animation-attention-pulse",
      "Run npm run lint",
      "Commit: feat(styles): add execution state animations"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Create useTaskExecutionState hook",
    "plan_section": "UI Components - Reactive TaskCard",
    "steps": [
      "Create src/hooks/useTaskExecutionState.ts:",
      "  - Props: taskId: string",
      "  - Combine data from useTaskQuery(taskId) and useStepProgress(taskId)",
      "  - Return TaskExecutionState: {",
      "      isActive: boolean (has recent activity),",
      "      duration: number | null (seconds since startedAt),",
      "      phase: 'idle' | 'executing' | 'qa' | 'review' | 'done',",
      "      stepProgress: StepProgressSummary | null",
      "    }",
      "  - Calculate duration using useMemo with interval timer when executing",
      "Create useTaskExecutionState.test.ts",
      "Run npm run lint && npm run typecheck && npm run test",
      "Commit: feat(hooks): add useTaskExecutionState hook"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Add execution visuals to TaskCard",
    "plan_section": "UI Components - Reactive TaskCard",
    "steps": [
      "Update src/components/tasks/TaskBoard/TaskCard.tsx:",
      "  - Import useTaskExecutionState(task.id)",
      "  - Add executing state styles:",
      "    - If status is 'executing': add task-card-executing class, pulsing orange border",
      "    - If status is 'qa_*': add pulsing border with QA icon",
      "    - If status is 'pending_review': add amber border with Eye icon",
      "    - If status is 'revision_needed': add task-card-attention class",
      "  - Add activity dots indicator in top-right corner when executing:",
      "    - Three dots with staggered bounce animation",
      "  - Conditionally render based on task.internalStatus",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(card): add execution state visuals to TaskCard"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Add StepProgressBar to TaskCard",
    "plan_section": "UI Components - Reactive TaskCard",
    "steps": [
      "Update src/components/tasks/TaskBoard/TaskCard.tsx:",
      "  - Import StepProgressBar",
      "  - At bottom of card, conditionally render StepProgressBar:",
      "    - Show when task.internalStatus is 'executing', 'qa_*', or 'pending_review'",
      "    - Use compact={true} variant",
      "  - Add duration badge next to progress bar when executing:",
      "    - Format: '2m 15s' using formatDuration helper",
      "    - Clock icon from Lucide",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(card): add StepProgressBar and duration to TaskCard"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Add taskFullViewId to uiStore",
    "plan_section": "UI Components - TaskFullView",
    "steps": [
      "Update src/stores/uiStore.ts:",
      "  - Add taskFullViewId: string | null (default null)",
      "  - Add openTaskFullView: (taskId: string) => void",
      "  - Add closeTaskFullView: () => void",
      "Run npm run typecheck",
      "Commit: feat(stores): add taskFullViewId to uiStore"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Create TaskDetailPanel component",
    "plan_section": "UI Components - TaskFullView",
    "steps": [
      "Create src/components/tasks/TaskDetailPanel.tsx:",
      "  - Extract task detail content from TaskDetailModal into reusable component",
      "  - Props: task: Task, showContext?: boolean, showHistory?: boolean",
      "  - Render: priority badge, title, category, status, description",
      "  - Render: TaskContextPanel (collapsible) if task has sourceProposalId or planArtifactId",
      "  - Render: StepList if task has steps (check via useTaskSteps)",
      "  - Render: StateHistoryTimeline (collapsible)",
      "  - No edit buttons - parent handles that",
      "Create TaskDetailPanel.test.tsx",
      "Run npm run lint && npm run typecheck && npm run test",
      "Commit: feat(components): add TaskDetailPanel component"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Create TaskChatPanel component",
    "plan_section": "UI Components - TaskFullView",
    "steps": [
      "Create src/components/tasks/TaskChatPanel.tsx:",
      "  - Props: taskId: string, contextType: 'task' | 'task_execution'",
      "  - Reuse ChatPanel internals but without resize/collapse functionality",
      "  - Determine context based on task status:",
      "    - If executing/qa_*: contextType = 'task_execution'",
      "    - Otherwise: contextType = 'task'",
      "  - Show header with context indicator (e.g., 'Worker Execution' when executing)",
      "  - Render message list with auto-scroll",
      "  - Render input at bottom",
      "Create TaskChatPanel.test.tsx",
      "Run npm run lint && npm run typecheck && npm run test",
      "Commit: feat(components): add TaskChatPanel for embedded chat"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Create TaskFullView component",
    "plan_section": "UI Components - TaskFullView",
    "steps": [
      "Create src/components/tasks/TaskFullView.tsx:",
      "  - Props: taskId: string, onClose: () => void",
      "  - Full-screen overlay with 24px margin (Raycast-style)",
      "  - Header: Back button, title, priority badge, status, edit/archive buttons, close",
      "  - Split layout: left panel (TaskDetailPanel) | right panel (TaskChatPanel)",
      "  - Default 50/50 split",
      "  - Add drag handle between panels for resizing",
      "  - Minimum panel width: 360px",
      "  - Footer: execution controls if task is executing (Pause/Stop)",
      "  - Escape key closes view",
      "Create TaskFullView.test.tsx",
      "Run npm run lint && npm run typecheck && npm run test",
      "Commit: feat(components): add TaskFullView with split layout"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Add resizable panels to TaskFullView",
    "plan_section": "UI Components - TaskFullView",
    "steps": [
      "Update src/components/tasks/TaskFullView.tsx:",
      "  - Add panelWidth state (default 50%)",
      "  - Add drag handle div between panels with cursor-col-resize",
      "  - Implement onMouseDown handler on drag handle:",
      "    - Track mouse movement",
      "    - Calculate new width percentage",
      "    - Clamp to min 360px on each side",
      "  - Apply width via inline style: left panel = panelWidth%, right panel = (100 - panelWidth)%",
      "  - Store preference in localStorage",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(fullview): add resizable panels"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Wire up TaskFullView in App",
    "plan_section": "UI Components - TaskFullView",
    "steps": [
      "Update src/App.tsx:",
      "  - Import TaskFullView and useUiStore",
      "  - Get taskFullViewId and closeTaskFullView from uiStore",
      "  - If taskFullViewId is not null, render TaskFullView with taskId={taskFullViewId}",
      "  - Pass onClose={closeTaskFullView}",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(app): wire up TaskFullView rendering"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Open TaskFullView from TaskCard click",
    "plan_section": "Integration",
    "steps": [
      "Update src/components/tasks/TaskBoard/TaskCard.tsx:",
      "  - Import openTaskFullView from uiStore",
      "  - Determine which view to open based on task status:",
      "    - If executing, qa_*, pending_review, revision_needed: openTaskFullView(task.id)",
      "    - Otherwise: openModal('task-detail', { taskId: task.id })",
      "  - Update click handler to use this logic",
      "  - Context menu 'View Details' should use same logic",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(card): open TaskFullView for executing tasks"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Add step editor to TaskCreationForm",
    "plan_section": "Integration",
    "steps": [
      "Update src/components/tasks/TaskCreationForm.tsx:",
      "  - Add steps: string[] state (default empty)",
      "  - Add 'Steps' section with:",
      "    - List of step inputs with delete buttons",
      "    - 'Add step' button at bottom",
      "    - Drag-to-reorder using dnd-kit (optional, can be simple up/down buttons)",
      "  - On form submit, include steps array in creation data",
      "  - Backend will create TaskSteps from the array",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(form): add step editor to TaskCreationForm"
    ],
    "passes": false
  },
  {
    "category": "frontend",
    "description": "Add step editor to TaskEditForm",
    "plan_section": "Integration",
    "steps": [
      "Update src/components/tasks/TaskEditForm.tsx:",
      "  - Fetch existing steps via useTaskSteps(task.id)",
      "  - Add StepList with editable={true}",
      "  - Allow adding new steps inline",
      "  - Changes are saved via useStepMutations",
      "  - Only allow editing steps if task is not executing",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(form): add step editor to TaskEditForm"
    ],
    "passes": false
  },
  {
    "category": "backend",
    "description": "Add steps to task creation command",
    "plan_section": "Integration",
    "steps": [
      "Update src-tauri/src/commands/task_commands.rs create_task command:",
      "  - Add optional steps: Option<Vec<String>> parameter",
      "  - If steps provided, create TaskStep for each after task creation",
      "  - Use created_by = 'user'",
      "Write unit test for task creation with steps",
      "Run cargo test",
      "Commit: feat(commands): support steps in create_task command"
    ],
    "passes": false
  },
  {
    "category": "documentation",
    "description": "Update CLAUDE.md files for Phase 19",
    "plan_section": "Documentation",
    "steps": [
      "Update src/CLAUDE.md with:",
      "  - New types: TaskStep, TaskStepStatus, StepProgressSummary",
      "  - New hooks: useTaskSteps, useStepProgress, useStepMutations, useStepEvents, useTaskExecutionState",
      "  - New components: StepProgressBar, StepList, StepItem, TaskDetailPanel, TaskChatPanel, TaskFullView",
      "  - New uiStore state: taskFullViewId",
      "Update src-tauri/CLAUDE.md with:",
      "  - TaskStep entity and TaskStepRepository",
      "  - Step commands: create_task_step, get_task_steps, update_task_step, delete_task_step,",
      "    reorder_task_steps, get_step_progress, start_step, complete_step, skip_step, fail_step",
      "  - HTTP endpoints for MCP: /api/task_steps/:task_id, /api/start_step, etc.",
      "  - Worker tool scoping update",
      "  - Events: step:created, step:updated, step:deleted, steps:reordered",
      "Update logs/activity.md with Phase 19 completion summary",
      "Commit: docs: update documentation for Phase 19"
    ],
    "passes": false
  }
]
```

---

## Key Architecture Decisions

| Decision | Rationale |
|----------|-----------|
| **Separate `task_steps` table** | Proper relational model, queryable, auditable, supports future features |
| **Steps have 6 states** | Richer progress info (skipped, failed) vs simple done/not-done |
| **Worker MUST call step tools** | Enforced by prompt, provides reliable deterministic progress |
| **Steps imported from proposal.steps** | Continuity from ideation to execution |
| **Full-screen TaskFullView** | Chat needs space, modal was too cramped for execution monitoring |
| **50/50 default split** | Both panels equally important; resizable for preference |
| **Context-aware chat** | Right panel shows relevant conversation based on task state |
| **StepProgressSummary calculated server-side** | Single source of truth, consistent across clients |
| **Real-time events for steps** | UI updates immediately when worker marks step complete |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] TaskStep entity creation and status helpers work
- [ ] SqliteTaskStepRepository CRUD operations work
- [ ] MemoryTaskStepRepository matches SQLite behavior
- [ ] Step transition commands validate states correctly
- [ ] StepProgressSummary calculation is correct
- [ ] Steps imported from proposal on task creation
- [ ] HTTP endpoints return correct responses

### Frontend - Run `npm run test`
- [ ] useTaskSteps.test.ts passes
- [ ] useStepMutations.test.ts passes
- [ ] StepProgressBar.test.tsx passes
- [ ] StepItem.test.tsx passes
- [ ] StepList.test.tsx passes
- [ ] TaskDetailPanel.test.tsx passes
- [ ] TaskChatPanel.test.tsx passes
- [ ] TaskFullView.test.tsx passes
- [ ] useTaskExecutionState.test.ts passes

### Build Verification - Run `npm run build` and `cargo build --release`
- [ ] No TypeScript errors
- [ ] No Rust compilation errors
- [ ] No lint warnings (`npm run lint` and `cargo clippy`)

### Type Verification - Run `npm run typecheck`
- [ ] TaskStep type works with API
- [ ] StepProgressSummary type works with API
- [ ] All new hooks have correct return types
