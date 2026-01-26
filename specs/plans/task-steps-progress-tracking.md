# RalphX Task Steps: Deterministic Progress Tracking

## Overview

This plan introduces a **Task Steps** system that provides deterministic progress tracking for task execution. Steps are discrete, trackable units of work that the agent updates as it progresses through a task.

**Problem Statement:**
Currently, we cannot show meaningful progress during task execution. The agent streams tool calls and text, but there's no structured way to know "how far along" a task is.

**Solution:**
Add a dedicated `task_steps` table with proper states, timestamps, and MCP tools that allow:
1. Tasks to have structured steps (created by user, agent, or from proposal)
2. Worker agent to update step status as it works
3. UI to show real progress: "Step 3/7: Running tests"

---

## Data Model

### TaskStep Entity

```rust
// src-tauri/src/domain/entities/task_step.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Unique identifier for a task step
pub struct TaskStepId(pub String);

/// Status of a task step
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStepStatus {
    /// Step is waiting to be worked on
    Pending,
    /// Step is currently being executed
    InProgress,
    /// Step completed successfully
    Completed,
    /// Step was skipped (not applicable, blocked, or deferred)
    Skipped,
    /// Step failed and needs attention
    Failed,
    /// Step was cancelled (task cancelled or step removed)
    Cancelled,
}

/// A single step within a task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStep {
    /// Unique identifier
    pub id: TaskStepId,
    /// The task this step belongs to
    pub task_id: TaskId,
    /// Short description of the step
    pub title: String,
    /// Optional longer description
    pub description: Option<String>,
    /// Current status
    pub status: TaskStepStatus,
    /// Order within the task (0-indexed)
    pub sort_order: i32,
    /// Optional: ID of step this depends on (must complete first)
    pub depends_on: Option<TaskStepId>,
    /// Who created this step: "user", "agent", "proposal", "system"
    pub created_by: String,
    /// Optional note from agent when completing/skipping/failing
    pub completion_note: Option<String>,
    /// When the step was created
    pub created_at: DateTime<Utc>,
    /// When the step was last updated
    pub updated_at: DateTime<Utc>,
    /// When the step started (status → InProgress)
    pub started_at: Option<DateTime<Utc>>,
    /// When the step completed (status → Completed/Skipped/Failed/Cancelled)
    pub completed_at: Option<DateTime<Utc>>,
}

impl TaskStep {
    pub fn new(task_id: TaskId, title: String, sort_order: i32, created_by: &str) -> Self {
        let now = Utc::now();
        Self {
            id: TaskStepId::new(),
            task_id,
            title,
            description: None,
            status: TaskStepStatus::Pending,
            sort_order,
            depends_on: None,
            created_by: created_by.to_string(),
            completion_note: None,
            created_at: now,
            updated_at: now,
            started_at: None,
            completed_at: None,
        }
    }

    /// Check if this step can be started
    pub fn can_start(&self) -> bool {
        self.status == TaskStepStatus::Pending
    }

    /// Check if this step is terminal (won't change further)
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            TaskStepStatus::Completed
                | TaskStepStatus::Skipped
                | TaskStepStatus::Failed
                | TaskStepStatus::Cancelled
        )
    }
}
```

### TaskStepStatus Transitions

```
         ┌──────────────┐
         │   Pending    │
         └──────────────┘
                │
                ▼
         ┌──────────────┐
    ┌────│  InProgress  │────┐
    │    └──────────────┘    │
    │           │            │
    ▼           ▼            ▼
┌────────┐ ┌────────┐ ┌────────┐
│Completed│ │ Failed │ │ Skipped│
└────────┘ └────────┘ └────────┘

Any state → Cancelled (task cancelled)
```

**Valid Transitions:**
| From | To | Trigger |
|------|----|----|
| Pending | InProgress | Agent starts step |
| Pending | Skipped | Agent determines step not needed |
| Pending | Cancelled | Task cancelled |
| InProgress | Completed | Agent finishes step successfully |
| InProgress | Failed | Agent encounters error |
| InProgress | Skipped | Agent determines step not needed mid-work |
| InProgress | Cancelled | Task cancelled |
| Failed | InProgress | Agent retries (rare, explicit) |

