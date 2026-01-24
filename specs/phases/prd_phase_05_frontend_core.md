# RalphX - Phase 5: Frontend Core

## Overview

This phase implements the **Frontend Core** infrastructure: Zustand state management, TanStack Query data fetching, Tauri event listeners, and typed API wrappers. It establishes the reactive data layer that connects the React UI to the Rust backend via Tauri's IPC bridge.

## Dependencies

- Phase 1 (Foundation) must be complete:
  - Strict TypeScript configuration
  - Zod schemas for Task, Project, InternalStatus
  - Basic Tauri invoke wrapper (typedInvoke)
  - Design system CSS variables

- Phase 2 (Data Layer) must be complete:
  - TaskRepository and ProjectRepository traits
  - Tauri commands for task and project CRUD

- Phase 4 (Agentic Client) must be complete:
  - AgentConfig, AgentHandle types (for frontend type definitions)

## Scope

### Included
- Zustand stores: taskStore, projectStore, uiStore, activityStore
- TanStack Query setup with QueryClient and QueryClientProvider
- Custom hooks: useTasks, useProjects, useTaskMutation, useEvents
- Tauri event listening with proper cleanup
- Event batching for high-frequency agent messages
- Event type definitions (AgentMessageEvent, TaskStatusEvent, etc.)
- EventProvider component for global event listeners
- Extended Tauri API wrappers (api.tasks.*, api.projects.*)
- Frontend-specific Zod schemas (TaskEventSchema, WorkflowSchema)

### Excluded
- UI components (Phase 6: Kanban UI)
- Agent profile types (Phase 7)
- QA-specific types (Phase 8)
- Ideation/chat types (Phase 10)
- Workflow CRUD operations (Phase 11)

## Detailed Requirements

### 1. Module Organization

From the master plan (lines 5633-5680):

```
src/
├── main.tsx                    # Entry point only
├── App.tsx                     # Router setup + EventProvider
├── types/                      # Shared type definitions
│   ├── index.ts                # Re-exports
│   ├── task.ts                 # Task types + Zod schemas (Phase 1)
│   ├── project.ts              # Project types (Phase 1)
│   ├── status.ts               # InternalStatus (Phase 1)
│   ├── events.ts               # Event types (NEW)
│   └── workflow.ts             # WorkflowSchema types (NEW)
├── lib/                        # Utilities, no React
│   ├── tauri.ts                # Extended Tauri invoke wrappers (expand)
│   ├── validation.ts           # Zod schemas
│   └── formatters.ts           # Date, number formatters (NEW)
├── hooks/                      # Custom React hooks (NEW)
│   ├── useTasks.ts
│   ├── useProjects.ts
│   ├── useTaskMutation.ts
│   └── useEvents.ts
├── stores/                     # Zustand stores (NEW)
│   ├── taskStore.ts
│   ├── projectStore.ts
│   ├── uiStore.ts
│   └── activityStore.ts
├── providers/                  # Context providers (NEW)
│   └── EventProvider.tsx
├── components/
│   └── ui/                     # Primitive components (placeholder)
└── pages/                      # Route components (placeholder)
```

### 2. Event Types

From the master plan (lines 1845-1898):

```typescript
// src/types/events.ts

// Agent activity events (high frequency)
export interface AgentMessageEvent {
  taskId: string;
  type: 'thinking' | 'tool_call' | 'tool_result' | 'text' | 'error';
  content: string;
  timestamp: number;
  metadata?: Record<string, unknown>;
}

// Task status changes
export interface TaskStatusEvent {
  taskId: string;
  fromStatus: string | null;
  toStatus: string;
  changedBy: 'user' | 'system' | 'ai_worker' | 'ai_reviewer' | 'ai_supervisor';
  reason?: string;
}

// Supervisor alerts
export interface SupervisorAlertEvent {
  taskId: string;
  severity: 'low' | 'medium' | 'high' | 'critical';
  type: 'loop_detected' | 'stuck' | 'error' | 'escalation';
  message: string;
  suggestedAction?: string;
}

// Review events
export interface ReviewEvent {
  taskId: string;
  reviewId: string;
  type: 'started' | 'completed' | 'needs_human' | 'fix_proposed';
  outcome?: 'approved' | 'changes_requested' | 'escalated';
}

// File change events (for diff viewer)
export interface FileChangeEvent {
  projectId: string;
  filePath: string;
  changeType: 'created' | 'modified' | 'deleted';
}

// Progress events
export interface ProgressEvent {
  taskId: string;
  progress: number;  // 0-100
  stage: string;     // "Running tests", "Committing changes"
}
```

