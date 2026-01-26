# RalphX Frontend - CLAUDE.md

This document provides context about the React/TypeScript frontend codebase for progressive discovery.

## Tech Stack

| Technology | Version | Purpose |
|------------|---------|---------|
| React | 19.1 | UI framework |
| TypeScript | 5.8 | Type-safe JavaScript |
| Zustand | 5.0 | State management (with immer middleware) |
| TanStack Query | 5.90 | Server state, caching, data fetching |
| Tailwind CSS | 4.1 | Utility-first CSS |
| Zod | 4.3 | Runtime schema validation |
| dnd-kit | 6.3 | Drag-and-drop (Kanban board) |
| Vite | 7.0 | Build tool and dev server |
| Vitest | 4.0 | Unit testing framework |
| Testing Library | 16.3 | Component testing utilities |
| Tauri API | 2.x | Native backend communication |

---

## Directory Structure

```
src/
├── api/                    # Tauri API wrappers (ideation, proposals, chat)
│   ├── ideation.ts         # Session/proposal/dependency API with transforms
│   ├── proposal.ts         # Proposal-specific API
│   └── chat.ts             # Context-aware chat API (sendContextMessage, conversations, agent runs, execution chat)
│
├── components/             # React components
│   ├── Chat/               # Context-aware chat (Phase 15)
│   │   ├── ChatPanel       # Main chat interface with conversation switching
│   │   ├── ChatMessage     # Message bubbles with tool call display
│   │   ├── ChatInput       # Input with queue mode and keyboard navigation
│   │   ├── ConversationSelector  # History dropdown with new conversation
│   │   ├── QueuedMessage   # Individual queued message (edit/delete)
│   │   ├── QueuedMessageList     # Queue UI shown when agent running
│   │   └── ToolCallIndicator     # Collapsible tool call display (with artifact context preview - Phase 17)
│   ├── PermissionDialog.tsx      # UI-based permission approval for agent tools
│   ├── execution/          # Execution control (ExecutionControlBar)
│   ├── Ideation/           # Ideation view (ProposalCard, ProposalList, etc.)
│   │   ├── PlanDisplay     # Plan artifact viewer with collapse/expand (Phase 16)
│   │   ├── PlanEditor      # Markdown editor for plan artifacts (Phase 16)
│   │   └── PlanTemplateSelector # Template picker for new plans (Phase 16)
│   ├── modals/             # Modal dialogs (AskUserQuestionModal)
│   ├── qa/                 # QA components (TaskQABadge, QASettingsPanel)
│   ├── reviews/            # Review system (ReviewsPanel, ReviewCard)
│   ├── settings/           # Settings components
│   │   └── IdeationSettingsPanel # Ideation plan mode configuration (Phase 16)
│   ├── Task/               # Task components (TaskBoard/, TaskDetailView, TaskContextPanel - Phase 17)
│   │   ├── TaskBoard/      # Kanban board with dnd-kit integration
│   │   ├── TaskContextPanel # View task context (proposal, plan, related artifacts)
│   │   └── TaskDetailPanel # Task detail view with "View Context" button
│   └── ui/                 # Shared UI components (StatusBadge)
│
├── hooks/                  # Custom React hooks
│   ├── use*.ts             # TanStack Query hooks (useTasks, useProjects, etc.)
│   ├── useChat.ts          # Context-aware chat with conversation switching, queue, events
│   ├── useEvents.ts        # Tauri event listeners
│   └── use*Mutation.ts     # Mutation hooks for data updates
│
├── lib/                    # Utility libraries
│   ├── api/                # Additional Tauri API wrappers (workflows, artifacts, task-context - Phase 17, etc.)
│   ├── tauri.ts            # Main Tauri invoke wrapper with Zod validation
│   ├── queryClient.ts      # TanStack Query client configuration
│   ├── validation.ts       # Shared validation utilities
│   └── formatters.ts       # Display formatting utilities
│
├── providers/              # React context providers
│   └── EventProvider.tsx   # Global Tauri event listener setup
│
├── stores/                 # Zustand stores
│   ├── taskStore.ts        # Task state
│   ├── projectStore.ts     # Project state
│   ├── uiStore.ts          # UI state (modals, sidebar, views)
│   ├── ideationStore.ts    # Ideation session state (includes planArtifact state - Phase 16)
│   ├── proposalStore.ts    # Task proposal state
│   ├── chatStore.ts        # Chat state (active conversation, queue, agent running, execution queue)
│   ├── qaStore.ts          # QA state
│   ├── reviewStore.ts      # Review state
│   └── ...Store.ts         # Additional domain stores
│
├── styles/                 # CSS files
│   └── globals.css         # Design tokens and base styles
│
├── test/                   # Test utilities
│   ├── setup.ts            # Vitest setup (mocks Tauri APIs)
│   └── store-utils.ts      # Store testing utilities
│
├── types/                  # TypeScript types and Zod schemas
│   ├── index.ts            # Re-exports all types
│   ├── task.ts             # Task type and schemas (includes sourceProposalId, planArtifactId - Phase 16)
│   ├── task-context.ts     # TaskContext, TaskProposalSummary, ArtifactSummary types (Phase 17)
│   ├── project.ts          # Project type and schemas
│   ├── status.ts           # InternalStatus enum (14 statuses)
│   ├── chat-conversation.ts # ChatConversation and AgentRun types (Phase 15A/15B, includes task_execution context type)
│   ├── permission.ts       # Permission request types for UI-based approval
│   ├── ideation.ts         # Ideation types (includes planArtifactId fields - Phase 16)
│   ├── ideation-config.ts  # IdeationSettings and IdeationPlanMode types (Phase 16)
│   └── *.ts                # Domain-specific types (qa, review, ideation, etc.)
│
├── integration/            # Integration tests
│   └── qa-ui-flow.test.tsx # End-to-end UI flow tests
│
├── App.tsx                 # Main application shell
└── main.tsx                # React entry point
```