---

## Database Schema

### Migration: `XXX_add_task_steps.sql`

```sql
-- Task steps table for structured progress tracking
CREATE TABLE task_steps (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    sort_order INTEGER NOT NULL DEFAULT 0,
    depends_on TEXT REFERENCES task_steps(id) ON DELETE SET NULL,
    created_by TEXT NOT NULL DEFAULT 'user',
    completion_note TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    started_at TEXT,
    completed_at TEXT
);

-- Index for querying steps by task (most common query)
CREATE INDEX idx_task_steps_task_id ON task_steps(task_id);

-- Index for ordering within a task
CREATE INDEX idx_task_steps_task_order ON task_steps(task_id, sort_order);

-- Index for finding pending/in_progress steps
CREATE INDEX idx_task_steps_status ON task_steps(task_id, status);
```

---

## Repository Layer

### TaskStepRepository Trait

```rust
// src-tauri/src/domain/repositories/task_step_repository.rs

#[async_trait]
pub trait TaskStepRepository: Send + Sync {
    /// Create a new step
    async fn create(&self, step: TaskStep) -> AppResult<TaskStep>;

    /// Get step by ID
    async fn get_by_id(&self, id: &TaskStepId) -> AppResult<Option<TaskStep>>;

    /// Get all steps for a task (ordered by sort_order)
    async fn get_by_task(&self, task_id: &TaskId) -> AppResult<Vec<TaskStep>>;

    /// Get steps by status for a task
    async fn get_by_task_and_status(
        &self,
        task_id: &TaskId,
        status: TaskStepStatus,
    ) -> AppResult<Vec<TaskStep>>;

    /// Update a step
    async fn update(&self, step: &TaskStep) -> AppResult<()>;

    /// Delete a step
    async fn delete(&self, id: &TaskStepId) -> AppResult<()>;

    /// Delete all steps for a task
    async fn delete_by_task(&self, task_id: &TaskId) -> AppResult<()>;

    /// Count steps by status for a task
    async fn count_by_status(
        &self,
        task_id: &TaskId,
    ) -> AppResult<HashMap<TaskStepStatus, u32>>;

    /// Bulk create steps (for importing from proposal)
    async fn bulk_create(&self, steps: Vec<TaskStep>) -> AppResult<Vec<TaskStep>>;

    /// Reorder steps (update sort_order for multiple steps)
    async fn reorder(&self, task_id: &TaskId, step_ids: Vec<TaskStepId>) -> AppResult<()>;
}
```

---

## Tauri Commands

### Step CRUD Commands

```rust
// src-tauri/src/commands/task_step_commands.rs

/// Create a new step for a task
#[tauri::command]
pub async fn create_task_step(
    task_id: String,
    title: String,
    description: Option<String>,
    sort_order: Option<i32>,
    state: State<'_, AppState>,
) -> Result<TaskStep, AppError>;

/// Get all steps for a task
#[tauri::command]
pub async fn get_task_steps(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<TaskStep>, AppError>;

/// Update a step's details (title, description, sort_order)
#[tauri::command]
pub async fn update_task_step(
    step_id: String,
    title: Option<String>,
    description: Option<String>,
    sort_order: Option<i32>,
    state: State<'_, AppState>,
) -> Result<TaskStep, AppError>;

/// Delete a step
#[tauri::command]
pub async fn delete_task_step(
    step_id: String,
    state: State<'_, AppState>,
) -> Result<(), AppError>;

/// Reorder steps within a task
#[tauri::command]
pub async fn reorder_task_steps(
    task_id: String,
    step_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<TaskStep>, AppError>;

/// Import steps from proposal (when creating task from proposal)
#[tauri::command]
pub async fn import_steps_from_proposal(
    task_id: String,
    proposal_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<TaskStep>, AppError>;
```

### Step Status Commands (for MCP/Worker)