### 3. TaskEvent Schema (Discriminated Union)

From the master plan (lines 5719-5743):

```typescript
// src/types/events.ts (continued)
import { z } from "zod";
import { InternalStatusSchema, TaskSchema } from "./task";

export const TaskEventSchema = z.discriminatedUnion("type", [
  z.object({
    type: z.literal("created"),
    task: TaskSchema,
  }),
  z.object({
    type: z.literal("updated"),
    taskId: z.string().uuid(),
    changes: TaskSchema.partial(),
  }),
  z.object({
    type: z.literal("deleted"),
    taskId: z.string().uuid(),
  }),
  z.object({
    type: z.literal("status_changed"),
    taskId: z.string().uuid(),
    from: InternalStatusSchema,
    to: InternalStatusSchema,
    changedBy: z.enum(["user", "system", "agent"]),
  }),
]);

export type TaskEvent = z.infer<typeof TaskEventSchema>;
```

### 4. Workflow Schema

From the master plan (lines 7754-7778):

```typescript
// src/types/workflow.ts
import { z } from "zod";
import { InternalStatusSchema } from "./status";

export const WorkflowColumnSchema = z.object({
  id: z.string(),
  name: z.string(),
  color: z.string().optional(),
  icon: z.string().optional(),
  mapsTo: InternalStatusSchema,
  behavior: z.object({
    skipReview: z.boolean().optional(),
    autoAdvance: z.boolean().optional(),
    agentProfile: z.string().optional(),
  }).optional(),
});

export type WorkflowColumn = z.infer<typeof WorkflowColumnSchema>;

export const WorkflowSchemaZ = z.object({
  id: z.string(),
  name: z.string(),
  description: z.string().optional(),
  columns: z.array(WorkflowColumnSchema),
  defaults: z.object({
    workerProfile: z.string().optional(),
    reviewerProfile: z.string().optional(),
  }).optional(),
});

export type WorkflowSchema = z.infer<typeof WorkflowSchemaZ>;
```

### 5. Zustand Store Pattern

From the master plan (lines 5873-5923):

```typescript
// src/stores/taskStore.ts
import { create } from "zustand";
import { immer } from "zustand/middleware/immer";
import { Task, InternalStatus } from "@/types/task";

interface TaskState {
  tasks: Record<string, Task>;
  selectedTaskId: string | null;
}

interface TaskActions {
  setTasks: (tasks: Task[]) => void;
  updateTask: (taskId: string, changes: Partial<Task>) => void;
  selectTask: (taskId: string | null) => void;
}

export const useTaskStore = create<TaskState & TaskActions>()(
  immer((set) => ({
    tasks: {},
    selectedTaskId: null,

    setTasks: (tasks) =>
      set((state) => {
        state.tasks = Object.fromEntries(tasks.map((t) => [t.id, t]));
      }),

    updateTask: (taskId, changes) =>
      set((state) => {
        const task = state.tasks[taskId];
        if (task) {
          Object.assign(task, changes);
        }
      }),

    selectTask: (taskId) =>
      set((state) => {
        state.selectedTaskId = taskId;
      }),
  }))
);

// Selectors (outside store for memoization)
export const selectTasksByStatus = (status: InternalStatus) => (state: TaskState) =>
  Object.values(state.tasks).filter((t) => t.internalStatus === status);

export const selectSelectedTask = (state: TaskState & TaskActions) =>
  state.selectedTaskId ? state.tasks[state.selectedTaskId] : null;
```

### 6. TanStack Query Hooks

From the master plan (lines 5824-5870, 2867-2943):