---

## Key Patterns

### 1. Zustand Stores with Immer

All stores use Zustand with immer middleware for immutable updates:

```typescript
import { create } from "zustand";
import { immer } from "zustand/middleware/immer";

interface TaskState {
  tasks: Record<string, Task>;  // O(1) lookup by ID
  selectedTaskId: string | null;
}

interface TaskActions {
  setTasks: (tasks: Task[]) => void;
  updateTask: (taskId: string, changes: Partial<Task>) => void;
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
  }))
);

// Selectors defined outside store for memoization
export const selectTasksByStatus = (status: InternalStatus) =>
  (state: TaskState): Task[] =>
    Object.values(state.tasks).filter((t) => t.internalStatus === status);
```

### 2. TanStack Query with Tauri

Data fetching uses TanStack Query with typed Tauri invokes:

```typescript
// Query key factory pattern
export const taskKeys = {
  all: ["tasks"] as const,
  lists: () => [...taskKeys.all, "list"] as const,
  list: (projectId: string) => [...taskKeys.lists(), projectId] as const,
  details: () => [...taskKeys.all, "detail"] as const,
  detail: (taskId: string) => [...taskKeys.details(), taskId] as const,
};

// Hook using TanStack Query
export function useTasks(projectId: string) {
  return useQuery<Task[], Error>({
    queryKey: taskKeys.list(projectId),
    queryFn: () => api.tasks.list(projectId),
  });
}
```

### 3. Typed Tauri API with Zod Validation

All Tauri commands are wrapped with runtime Zod validation:

```typescript
// Generic typed invoke
export async function typedInvoke<T>(
  cmd: string,
  args: Record<string, unknown>,
  schema: z.ZodType<T>
): Promise<T> {
  const result = await invoke(cmd, args);
  return schema.parse(result);
}

// API object with all commands
export const api = {
  tasks: {
    list: (projectId: string) =>
      typedInvoke("list_tasks", { projectId }, TaskListSchema),
    create: (input: CreateTask) =>
      typedInvoke("create_task", { input }, TaskSchema),
  },
  // ...
};
```

### 4. Types with Zod Schemas

Types are defined using Zod schemas for runtime validation:

```typescript
import { z } from "zod";

export const TaskSchema = z.object({
  id: z.string().min(1),
  projectId: z.string().min(1),
  title: z.string().min(1),
  description: z.string().nullable(),
  priority: z.number().int(),
  internalStatus: InternalStatusSchema,
  createdAt: z.string().datetime(),
  updatedAt: z.string().datetime(),
});

export type Task = z.infer<typeof TaskSchema>;
```

### 5. Component Organization

Components follow a feature-based structure with co-located tests:

```
components/tasks/TaskBoard/
├── index.tsx           # Public exports
├── TaskBoard.tsx       # Main component
├── TaskBoard.test.tsx  # Component tests
├── TaskCard.tsx        # Child component
├── TaskCard.test.tsx   # Child tests
├── Column.tsx          # Column component
├── hooks.ts            # Board-specific hooks
├── hooks.test.tsx      # Hook tests
├── validation.ts       # Board validation logic
├── validation.test.ts  # Validation tests
└── reorder.ts          # Drag-drop reorder logic
```

### 6. Event-Driven Updates

Global events from Tauri are handled via EventProvider:

```typescript
export function EventProvider({ children }: EventProviderProps) {
  useTaskEvents();       // task:created, task:updated, task:deleted
  useSupervisorAlerts(); // supervisor:alert
  useReviewEvents();     // review:* events
  useFileChangeEvents(); // file:changed

  return <>{children}</>;
}
```

### 7. Path Aliases

Import paths use the `@/` alias (configured in vite.config.ts and tsconfig.json):

```typescript
import { Task } from "@/types/task";
import { useTaskStore } from "@/stores/taskStore";
import { api } from "@/lib/tauri";
```

### 8. Context-Aware Chat (Phase 15A & 15B)

The chat system supports multiple conversations per context with MCP tool integration:

```typescript
// useChat hook provides full chat functionality
export function useChat() {
  const { contextType, contextId } = useActiveContext();

  const hook = useChat({
    contextType,
    contextId
  });

  // hook provides:
  // - messages: current conversation's messages
  // - conversations: list of all conversations for this context
  // - activeConversation: current conversation object
  // - agentRunStatus: current agent run status
  // - sendMessage: send message (or queue if agent running)
  // - switchConversation: switch to different conversation
  // - createConversation: start new conversation
}
```

**Key features:**
- **Multiple conversations per context** - Each ideation session, task, or project can have multiple chat conversations
- **Task execution chat (Phase 15B)** - Worker execution output displayed as chat with task_execution context type
- **Conversation switching** - ConversationSelector component lets you switch between conversations or start new ones
- **Execution history** - View past execution attempts for a task, switch between them

### 9. Ideation Plan Artifacts (Phase 16)

The ideation system supports implementation plans as artifacts before task proposal creation:

**IdeationSettings:**
- **Plan Mode** - Required, Optional, or Parallel workflow
  - `Required` - Plan must be created before proposals
  - `Optional` - Plan suggested for complex features (default)
  - `Parallel` - Plan and proposals created together
- **Plan Approval** - Require explicit approval when `require_plan_approval` is true
- **Auto-suggest plans** - Suggest plans for complex features in Optional mode
- **Auto-link proposals** - Automatically link proposals to session plan

**Plan Components:**

```typescript
// PlanDisplay component
export function PlanDisplay({
  planArtifact,
  sessionId,
  requireApproval
}: PlanDisplayProps) {
  // - Shows plan title and markdown content (with syntax highlighting)
  // - Collapse/expand functionality
  // - Edit and Export buttons in header
  // - 'Approve Plan' button when require_plan_approval is true
  // - Plan-proposal linkage indicator
}

// PlanEditor component
export function PlanEditor({
  artifactId,
  initialContent,
  onSave,
  onCancel
}: PlanEditorProps) {
  // - Markdown editor with preview toggle
  // - Save and Cancel buttons
  // - Calls HTTP endpoint POST /api/update_plan_artifact
}

// PlanTemplateSelector component
export function PlanTemplateSelector({
  onSelectTemplate
}: PlanTemplateSelectorProps) {
  // - Fetches templates from active methodology
  // - Shows dropdown only when templates available
  // - Pre-populates plan content with selected template
}

// IdeationSettingsPanel component
export function IdeationSettingsPanel() {
  // - Plan Workflow Mode radio group (Required/Optional/Parallel)
  // - 'Require explicit approval' checkbox
  // - 'Suggest plans for complex features' checkbox
  // - 'Auto-link proposals to session plan' checkbox
  // - Uses shadcn RadioGroup and Checkbox components
}
```

**State Management:**

```typescript
// ideationStore.ts includes plan artifact state
interface IdeationState {
  sessions: Record<string, IdeationSession>;
  planArtifact: Artifact | null; // Current session's plan artifact
  // ...
}

interface IdeationActions {
  fetchPlanArtifact: (artifactId: string) => Promise<void>;
  // ...
}
```