```rust
/// Start a step (Pending → InProgress)
#[tauri::command]
pub async fn start_step(
    step_id: String,
    state: State<'_, AppState>,
) -> Result<TaskStep, AppError>;

/// Complete a step (InProgress → Completed)
#[tauri::command]
pub async fn complete_step(
    step_id: String,
    note: Option<String>,
    state: State<'_, AppState>,
) -> Result<TaskStep, AppError>;

/// Skip a step (Pending/InProgress → Skipped)
#[tauri::command]
pub async fn skip_step(
    step_id: String,
    reason: String,
    state: State<'_, AppState>,
) -> Result<TaskStep, AppError>;

/// Fail a step (InProgress → Failed)
#[tauri::command]
pub async fn fail_step(
    step_id: String,
    error: String,
    state: State<'_, AppState>,
) -> Result<TaskStep, AppError>;

/// Get step progress summary for a task
#[tauri::command]
pub async fn get_step_progress(
    task_id: String,
    state: State<'_, AppState>,
) -> Result<StepProgressSummary, AppError>;
```

### StepProgressSummary

```rust
#[derive(Debug, Serialize)]
pub struct StepProgressSummary {
    pub task_id: String,
    pub total: u32,
    pub completed: u32,
    pub in_progress: u32,
    pub pending: u32,
    pub skipped: u32,
    pub failed: u32,
    pub current_step: Option<TaskStep>,  // The InProgress step, if any
    pub next_step: Option<TaskStep>,     // First Pending step
    pub percent_complete: f32,           // (completed + skipped) / total * 100
}
```

---

## HTTP/MCP Endpoints

### New Endpoints for Worker Agent

```rust
// Add to http_server.rs

// Worker step tools
.route("/api/get_task_steps/:task_id", get(get_task_steps_handler))
.route("/api/start_step", post(start_step_handler))
.route("/api/complete_step", post(complete_step_handler))
.route("/api/skip_step", post(skip_step_handler))
.route("/api/fail_step", post(fail_step_handler))
.route("/api/add_step", post(add_step_handler))
.route("/api/get_step_progress/:task_id", get(get_step_progress_handler))
```

### Request/Response Types

```rust
#[derive(Debug, Deserialize)]
pub struct StartStepRequest {
    pub step_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CompleteStepRequest {
    pub step_id: String,
    pub note: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SkipStepRequest {
    pub step_id: String,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct FailStepRequest {
    pub step_id: String,
    pub error: String,
}

#[derive(Debug, Deserialize)]
pub struct AddStepRequest {
    pub task_id: String,
    pub title: String,
    pub description: Option<String>,
    pub after_step_id: Option<String>,  // Insert after this step
}

#[derive(Debug, Serialize)]
pub struct StepResponse {
    pub id: String,
    pub task_id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub sort_order: i32,
    pub completion_note: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}
```

---

## MCP Tool Definitions

### Worker Agent Tools

| Tool | Description | Parameters |
|------|-------------|------------|
| `get_task_steps` | Get all steps for current task | `task_id` |
| `start_step` | Mark step as in-progress | `step_id` |
| `complete_step` | Mark step as completed | `step_id`, `note?` |
| `skip_step` | Mark step as skipped | `step_id`, `reason` |
| `fail_step` | Mark step as failed | `step_id`, `error` |
| `add_step` | Add a new step during execution | `task_id`, `title`, `description?`, `after_step_id?` |
| `get_step_progress` | Get progress summary | `task_id` |

### Tool Scoping Update

```
| Agent | MCP Tools |
|-------|-----------|
| worker | get_task_context, get_artifact*, search_project_artifacts,
|        | **get_task_steps, start_step, complete_step, skip_step, fail_step, add_step, get_step_progress** |
```

---

## Worker Agent Prompt Updates

### System Prompt Addition

```markdown
## Step Progress Tracking

When executing a task, you MUST track progress using steps:

1. **At start of execution**, call `get_task_steps(task_id)` to see the plan
2. **Before starting each step**, call `start_step(step_id)`
3. **After completing each step**, call `complete_step(step_id, note?)`
4. **If a step is not applicable**, call `skip_step(step_id, reason)`
5. **If a step fails**, call `fail_step(step_id, error)` then decide how to proceed

### If no steps exist:
- Create steps as you plan your work using `add_step`
- Break down the task into 3-8 discrete, verifiable steps
- Each step should be completable in a single focused effort

### Step Guidelines:
- Keep step titles short and action-oriented: "Add login API endpoint"
- Use completion notes to document what was done
- Skip steps with clear reasons if requirements changed
- Fail steps with specific error messages for debugging

### Progress Flow:
```
get_task_steps → Plan work
start_step → Begin work
[do the work]
complete_step → Move to next
```
```