```typescript
// src/hooks/useTasks.ts
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { api } from "@/lib/tauri";

export function useTasks(projectId: string) {
  return useQuery({
    queryKey: ["tasks", projectId],
    queryFn: () => api.tasks.list(projectId),
  });
}

export function useTaskMutation(projectId: string) {
  const queryClient = useQueryClient();

  const moveMutation = useMutation({
    mutationFn: ({ taskId, toStatus }: { taskId: string; toStatus: string }) =>
      api.tasks.move(taskId, toStatus),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["tasks", projectId] });
    },
  });

  return { moveMutation };
}
```

### 7. Event Listening with Type Safety

From the master plan (lines 5925-5965, 1937-1991):

```typescript
// src/hooks/useEvents.ts
import { useEffect } from "react";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { TaskEventSchema, TaskEvent } from "@/types/events";
import { useTaskStore } from "@/stores/taskStore";
import { useActivityStore } from "@/stores/activityStore";

export function useTaskEvents() {
  const updateTask = useTaskStore((s) => s.updateTask);

  useEffect(() => {
    let unlisten: Promise<UnlistenFn>;

    unlisten = listen<unknown>("task:event", (event) => {
      // Runtime validation of backend events
      const parsed = TaskEventSchema.safeParse(event.payload);

      if (!parsed.success) {
        console.error("Invalid task event:", parsed.error);
        return;
      }

      const taskEvent = parsed.data;

      switch (taskEvent.type) {
        case "updated":
          updateTask(taskEvent.taskId, taskEvent.changes);
          break;
        case "status_changed":
          updateTask(taskEvent.taskId, { internalStatus: taskEvent.to });
          break;
        // ... handle other events
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [updateTask]);
}

export function useAgentEvents(taskId?: string) {
  const addMessage = useActivityStore((s) => s.addMessage);

  useEffect(() => {
    let unlisten: Promise<UnlistenFn>;

    unlisten = listen<AgentMessageEvent>("agent:message", (event) => {
      if (!taskId || event.payload.taskId === taskId) {
        addMessage(event.payload);
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [taskId, addMessage]);
}
```

### 8. Event Batching for Performance

From the master plan (lines 1993-2033):

```typescript
// src/hooks/useBatchedEvents.ts
import { useEffect, useRef, useState } from "react";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { AgentMessageEvent } from "@/types/events";

export function useBatchedAgentMessages(taskId: string) {
  const bufferRef = useRef<AgentMessageEvent[]>([]);
  const [messages, setMessages] = useState<AgentMessageEvent[]>([]);

  // Flush buffer every 50ms
  useEffect(() => {
    const interval = setInterval(() => {
      if (bufferRef.current.length > 0) {
        setMessages((prev) => [...prev, ...bufferRef.current]);
        bufferRef.current = [];
      }
    }, 50);

    return () => clearInterval(interval);
  }, []);

  // Buffer incoming events
  useEffect(() => {
    let unlisten: Promise<UnlistenFn>;

    unlisten = listen<AgentMessageEvent>("agent:message", (event) => {
      if (event.payload.taskId === taskId) {
        bufferRef.current.push(event.payload);
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [taskId]);

  return messages;
}
```

### 9. Global Event Provider

From the master plan (lines 2035-2063):

```typescript
// src/providers/EventProvider.tsx
import { useTaskEvents, useSupervisorAlerts, useReviewEvents, useFileChangeEvents } from "@/hooks/useEvents";

export function EventProvider({ children }: { children: React.ReactNode }) {
  // Set up global event listeners
  useTaskEvents();
  useSupervisorAlerts();
  useReviewEvents();
  useFileChangeEvents();

  return <>{children}</>;
}

// src/App.tsx usage:
function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <EventProvider>
        <Router>
          {/* ... */}
        </Router>
      </EventProvider>
    </QueryClientProvider>
  );
}
```

### 10. Extended Tauri API Wrappers

From the master plan (lines 5764-5780):