**Plan Features:**
- **Plan versioning** - Track `plan_version_at_creation` on proposals for historical view
- **Historical view** - "View plan as of creation" link on proposals when plan has been updated
- **Export/Import** - Download/upload plan as markdown file
- **Proactive sync** - ArtifactFlow auto-updates proposals when plan changes (with undo)
- **Task traceability** - Tasks track `source_proposal_id` and `plan_artifact_id` for worker context

**MCP Tools (orchestrator-ideation):**
- `create_plan_artifact(session_id, title, content)` - Create new plan
- `update_plan_artifact(artifact_id, content)` - Update existing plan
- `get_plan_artifact(artifact_id)` - Fetch plan content
- `link_proposals_to_plan(proposal_ids, artifact_id)` - Link proposals to plan
- `get_session_plan(session_id)` - Get current session's plan
- **Message queueing** - Messages sent while agent running are queued and auto-sent on completion
- **Tool call display** - ToolCallIndicator shows collapsible view of MCP tool calls
- **Keyboard navigation** - Up arrow in empty input edits last queued message
- **Real-time updates** - Subscribes to Tauri events:
  - Context-aware chat: `chat:chunk`, `chat:tool_call`, `chat:run_completed`
  - Task execution: `execution:chunk`, `execution:tool_call`, `execution:run_completed`
- **Permission system** - PermissionDialog provides UI-based approval for non-pre-approved tools

**Context Types:**
| Context Type | Where It Appears | Purpose |
|--------------|-----------------|---------|
| `ideation` | Ideation view | Chat with orchestrator-ideation agent (can create task proposals) |
| `task` | Task detail (chat mode) | Chat with chat-task agent (can update task, add notes) |
| `project` | Project view | Chat with chat-project agent (can suggest tasks) |
| `task_execution` | Task detail (executing status) | View worker execution output, queue messages to worker |

**Architecture:**
```
ChatPanel (with ConversationSelector)
    ↓
useChat hook
    ↓
chatApi.sendContextMessage() (or execution chat API for task_execution)
    ↓
Tauri backend spawns Claude CLI with --agent flag
    ↓
MCP server (ralphx-mcp-server) provides scoped tools
    ↓
Tool calls displayed in chat UI
```

### 10. Worker Artifact Context (Phase 17)

Workers can dynamically fetch and use artifacts linked to the task being executed. This provides workers with implementation plans, research documents, and related artifacts before beginning work.

**Task Context API:**

```typescript
// Task context types
export interface TaskContext {
  task: Task;
  sourceProposal: TaskProposalSummary | null;
  planArtifact: ArtifactSummary | null;
  relatedArtifacts: ArtifactSummary[];
  contextHints: string[];
}

export interface TaskProposalSummary {
  id: string;
  title: string;
  description: string | null;
  acceptanceCriteria: string[];
  implementationNotes: string | null;
  planVersionAtCreation: number | null;
}

export interface ArtifactSummary {
  id: string;
  title: string;
  artifactType: string;
  currentVersion: number;
  contentPreview: string; // 500-char preview
}

// API functions (src/api/task-context.ts)
export async function getTaskContext(taskId: string): Promise<TaskContext>;
export async function getArtifactFull(artifactId: string): Promise<Artifact>;
export async function getArtifactVersion(artifactId: string, version: number): Promise<Artifact>;
export async function getRelatedArtifacts(artifactId: string): Promise<ArtifactRelation[]>;
export async function searchArtifacts(projectId: string, query: string, artifactTypes?: string[]): Promise<ArtifactSummary[]>;
```

**TaskContextPanel Component:**

```typescript
// src/components/Task/TaskContextPanel.tsx
export function TaskContextPanel({ taskId }: TaskContextPanelProps) {
  // - Fetches task context via getTaskContext(taskId)
  // - Shows linked proposal summary (title, description, acceptance criteria) if present
  // - Shows plan artifact preview with "View Full" button
  // - Lists related artifacts with type icons
  // - Displays context hints for the worker
  // - Loading and empty states
  // - Uses shadcn Card and Collapsible components
}
```

**TaskDetailPanel Integration:**

The "View Context" button appears in TaskDetailPanel when the task has `sourceProposalId` or `planArtifactId`:

```typescript
// Conditional rendering in TaskDetailPanel
{(task.sourceProposalId || task.planArtifactId) && (
  <Button onClick={() => setShowContext(true)}>
    View Context
  </Button>
)}

{showContext && (
  <TaskContextPanel taskId={task.id} />
)}
```

**Artifact Indicators in Task Views:**

Task cards and detail views show visual indicators for linked artifacts:

```typescript
// TaskCard.tsx and TaskDetailView.tsx
{task.planArtifactId && (
  <Tooltip content="Has implementation plan">
    <FileText className="w-3.5 h-3.5 text-[var(--accent-primary)]" />
  </Tooltip>
)}

{task.sourceProposalId && (
  <Tooltip content="Created from proposal">
    <Lightbulb className="w-3.5 h-3.5 text-[var(--accent-secondary)]" />
  </Tooltip>
)}
```

**Tool Call Display in Execution Chat:**

The `ToolCallIndicator` component now detects and previews artifact context tools:

```typescript
// Enhanced tool call summaries
- get_task_context: Shows task title and number of linked artifacts
- get_artifact: Shows artifact title and content preview
- get_artifact_version: Shows artifact title and version number
- get_related_artifacts: Shows count of related artifacts
- search_project_artifacts: Shows query and result count
```

**MCP Tools for Workers:**

The worker agent has access to 5 artifact context MCP tools (scoped via `RALPHX_AGENT_TYPE=worker`):

| Tool | Purpose |
|------|---------|
| `get_task_context` | Get task with proposal summary, plan preview, related artifacts |
| `get_artifact` | Fetch full artifact content by ID |
| `get_artifact_version` | Fetch specific version of an artifact |
| `get_related_artifacts` | Get artifacts related to a given artifact |
| `search_project_artifacts` | Search for artifacts by query and type |

**Worker Instructions:**

Workers are instructed to fetch context before implementing (see `ralphx-plugin/agents/worker.md`):

1. **Step 1: Get Task Context** - Always call `get_task_context` first
2. **Step 2: Read Implementation Plan** - If `planArtifact` present, fetch with `get_artifact`
3. **Step 3: Fetch Related Artifacts** - Optional for complex tasks
4. **Step 4: Begin Implementation** - Start work with full context

**Key Features:**

- **Manual context fetch** - Workers decide what context is relevant
- **500-char preview** - Prevents context bloat; full content requires explicit fetch
- **Historical version access** - Workers can fetch `plan_version_at_creation` from proposal
- **Search capability** - Workers can search for related artifacts by query
- **Visual indicators** - Users see which tasks have rich context before opening

---

## Coding Standards

### TypeScript Configuration

The codebase uses strict TypeScript settings:

- `strict: true`
- `noUncheckedIndexedAccess: true` - Array/object access returns `T | undefined`
- `noImplicitReturns: true`
- `exactOptionalPropertyTypes: true` - Optional props must match exactly
- `verbatimModuleSyntax: true` - Explicit type-only imports

### Conditional Props Pattern

For optional props with `exactOptionalPropertyTypes`:

```typescript
// Build props conditionally to satisfy exactOptionalPropertyTypes
const qaBadgeProps = {
  needsQA: needsQA ?? false,
  ...(prepStatus !== undefined && { prepStatus }),
  ...(testStatus !== undefined && { testStatus }),
};
```

### CSS Variables (Design Tokens)

Styles use CSS custom properties defined in `globals.css`:

```typescript
// Use design tokens instead of hardcoded colors
style={{ backgroundColor: "var(--bg-elevated)", color: "var(--text-primary)" }}
```