---

## Frontend Types

### Zod Schemas

```typescript
// src/types/task-step.ts

import { z } from "zod";

export const TaskStepStatusSchema = z.enum([
  "pending",
  "in_progress",
  "completed",
  "skipped",
  "failed",
  "cancelled",
]);

export type TaskStepStatus = z.infer<typeof TaskStepStatusSchema>;

export const TaskStepSchema = z.object({
  id: z.string().uuid(),
  taskId: z.string().uuid(),
  title: z.string().min(1),
  description: z.string().nullable(),
  status: TaskStepStatusSchema,
  sortOrder: z.number().int(),
  dependsOn: z.string().uuid().nullable(),
  createdBy: z.string(),
  completionNote: z.string().nullable(),
  createdAt: z.string().datetime({ offset: true }),
  updatedAt: z.string().datetime({ offset: true }),
  startedAt: z.string().datetime({ offset: true }).nullable(),
  completedAt: z.string().datetime({ offset: true }).nullable(),
});

export type TaskStep = z.infer<typeof TaskStepSchema>;

export const StepProgressSummarySchema = z.object({
  taskId: z.string().uuid(),
  total: z.number().int().nonnegative(),
  completed: z.number().int().nonnegative(),
  inProgress: z.number().int().nonnegative(),
  pending: z.number().int().nonnegative(),
  skipped: z.number().int().nonnegative(),
  failed: z.number().int().nonnegative(),
  currentStep: TaskStepSchema.nullable(),
  nextStep: TaskStepSchema.nullable(),
  percentComplete: z.number().min(0).max(100),
});

export type StepProgressSummary = z.infer<typeof StepProgressSummarySchema>;
```

---

## Frontend Hooks

### useTaskSteps Hook

```typescript
// src/hooks/useTaskSteps.ts

export const stepKeys = {
  all: ["steps"] as const,
  byTask: (taskId: string) => [...stepKeys.all, "task", taskId] as const,
  progress: (taskId: string) => [...stepKeys.all, "progress", taskId] as const,
};

export function useTaskSteps(taskId: string) {
  return useQuery({
    queryKey: stepKeys.byTask(taskId),
    queryFn: () => api.steps.getByTask(taskId),
    staleTime: 30_000, // Steps change during execution
  });
}

export function useStepProgress(taskId: string) {
  return useQuery({
    queryKey: stepKeys.progress(taskId),
    queryFn: () => api.steps.getProgress(taskId),
    staleTime: 5_000, // Refresh frequently during execution
    refetchInterval: (query) => {
      // Poll every 5s if task is executing
      const data = query.state.data;
      if (data && data.inProgress > 0) return 5_000;
      return false;
    },
  });
}
```

### useStepMutations Hook

```typescript
// src/hooks/useStepMutations.ts

export function useStepMutations(taskId: string) {
  const queryClient = useQueryClient();

  const invalidate = () => {
    queryClient.invalidateQueries({ queryKey: stepKeys.byTask(taskId) });
    queryClient.invalidateQueries({ queryKey: stepKeys.progress(taskId) });
  };

  const createMutation = useMutation({
    mutationFn: (data: CreateStepInput) => api.steps.create(taskId, data),
    onSuccess: invalidate,
  });

  const updateMutation = useMutation({
    mutationFn: ({ stepId, ...data }: UpdateStepInput) =>
      api.steps.update(stepId, data),
    onSuccess: invalidate,
  });

  const deleteMutation = useMutation({
    mutationFn: (stepId: string) => api.steps.delete(stepId),
    onSuccess: invalidate,
  });

  const reorderMutation = useMutation({
    mutationFn: (stepIds: string[]) => api.steps.reorder(taskId, stepIds),
    onSuccess: invalidate,
  });

  return {
    create: createMutation,
    update: updateMutation,
    delete: deleteMutation,
    reorder: reorderMutation,
  };
}
```