```typescript
// src/lib/tauri.ts
import { invoke } from "@tauri-apps/api/core";
import { TaskSchema, Task } from "@/types/task";
import { ProjectSchema, Project } from "@/types/project";
import { z } from "zod";

// Generic invoke wrapper with runtime validation
async function typedInvoke<T>(
  cmd: string,
  args: Record<string, unknown>,
  schema: z.ZodType<T>
): Promise<T> {
  const result = await invoke(cmd, args);
  return schema.parse(result);
}

// Typed API functions
export const api = {
  health: {
    check: () => typedInvoke("health_check", {}, z.object({ status: z.string() })),
  },

  tasks: {
    list: (projectId: string) =>
      typedInvoke("list_tasks", { projectId }, z.array(TaskSchema)),

    get: (taskId: string) =>
      typedInvoke("get_task", { taskId }, TaskSchema),

    create: (projectId: string, title: string, description?: string) =>
      typedInvoke("create_task", { projectId, title, description }, TaskSchema),

    update: (taskId: string, changes: Partial<Task>) =>
      typedInvoke("update_task", { taskId, changes }, TaskSchema),

    move: (taskId: string, toStatus: string) =>
      typedInvoke("move_task", { taskId, toStatus }, TaskSchema),

    delete: (taskId: string) =>
      typedInvoke("delete_task", { taskId }, z.boolean()),
  },

  projects: {
    list: () =>
      typedInvoke("list_projects", {}, z.array(ProjectSchema)),

    get: (projectId: string) =>
      typedInvoke("get_project", { projectId }, ProjectSchema),

    create: (name: string, workingDirectory: string) =>
      typedInvoke("create_project", { name, workingDirectory }, ProjectSchema),

    update: (projectId: string, changes: Partial<Project>) =>
      typedInvoke("update_project", { projectId, changes }, ProjectSchema),

    delete: (projectId: string) =>
      typedInvoke("delete_project", { projectId }, z.boolean()),
  },
};
```

### 11. Event Summary Table

From the master plan (lines 2065-2075):

| Event | Frequency | Source | UI Updates |
|-------|-----------|--------|------------|
| `agent:message` | High (streaming) | VM/Agent | Activity stream |
| `task:status` | Medium | System/AI | Kanban board, task cards |
| `supervisor:alert` | Low | Supervisor | Toast + alerts panel |
| `review:update` | Low | AI Reviewer | Reviews panel, badges |
| `file:change` | Medium | File watcher | Diff viewer |
| `progress:update` | Medium | Agent | Progress bars |

### 12. Testing Patterns

From the master plan (lines 2867-2943):

Tests must mock both Tauri invoke and TanStack Query. Example:

```typescript
// src/hooks/useTasks.test.ts
import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useTasks } from "./useTasks";
import { api } from "@/lib/tauri";

vi.mock("@/lib/tauri", () => ({
  api: {
    tasks: { list: vi.fn() },
  },
}));

describe("useTasks", () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    vi.clearAllMocks();
  });

  const wrapper = ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );

  it("fetches tasks for project", async () => {
    const mockTasks = [{ id: "1", title: "Task 1" }];
    vi.mocked(api.tasks.list).mockResolvedValue(mockTasks);

    const { result } = renderHook(() => useTasks("project-123"), { wrapper });

    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(api.tasks.list).toHaveBeenCalledWith("project-123");
    expect(result.current.data).toEqual(mockTasks);
  });
});
```

## Implementation Notes

### TDD is Mandatory
Every task follows the TDD cycle:
1. RED: Write failing tests first
2. GREEN: Write minimal implementation to pass
3. REFACTOR: Clean up while keeping tests green

### Store Design Principles
1. Keep stores focused - one domain per store
2. Use immer middleware for immutable updates
3. Define selectors outside the store for memoization
4. Avoid storing derived data - compute in selectors

### Event Handling Best Practices
1. Always clean up listeners in useEffect return
2. Use safeParse for runtime validation
3. Batch high-frequency events to prevent re-render thrashing
4. Log validation errors for debugging

### File Size Limits
- Hook: max 100 lines
- Store: max 150 lines
- Type definitions: max 200 lines

## Task List