Key tokens:
- Backgrounds: `--bg-base`, `--bg-surface`, `--bg-elevated`, `--bg-hover`
- Text: `--text-primary`, `--text-secondary`, `--text-muted`
- Accent: `--accent-primary` (warm orange #ff6b35), `--accent-secondary`
- Status: `--status-success`, `--status-warning`, `--status-error`, `--status-info`
- Borders: `--border-subtle`, `--border-default`, `--border-focus`

### Tailwind CSS v4 Configuration

RalphX uses **Tailwind CSS v4** which has a different configuration pattern than v3:

**Critical Rules:**
- ❌ NO `tailwind.config.js` file - v4 ignores it completely
- ❌ NO `tailwindcss-animate` package - deprecated in v4
- ✅ Use `@tailwindcss/vite` plugin in `vite.config.ts`
- ✅ All theme config goes in `globals.css` via `@theme inline`

**Configuration Files:**
| File | Purpose |
|------|---------|
| `vite.config.ts` | Contains `@tailwindcss/vite` plugin |
| `styles/globals.css` | Contains `@theme inline` with all design tokens |
| `components.json` | shadcn config with `"config": ""` (empty for v4) |

**CSS Structure in globals.css:**

```css
@import "tailwindcss";

/* 1. Define CSS variables at root level (NOT in @layer base) */
:root {
  --bg-base: hsl(0 0% 6%);           /* hsl() wrapper required */
  --accent-primary: hsl(14 100% 60%);
}

/* 2. Map variables to Tailwind utilities */
@theme inline {
  --color-bg-base: var(--bg-base);
  --color-accent-primary: var(--accent-primary);
}

/* 3. Apply base styles */
@layer base {
  body {
    background-color: var(--bg-base);  /* NO hsl() here - already wrapped */
  }
}
```

**Using Design Tokens:**
- Tailwind classes: `bg-bg-base`, `text-accent-primary`, `bg-background`
- Inline styles: `var(--bg-base)`, `var(--accent-primary)`

### Anti-AI-Slop Design

Per the master plan:
- NO purple gradients
- NO Inter font (uses SF Pro / system fonts)
- NO generic icon grids
- Warm orange accent color (#ff6b35)

---

## Testing

### Test Framework

- **Vitest** for unit and integration tests
- **Testing Library** for component rendering
- **jsdom** environment for DOM testing

### Running Tests

```bash
# Run all tests in watch mode
pnpm test

# Run tests once
pnpm test:run

# Run with coverage
pnpm test:coverage

# Type check without emit
pnpm typecheck
```

### Test Setup

Tests automatically mock Tauri APIs (see `test/setup.ts`):

```typescript
// Tauri invoke is mocked
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

// Tauri events are mocked
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {})),
  emit: vi.fn(),
}));
```

### Testing Stores

Use the store-utils for testing Zustand stores:

```typescript
import { useTaskStore } from "@/stores/taskStore";

// Reset store before each test
beforeEach(() => {
  useTaskStore.setState({ tasks: {}, selectedTaskId: null });
});
```

### Testing Components

```typescript
import { render, screen } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import { TaskCard } from "./TaskCard";

const wrapper = ({ children }) => (
  <QueryClientProvider client={queryClient}>
    {children}
  </QueryClientProvider>
);

test("renders task title", () => {
  render(<TaskCard task={mockTask} />, { wrapper });
  expect(screen.getByTestId("task-title")).toHaveTextContent("My Task");
});
```

---

## Development

### Scripts

```bash
pnpm dev           # Start Vite dev server (port 1420)
pnpm build         # TypeScript compile + Vite build
pnpm preview       # Preview production build
pnpm tauri         # Run Tauri commands
pnpm test          # Run Vitest in watch mode
pnpm test:run      # Run Vitest once
pnpm typecheck     # TypeScript type check
```

### Environment

The frontend runs on port 1420 (fixed for Tauri). Hot module replacement is enabled.

### Adding New Features

1. **Types**: Define Zod schema in `types/`, export from `types/index.ts`
2. **API**: Add Tauri command wrapper in `lib/tauri.ts` or `lib/api/`
3. **Store**: Create Zustand store in `stores/` with immer middleware
4. **Hook**: Create TanStack Query hook in `hooks/`
5. **Component**: Create component with co-located test file
6. **Tests**: Write tests FIRST (TDD is mandatory per project guidelines)

---

## Key Files Reference

| File | Purpose |
|------|---------|
| `App.tsx` | Main application shell with providers and routing |
| `main.tsx` | React entry point |
| `lib/tauri.ts` | All Tauri command wrappers with Zod validation |
| `lib/queryClient.ts` | TanStack Query client configuration |
| `stores/uiStore.ts` | UI state (modals, sidebar, views, execution status) |
| `types/index.ts` | Central type exports |
| `types/status.ts` | 14 internal task statuses |
| `styles/globals.css` | Design tokens, `@theme inline`, and base styles (Tailwind v4) |
| `test/setup.ts` | Vitest setup with Tauri mocks |
| `../vite.config.ts` | Vite config with `@tailwindcss/vite` plugin |
| `../components.json` | shadcn/ui config (empty `config` field for v4) |