---

## Frontend Components

### StepProgressBar

```tsx
// src/components/tasks/StepProgressBar.tsx

interface StepProgressBarProps {
  taskId: string;
  compact?: boolean;
}

export function StepProgressBar({ taskId, compact = false }: StepProgressBarProps) {
  const { data: progress, isLoading } = useStepProgress(taskId);

  if (isLoading || !progress || progress.total === 0) return null;

  const { completed, skipped, failed, inProgress, total, percentComplete } = progress;

  return (
    <div className={cn("flex items-center gap-2", compact && "gap-1")}>
      {/* Progress dots */}
      <div className="flex gap-0.5">
        {Array.from({ length: total }).map((_, i) => {
          let status: TaskStepStatus = "pending";
          if (i < completed) status = "completed";
          else if (i < completed + skipped) status = "skipped";
          else if (i < completed + skipped + failed) status = "failed";
          else if (i < completed + skipped + failed + inProgress) status = "in_progress";

          return (
            <span
              key={i}
              className={cn(
                "w-1.5 h-1.5 rounded-full",
                status === "completed" && "bg-status-success",
                status === "skipped" && "bg-text-muted",
                status === "failed" && "bg-status-error",
                status === "in_progress" && "bg-accent-primary animate-pulse",
                status === "pending" && "bg-border-default"
              )}
            />
          );
        })}
      </div>

      {/* Progress text */}
      {!compact && (
        <span className="text-xs text-text-muted">
          {completed + skipped}/{total}
        </span>
      )}
    </div>
  );
}
```

### StepList (for TaskDetailPanel)

```tsx
// src/components/tasks/StepList.tsx

interface StepListProps {
  taskId: string;
  editable?: boolean;
}

export function StepList({ taskId, editable = false }: StepListProps) {
  const { data: steps, isLoading } = useTaskSteps(taskId);
  const mutations = useStepMutations(taskId);

  if (isLoading) return <Skeleton />;
  if (!steps || steps.length === 0) {
    return (
      <EmptyState
        icon={ListChecks}
        title="No steps defined"
        description="Steps will appear here as the agent works"
      />
    );
  }

  return (
    <div className="space-y-2">
      {steps.map((step, index) => (
        <StepItem
          key={step.id}
          step={step}
          index={index}
          editable={editable}
          onUpdate={(changes) => mutations.update.mutate({ stepId: step.id, ...changes })}
          onDelete={() => mutations.delete.mutate(step.id)}
        />
      ))}
    </div>
  );
}
```

### StepItem

```tsx
// src/components/tasks/StepItem.tsx

interface StepItemProps {
  step: TaskStep;
  index: number;
  editable?: boolean;
  onUpdate?: (changes: Partial<TaskStep>) => void;
  onDelete?: () => void;
}

export function StepItem({ step, index, editable, onUpdate, onDelete }: StepItemProps) {
  const statusIcon = {
    pending: Circle,
    in_progress: Loader2,
    completed: CheckCircle2,
    skipped: MinusCircle,
    failed: XCircle,
    cancelled: XCircle,
  }[step.status];

  const statusColor = {
    pending: "text-text-muted",
    in_progress: "text-accent-primary animate-spin",
    completed: "text-status-success",
    skipped: "text-text-muted",
    failed: "text-status-error",
    cancelled: "text-text-muted",
  }[step.status];

  return (
    <div
      className={cn(
        "flex items-start gap-3 p-3 rounded-lg border",
        step.status === "in_progress" && "border-accent-primary bg-accent-muted",
        step.status === "completed" && "opacity-75",
        step.status === "skipped" && "opacity-50 line-through"
      )}
    >
      <Icon className={cn("w-5 h-5 mt-0.5", statusColor)} />

      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="text-xs text-text-muted font-mono">#{index + 1}</span>
          <span className="font-medium truncate">{step.title}</span>
        </div>

        {step.description && (
          <p className="text-sm text-text-secondary mt-1">{step.description}</p>
        )}

        {step.completionNote && (
          <p className="text-xs text-text-muted mt-1 italic">
            Note: {step.completionNote}
          </p>
        )}

        {step.startedAt && (
          <span className="text-xs text-text-muted">
            Started {formatRelativeTime(step.startedAt)}
          </span>
        )}
      </div>

      {editable && step.status === "pending" && (
        <Button variant="ghost" size="icon" onClick={onDelete}>
          <Trash2 className="w-4 h-4" />
        </Button>
      )}
    </div>
  );
}
```