```json
[
  {
    "category": "setup",
    "description": "Install TanStack Query and Zustand dependencies",
    "steps": [
      "Install TanStack Query: `npm install @tanstack/react-query`",
      "Install Zustand with immer: `npm install zustand immer`",
      "Install dev query tools: `npm install -D @tanstack/react-query-devtools`",
      "Verify package.json has correct versions",
      "Run `npm install` to ensure lock file is updated"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create event type definitions",
    "steps": [
      "Create src/types/events.ts",
      "Define AgentMessageEvent interface",
      "Define TaskStatusEvent interface",
      "Define SupervisorAlertEvent interface",
      "Define ReviewEvent interface",
      "Define FileChangeEvent interface",
      "Define ProgressEvent interface",
      "Export all types from src/types/index.ts"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create TaskEvent Zod schema (discriminated union)",
    "steps": [
      "Write tests for TaskEventSchema validation in src/types/events.test.ts",
      "Test each variant: created, updated, deleted, status_changed",
      "Test invalid payloads are rejected",
      "Implement TaskEventSchema discriminated union in src/types/events.ts",
      "Export TaskEvent type from src/types/index.ts",
      "Verify all tests pass"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create WorkflowSchema type definitions",
    "steps": [
      "Write tests for WorkflowColumnSchema and WorkflowSchemaZ in src/types/workflow.test.ts",
      "Create src/types/workflow.ts",
      "Implement WorkflowColumnSchema with all fields",
      "Implement WorkflowSchemaZ (named to avoid collision with type)",
      "Export types from src/types/index.ts",
      "Verify tests pass"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement taskStore with Zustand and immer",
    "steps": [
      "Write tests for taskStore in src/stores/taskStore.test.ts",
      "Test setTasks converts array to Record",
      "Test updateTask modifies existing task",
      "Test selectTask updates selectedTaskId",
      "Test selectTasksByStatus selector",
      "Test selectSelectedTask selector",
      "Create src/stores/taskStore.ts",
      "Implement TaskState interface",
      "Implement TaskActions interface",
      "Create store with immer middleware",
      "Implement all actions",
      "Export selectors",
      "Verify all tests pass"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement projectStore with Zustand",
    "steps": [
      "Write tests for projectStore in src/stores/projectStore.test.ts",
      "Test setProjects, updateProject, selectProject actions",
      "Test selectActiveProject selector",
      "Create src/stores/projectStore.ts",
      "Implement ProjectState and ProjectActions interfaces",
      "Create store with immer middleware",
      "Implement all actions and selectors",
      "Verify tests pass"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement uiStore for UI state",
    "steps": [
      "Write tests for uiStore in src/stores/uiStore.test.ts",
      "Test sidebar visibility toggle",
      "Test modal state management",
      "Test theme preference (if applicable)",
      "Create src/stores/uiStore.ts",
      "Implement UiState and UiActions interfaces",
      "Include: sidebarOpen, activeModal, notifications",
      "Verify tests pass"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement activityStore for agent messages",
    "steps": [
      "Write tests for activityStore in src/stores/activityStore.test.ts",
      "Test addMessage adds to messages array",
      "Test addAlert adds to alerts array",
      "Test clearMessages clears array",
      "Test message count limits (optional ring buffer)",
      "Create src/stores/activityStore.ts",
      "Implement ActivityState with messages and alerts",
      "Implement addMessage, addAlert, clearMessages actions",
      "Verify tests pass"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Extend Tauri API wrappers for tasks",
    "steps": [
      "Write tests for api.tasks methods in src/lib/tauri.test.ts",
      "Mock invoke to return test data",
      "Test list, get, create, update, move, delete methods",
      "Test Zod validation is applied",
      "Extend src/lib/tauri.ts with api.tasks namespace",
      "Implement all task API methods",
      "Verify tests pass"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Extend Tauri API wrappers for projects",
    "steps": [
      "Write tests for api.projects methods in src/lib/tauri.test.ts",
      "Test list, get, create, update, delete methods",
      "Extend src/lib/tauri.ts with api.projects namespace",
      "Implement all project API methods",
      "Verify tests pass"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Set up TanStack Query with QueryClientProvider",
    "steps": [
      "Create src/lib/queryClient.ts with QueryClient configuration",
      "Configure default options: retry, staleTime, etc.",
      "Update App.tsx to wrap with QueryClientProvider",
      "Add ReactQueryDevtools in development mode",
      "Write integration test verifying QueryClient is available"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement useTasks hook with TanStack Query",
    "steps": [
      "Write tests for useTasks in src/hooks/useTasks.test.ts",
      "Test successful data fetching",
      "Test error handling",
      "Test loading state",
      "Create src/hooks/useTasks.ts",
      "Implement useTasks hook using useQuery",
      "Configure queryKey with projectId",
      "Verify tests pass"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement useProjects hook with TanStack Query",
    "steps": [
      "Write tests for useProjects in src/hooks/useProjects.test.ts",
      "Test successful data fetching",
      "Test error handling",
      "Create src/hooks/useProjects.ts",
      "Implement useProjects hook using useQuery",
      "Verify tests pass"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement useTaskMutation hook",
    "steps": [
      "Write tests for useTaskMutation in src/hooks/useTaskMutation.test.ts",
      "Test move mutation invalidates cache",
      "Test create mutation",
      "Test update mutation",
      "Test delete mutation",
      "Create src/hooks/useTaskMutation.ts",
      "Implement mutations with optimistic updates (optional)",
      "Verify tests pass"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement useTaskEvents hook with Tauri event listening",
    "steps": [
      "Write tests for useTaskEvents in src/hooks/useEvents.test.ts",
      "Mock @tauri-apps/api/event listen function",
      "Test event listener is set up on mount",
      "Test cleanup on unmount",
      "Test valid events update store",
      "Test invalid events are logged",
      "Create src/hooks/useEvents.ts",
      "Implement useTaskEvents with TaskEventSchema validation",
      "Handle all event types: created, updated, deleted, status_changed",
      "Verify tests pass"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement useAgentEvents hook for activity stream",
    "steps": [
      "Write tests for useAgentEvents in src/hooks/useEvents.test.ts",
      "Test filtering by taskId",
      "Test adding messages to activityStore",
      "Add useAgentEvents to src/hooks/useEvents.ts",
      "Implement with optional taskId filter",
      "Verify tests pass"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement useSupervisorAlerts hook",
    "steps": [
      "Write tests for useSupervisorAlerts in src/hooks/useEvents.test.ts",
      "Test alerts are added to activityStore",
      "Add useSupervisorAlerts to src/hooks/useEvents.ts",
      "Listen to supervisor:alert event",
      "Verify tests pass"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Implement event batching hook for performance",
    "steps": [
      "Write tests for useBatchedAgentMessages in src/hooks/useBatchedEvents.test.ts",
      "Test messages are buffered",
      "Test buffer is flushed every 50ms",
      "Test cleanup on unmount",
      "Create src/hooks/useBatchedEvents.ts",
      "Implement useBatchedAgentMessages with useRef buffer",
      "Use setInterval for periodic flush",
      "Verify tests pass"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create EventProvider component for global listeners",
    "steps": [
      "Write tests for EventProvider in src/providers/EventProvider.test.tsx",
      "Test that all global event hooks are called",
      "Test children are rendered",
      "Create src/providers/EventProvider.tsx",
      "Call useTaskEvents, useSupervisorAlerts, useReviewEvents, useFileChangeEvents",
      "Return children wrapped in fragment",
      "Verify tests pass"
    ],
    "passes": true
  },
  {
    "category": "integration",
    "description": "Integrate EventProvider and QueryClientProvider in App",
    "steps": [
      "Write integration test for App in src/App.test.tsx",
      "Test QueryClientProvider is present",
      "Test EventProvider is present",
      "Update App.tsx to wrap content with both providers",
      "QueryClientProvider should be outermost",
      "EventProvider inside QueryClientProvider",
      "Verify app starts with `npm run tauri dev`"
    ],
    "passes": true
  },
  {
    "category": "feature",
    "description": "Create formatters utility module",
    "steps": [
      "Write tests for formatters in src/lib/formatters.test.ts",
      "Test formatDate with various inputs",
      "Test formatRelativeTime (e.g., '2 hours ago')",
      "Test formatDuration (e.g., '5m 30s')",
      "Create src/lib/formatters.ts",
      "Implement formatDate, formatRelativeTime, formatDuration",
      "Verify tests pass"
    ],
    "passes": true
  },
  {
    "category": "testing",
    "description": "Create test utilities for stores and hooks",
    "steps": [
      "Create src/test/store-utils.ts with renderHookWithProviders helper",
      "Include QueryClientProvider wrapper",
      "Include store reset utilities",
      "Create src/test/mock-data.ts with sample tasks, projects",
      "Document usage in test files"
    ],
    "passes": true
  }
]
```