---

## Events

### Step Events

```rust
// Emitted via Tauri events

// When step status changes
"step:updated" {
  step_id: String,
  task_id: String,
  status: String,
  previous_status: String,
}

// When steps are reordered
"steps:reordered" {
  task_id: String,
  step_ids: Vec<String>,
}

// When a step is added
"step:created" {
  step: TaskStep,
}

// When a step is deleted
"step:deleted" {
  step_id: String,
  task_id: String,
}
```

### Frontend Event Handling

```typescript
// src/hooks/useStepEvents.ts

export function useStepEvents() {
  const queryClient = useQueryClient();

  useEffect(() => {
    const unlistenUpdated = listen("step:updated", (event) => {
      const { task_id } = event.payload;
      queryClient.invalidateQueries({ queryKey: stepKeys.byTask(task_id) });
      queryClient.invalidateQueries({ queryKey: stepKeys.progress(task_id) });
    });

    // ... other events

    return () => {
      unlistenUpdated.then(fn => fn());
    };
  }, [queryClient]);
}
```

---

## Integration Points

### 1. Task Creation from Proposal

When creating a task from a proposal, automatically import steps from `proposal.steps`:

```rust
// In create_tasks_from_proposals or similar

if let Some(steps_json) = &proposal.steps {
    let steps: Vec<String> = serde_json::from_str(steps_json)?;
    let task_steps: Vec<TaskStep> = steps
        .into_iter()
        .enumerate()
        .map(|(i, title)| TaskStep::new(task.id.clone(), title, i as i32, "proposal"))
        .collect();
    task_step_repo.bulk_create(task_steps).await?;
}
```

### 2. Task Creation UI

Add step editor to TaskCreationForm:

- Drag-to-reorder
- Add/remove steps
- Optional descriptions

### 3. TaskDetailModal/TaskFullView

Show StepList in left panel when task has steps.

### 4. TaskCard

Show StepProgressBar at bottom of card when task is executing.

### 5. Worker Agent Execution

ExecutionChatService should:
1. Check if task has steps
2. Include steps in initial context
3. Instruct worker to use step tools

---

## Implementation Tasks

### Phase 1: Backend Data Layer

```json
[
  {
    "id": "S1",
    "task": "Create TaskStep entity and TaskStepStatus enum",
    "files": ["src-tauri/src/domain/entities/task_step.rs", "src-tauri/src/domain/entities/mod.rs"]
  },
  {
    "id": "S2",
    "task": "Create database migration for task_steps table",
    "files": ["src-tauri/src/infrastructure/sqlite/migrations.rs"]
  },
  {
    "id": "S3",
    "task": "Create TaskStepRepository trait",
    "files": ["src-tauri/src/domain/repositories/task_step_repository.rs"]
  },
  {
    "id": "S4",
    "task": "Implement SqliteTaskStepRepository",
    "files": ["src-tauri/src/infrastructure/sqlite/sqlite_task_step_repo.rs"]
  },
  {
    "id": "S5",
    "task": "Implement MemoryTaskStepRepository (for tests)",
    "files": ["src-tauri/src/infrastructure/memory/memory_task_step_repo.rs"]
  },
  {
    "id": "S6",
    "task": "Add TaskStepRepository to AppState",
    "files": ["src-tauri/src/application/app_state.rs"]
  }
]
```

### Phase 2: Tauri Commands

```json
[
  {
    "id": "S7",
    "task": "Create task_step_commands.rs with CRUD operations",
    "files": ["src-tauri/src/commands/task_step_commands.rs", "src-tauri/src/commands/mod.rs"]
  },
  {
    "id": "S8",
    "task": "Add step status commands (start, complete, skip, fail)",
    "files": ["src-tauri/src/commands/task_step_commands.rs"]
  },
  {
    "id": "S9",
    "task": "Register commands in lib.rs",
    "files": ["src-tauri/src/lib.rs"]
  },
  {
    "id": "S10",
    "task": "Write unit tests for step commands",
    "files": ["src-tauri/tests/task_step_commands.rs"]
  }
]
```

### Phase 3: HTTP/MCP Endpoints

```json
[
  {
    "id": "S11",
    "task": "Add step endpoints to http_server.rs",
    "files": ["src-tauri/src/http_server.rs"]
  },
  {
    "id": "S12",
    "task": "Update worker agent tool scoping",
    "files": ["src-tauri/src/http_server.rs", "ralphx-plugin/agents/worker.md"]
  },
  {
    "id": "S13",
    "task": "Add step tool definitions to MCP server",
    "files": ["MCP configuration"]
  }
]
```

### Phase 4: Worker Agent Updates

```json
[
  {
    "id": "S14",
    "task": "Update worker agent system prompt with step instructions",
    "files": ["ralphx-plugin/agents/worker.md"]
  },
  {
    "id": "S15",
    "task": "Update ExecutionChatService to include steps in context",
    "files": ["src-tauri/src/application/execution_chat_service.rs"]
  }
]
```

### Phase 5: Frontend

```json
[
  {
    "id": "S16",
    "task": "Add TaskStep types and schemas",
    "files": ["src/types/task-step.ts", "src/types/index.ts"]
  },
  {
    "id": "S17",
    "task": "Add step API bindings",
    "files": ["src/lib/tauri.ts"]
  },
  {
    "id": "S18",
    "task": "Create useTaskSteps and useStepProgress hooks",
    "files": ["src/hooks/useTaskSteps.ts"]
  },
  {
    "id": "S19",
    "task": "Create useStepMutations hook",
    "files": ["src/hooks/useStepMutations.ts"]
  },
  {
    "id": "S20",
    "task": "Create StepProgressBar component",
    "files": ["src/components/tasks/StepProgressBar.tsx"]
  },
  {
    "id": "S21",
    "task": "Create StepList and StepItem components",
    "files": ["src/components/tasks/StepList.tsx", "src/components/tasks/StepItem.tsx"]
  },
  {
    "id": "S22",
    "task": "Create useStepEvents hook",
    "files": ["src/hooks/useStepEvents.ts"]
  },
  {
    "id": "S23",
    "task": "Integrate StepProgressBar into TaskCard",
    "files": ["src/components/tasks/TaskBoard/TaskCard.tsx"]
  },
  {
    "id": "S24",
    "task": "Integrate StepList into TaskDetailPanel",
    "files": ["src/components/tasks/TaskDetailPanel.tsx"]
  }
]
```

### Phase 6: Integration

```json
[
  {
    "id": "S25",
    "task": "Import steps from proposal when creating task",
    "files": ["src-tauri/src/application/apply_service.rs"]
  },
  {
    "id": "S26",
    "task": "Add step editor to TaskCreationForm",
    "files": ["src/components/tasks/TaskCreationForm.tsx"]
  },
  {
    "id": "S27",
    "task": "Add step editor to TaskEditForm",
    "files": ["src/components/tasks/TaskEditForm.tsx"]
  }
]
```

---

## Success Criteria

1. **Deterministic Progress**: Can show "Step 3/7" on TaskCard
2. **Real-time Updates**: Steps update as worker progresses
3. **Agent Compliance**: Worker agent uses step tools consistently
4. **User Control**: Users can add/edit/reorder steps before execution
5. **Proposal Integration**: Steps flow from proposal → task automatically
6. **Visual Clarity**: Progress bar clearly shows completed/in-progress/pending

---

## Open Questions

1. Should steps be immutable once task is executing? (Probably yes for completed steps)
2. Should we support sub-steps? (Probably not for MVP)
3. Should steps have estimated duration? (Nice-to-have, hard for agent to estimate)
4. Should we track which tool calls belong to which step? (Complex, defer)
