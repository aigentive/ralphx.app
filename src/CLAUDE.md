# src/CLAUDE.md (COMPACT) — Frontend

## Stack
React 19.1 | TS 5.8 | Zustand 5.0+immer | TanStack Query 5.90 | Tailwind 4.1 | Zod 4.3
dnd-kit 6.3 | Vite 7.0 | Vitest 4.0 | Testing Library 16.3 | Tauri API 2.x

## Structure
```
src/
├─ api/                  # Tauri wrappers: ideation.ts, proposal.ts, chat.ts
├─ components/
│  ├─ Chat/              # ChatPanel, ChatMessage, ChatInput, ConversationSelector,
│  │                     # QueuedMessage*, ToolCallIndicator (artifact preview Ph17)
│  ├─ PermissionDialog   # UI tool approval
│  ├─ execution/         # ExecutionControlBar
│  ├─ Ideation/          # ProposalCard/List, PlanDisplay|Editor|TemplateSelector (Ph16)
│  ├─ modals/            # AskUserQuestionModal
│  ├─ qa/                # TaskQABadge, QASettingsPanel
│  ├─ reviews/           # ReviewsPanel, ReviewCard
│  ├─ settings/          # IdeationSettingsPanel (Ph16)
│  ├─ Task/              # TaskBoard/ (dnd-kit), TaskDetailPanel, TaskContextPanel (Ph17),
│  │                     # StatusDropdown, TaskEditForm, InlineTaskAdd, TaskSearchBar,
│  │                     # EmptySearchState, TaskCardContextMenu (Ph18)
│  └─ ui/                # StatusBadge, shadcn components
├─ hooks/                # useTasks, useChat, useEvents, use*Mutation,
│                        # useInfiniteTasksQuery, useTaskSearch (Ph18)
├─ lib/
│  ├─ api/               # Additional wrappers (workflows, artifacts, task-context)
│  ├─ tauri.ts           # typedInvoke with Zod validation
│  ├─ queryClient.ts     # TanStack config
│  └─ validation.ts, formatters.ts
├─ providers/            # EventProvider (global Tauri events)
├─ stores/               # *Store.ts (Zustand+immer): task, project, ui, ideation,
│                        # proposal, chat, qa, review
│                        # uiStore (Ph18): showArchived, boardSearchQuery, isSearching
├─ styles/globals.css    # @theme inline, design tokens
├─ test/                 # setup.ts (mocks Tauri), store-utils.ts
├─ types/                # Zod schemas: task, project, status (14 states),
│                        # chat-conversation, permission, ideation, ideation-config,
│                        # task-context (Ph17), TaskListResponse, StatusTransition (Ph18)
└─ App.tsx, main.tsx
```

## Pattern 1: Zustand + Immer
```typescript
const useTaskStore = create<State & Actions>()(immer((set) => ({
  tasks: {},  // Record<id, Task> for O(1)
  updateTask: (id, changes) => set(s => { Object.assign(s.tasks[id], changes) })
})));
// Selectors outside for memoization
export const selectByStatus = (status) => (s) => Object.values(s.tasks).filter(...)
```

## Pattern 2: TanStack Query + Tauri
```typescript
// Key factory
const taskKeys = { all:["tasks"], list:(pid)=>[...taskKeys.all,"list",pid], detail:(tid)=>[...taskKeys.all,"detail",tid] }
// Hook
const useTasks = (pid) => useQuery({ queryKey:taskKeys.list(pid), queryFn:()=>api.tasks.list(pid) })
```

## Pattern 3: Typed Tauri + Zod
```typescript
async function typedInvoke<T>(cmd, args, schema: z.ZodType<T>): Promise<T> {
  return schema.parse(await invoke(cmd, args))
}
```

## ⚠️ CRITICAL: Tauri Param Conventions
| Context | JS Side | Rust Side | Why |
|---------|---------|-----------|-----|
| Direct params | `{ contextType, contextId }` (camelCase) | `context_type, context_id` | Tauri auto-converts |
| Struct fields | `{ input: { context_type, context_id } }` (snake_case) | `struct { context_type, context_id }` | serde exact-match |

**Add `#[serde(rename_all="camelCase")]` to Rust struct if you want camelCase in JS struct fields.**

Errors: "missing required key contextType" → used snake_case for direct param
        "missing required key input" → forgot struct wrapper

## Pattern 4: Types via Zod
```typescript
const TaskSchema = z.object({ id:z.string().min(1), internalStatus:InternalStatusSchema, ... })
type Task = z.infer<typeof TaskSchema>
```

## Pattern 5: Component Org
```
Component/
├─ index.tsx, Component.tsx, Component.test.tsx
├─ ChildComponent.tsx, hooks.ts, validation.ts, reorder.ts
└─ *.test.tsx (co-located)
```

## Pattern 6: Event-Driven Updates
```typescript
// EventProvider wraps app
function EventProvider({children}) {
  useTaskEvents();       // task:created/updated/deleted
  useSupervisorAlerts(); // supervisor:alert
  useReviewEvents();     // review:*
  useFileChangeEvents(); // file:changed
  return <>{children}</>
}
```

## Pattern 7: Path Aliases
`import { Task } from "@/types/task"` — configured in vite.config.ts + tsconfig.json

## Context-Aware Chat (Ph15A/15B)
```typescript
const { messages, conversations, activeConversation, agentRunStatus, sendMessage, switchConversation } = useChat({ contextType, contextId })
```
Context types: `ideation` | `task` | `project` | `task_execution`
Events: `chat:chunk|tool_call|run_completed` (ideation) | `execution:chunk|tool_call|run_completed` (worker)
Queue: messages during agent run → queued → auto-sent on completion

## Ideation Plans (Ph16)
```typescript
// IdeationSettings
type IdeationPlanMode = "Required" | "Optional" | "Parallel"
interface IdeationSettings { plan_mode, require_plan_approval, suggest_plans_for_complex, auto_link_to_session_plan }

// Components
PlanDisplay: collapse/expand, edit/export, approval button
PlanEditor: markdown editor + preview toggle
PlanTemplateSelector: methodology templates dropdown
IdeationSettingsPanel: RadioGroup(mode) + Checkboxes(approval, suggest, auto-link)

// State (ideationStore)
planArtifact: Artifact | null
fetchPlanArtifact(artifactId)

// MCP Tools (orchestrator-ideation)
create_plan_artifact(session_id, title, content)
update_plan_artifact(artifact_id, content)
get_plan_artifact(artifact_id)
link_proposals_to_plan(proposal_ids, artifact_id)
get_session_plan(session_id)
```
Traceability: Task.sourceProposalId, Task.planArtifactId | Proposal.planVersionAtCreation

## Worker Artifact Context (Ph17)
```typescript
interface TaskContext { task, sourceProposal: TaskProposalSummary|null, planArtifact: ArtifactSummary|null, relatedArtifacts[], contextHints[] }
interface ArtifactSummary { id, title, artifactType, currentVersion, contentPreview } // 500-char preview

// API (src/api/task-context.ts)
getTaskContext(taskId) → TaskContext
getArtifactFull(artifactId) → Artifact
getArtifactVersion(artifactId, version) → Artifact
getRelatedArtifacts(artifactId) → ArtifactRelation[]
searchArtifacts(projectId, query, types?) → ArtifactSummary[]

// TaskContextPanel: shows proposal summary, plan preview, related artifacts, context hints
// TaskDetailPanel: "View Context" button when sourceProposalId || planArtifactId
// TaskCard indicators: FileText icon (plan) | Lightbulb icon (proposal)
// ToolCallIndicator: previews get_task_context, get_artifact* tool results
```

## Task CRUD, Archive & Search (Ph18)
```typescript
// Types (task.ts)
interface Task { archivedAt: string | null, ... }
interface TaskListResponse { tasks: Task[], total: number, hasMore: boolean, offset: number }
interface StatusTransition { status: string, label: string }

// API (lib/tauri.ts)
api.tasks.archive(taskId)
api.tasks.restore(taskId)
api.tasks.permanentlyDelete(taskId)
api.tasks.getArchivedCount(projectId)
api.tasks.getValidTransitions(taskId)
api.tasks.search(projectId, query, includeArchived?)
api.tasks.list({ projectId, status?, offset?, limit?, includeArchived? })

// Mutations (hooks/useTaskMutation.ts)
archiveMutation, restoreMutation, permanentlyDeleteMutation
// States: isArchiving, isRestoring, isPermanentlyDeleting

// uiStore
showArchived: boolean, setShowArchived(show)
boardSearchQuery: string | null, setBoardSearchQuery(query)
isSearching: boolean, setIsSearching(searching)

// Hooks (hooks/)
useInfiniteTasksQuery({ projectId, status?, includeArchived? })
  → { data, fetchNextPage, hasNextPage, isFetchingNextPage, isLoading, flattenPages }
useTaskSearch({ projectId, query, includeArchived? })
  → { data: Task[], isLoading, isError }

// Components (components/tasks/)
StatusDropdown: fetches valid transitions from state machine, shows dropdown
TaskEditForm: edit task details (title, category, description, priority)
InlineTaskAdd: ghost card for quick-add in columns (draft, backlog)
TaskSearchBar: search input with result count, loading, close button
EmptySearchState: message-in-a-bottle when search returns no results
TaskCardContextMenu: right-click menu (view, edit, archive, restore, delete)

// TaskDetailModal (Ph18 updates)
- Edit mode toggle (Pencil icon) - only for non-archived, non-system-controlled tasks
- StatusDropdown for quick status changes
- Archive/Restore/Permanently Delete buttons based on archived state
- Archive badge for archived tasks

// TaskBoard (Ph18 updates)
- Infinite scroll per column via useInfiniteTasksQuery
- Search bar (Cmd+F) with server-side search via useTaskSearch
- Show archived toggle when archivedCount > 0
- Keyboard shortcuts: Cmd+N (create), Cmd+F (search), Escape (close search)
- Real-time updates via Tauri events: task:archived, task:restored, task:deleted

// TaskCard (Ph18 updates)
- Archived appearance: opacity-60, gray priority stripe, archive badge
- isDraggable logic: false for system-controlled states (executing, qa_*, pending_review, etc.)
- Context menu wrapper: right-click for quick actions
- Non-draggable visual: opacity-75, cursor-default, title tooltip

// Column (Ph18 updates)
- Infinite scroll with IntersectionObserver on sentinel element
- InlineTaskAdd on hover (draft/backlog only, not while dragging)
- Loading spinner at bottom when isFetchingNextPage
```

## Task Execution Experience (Ph19)
```typescript
// Types (types/task-step.ts)
type TaskStepStatus = 'pending' | 'in_progress' | 'completed' | 'skipped' | 'failed' | 'cancelled'
interface TaskStep {
  id: string, taskId: string, title: string, description: string | null,
  status: TaskStepStatus, sortOrder: number, dependsOn: string | null,
  createdBy: string, completionNote: string | null,
  createdAt: string, updatedAt: string, startedAt: string | null, completedAt: string | null
}
interface StepProgressSummary {
  taskId: string, total: number, completed: number, inProgress: number,
  pending: number, skipped: number, failed: number,
  currentStep: TaskStep | null, nextStep: TaskStep | null,
  percentComplete: number
}

// API (lib/tauri.ts)
api.steps.getByTask(taskId) → TaskStep[]
api.steps.create(taskId, data) → TaskStep
api.steps.update(stepId, data) → TaskStep
api.steps.delete(stepId) → void
api.steps.reorder(taskId, stepIds) → TaskStep[]
api.steps.getProgress(taskId) → StepProgressSummary
api.steps.start(stepId) → TaskStep
api.steps.complete(stepId, note?) → TaskStep
api.steps.skip(stepId, reason) → TaskStep
api.steps.fail(stepId, error) → TaskStep

// Hooks (hooks/)
useTaskSteps(taskId) → { data: TaskStep[], isLoading, isError }
  // Query key: stepKeys.byTask(taskId), staleTime: 30s
useStepProgress(taskId) → { data: StepProgressSummary, isLoading }
  // Query key: stepKeys.progress(taskId), staleTime: 5s
  // Auto-refetch every 5s if inProgress > 0
useStepMutations(taskId) → { create, update, delete, reorder }
  // All mutations invalidate stepKeys.byTask(taskId) and stepKeys.progress(taskId)
useStepEvents() → void
  // Listens for step:created, step:updated, step:deleted, steps:reordered
  // Invalidates queries on events
useTaskExecutionState(taskId) → TaskExecutionState
  // Combines task status and step progress
  // Returns: { isActive, duration, phase, stepProgress }
  // phase: 'idle' | 'executing' | 'qa' | 'review' | 'done'

// Components (components/tasks/)
StepProgressBar: { taskId, compact? } — Progress dots with status colors
  // completed: green, skipped: gray, failed: red, in_progress: orange+pulse, pending: border-gray
  // Shows "{completed+skipped}/{total}" if not compact
StepItem: { step, index, editable?, onUpdate?, onDelete? } — Single step with status icon
  // Status icons: Circle(pending), Loader2(in_progress), CheckCircle2(completed),
  //               MinusCircle(skipped), XCircle(failed)
  // Highlights in_progress with border-accent-primary, bg-accent-muted
  // Shows completion_note if exists
StepList: { taskId, editable? } — List of steps with mutations
  // Uses useTaskSteps + useStepMutations
  // EmptyState if no steps
TaskDetailPanel: { task, showContext?, showHistory? } — Reusable detail content
  // Extracted from TaskDetailModal
  // Shows: priority, title, category, status, description, TaskContextPanel, StepList, StateHistoryTimeline
TaskChatPanel: { taskId, contextType } — Embedded chat without resize/collapse
  // contextType: 'task' | 'task_execution' (determined by task status)
  // Shows context indicator header (e.g., "Worker Execution")
TaskFullView: { taskId, onClose } — Full-screen with split panels
  // 24px margin Raycast-style overlay
  // Header: back, title, priority, status, edit/archive buttons, close
  // Split layout: TaskDetailPanel (left) | TaskChatPanel (right)
  // Default 50/50 split, resizable with drag handle (min 360px each side)
  // Escape key closes
  // Stores panel width in localStorage

// uiStore (stores/uiStore.ts)
taskFullViewId: string | null
openTaskFullView: (taskId: string) => void
closeTaskFullView: () => void

// TaskCard (Ph19 updates)
- Execution state visuals:
  - executing: task-card-executing class, pulsing orange border, activity dots (3 dots with staggered bounce)
  - qa_*: pulsing border with QA icon
  - pending_review: amber border with Eye icon
  - revision_needed: task-card-attention class
- StepProgressBar in bottom (compact mode) when executing/qa/pending_review
- Duration badge with Clock icon: "2m 15s" format when executing
- Click behavior: opens TaskFullView for executing/qa/pending_review/revision_needed, modal otherwise

// TaskCreationForm (Ph19 updates)
- Steps editor: string[] state with add/remove/reorder (up/down arrows)
- Steps sent to create_task command
- Simple up/down buttons for reordering (no dnd-kit)

// TaskEditForm (Ph19 updates)
- StepList with editable={!isExecuting}
- Add step inline when not executing
- Changes saved via useStepMutations

// CSS (styles/globals.css)
@keyframes executing-pulse — box-shadow animation
@keyframes attention-pulse — opacity animation
.task-card-executing — pulsing orange border
.task-card-attention — attention animation

// Events (real-time)
step:created → { task_id, step_id }
step:updated → { task_id, step_id }
step:deleted → { task_id, step_id }
steps:reordered → { task_id, step_ids }
```

## TS Config (strict)
```json
{ "strict":true, "noUncheckedIndexedAccess":true, "noImplicitReturns":true, "exactOptionalPropertyTypes":true, "verbatimModuleSyntax":true }
```
Conditional props pattern for exactOptionalPropertyTypes:
```typescript
const props = { required: val, ...(optional !== undefined && { optional }) }
```

## CSS Variables (globals.css)
```css
:root { --bg-base:hsl(0 0% 6%); --accent-primary:hsl(14 100% 60%); ... }
@theme inline { --color-bg-base:var(--bg-base); --color-accent-primary:var(--accent-primary); }
```
Tokens: bg-base|surface|elevated|hover | text-primary|secondary|muted | accent-primary|secondary
        status-success|warning|error|info | border-subtle|default|focus

## Tailwind v4 Config
❌ NO tailwind.config.js (ignored)
❌ NO tailwindcss-animate (deprecated)
✅ `@tailwindcss/vite` in vite.config.ts
✅ `@theme inline` in globals.css
✅ `"config":""` in components.json

## Anti-AI-Slop
NO purple gradients | NO Inter font | NO generic icon grids | Warm orange #ff6b35

## Testing
```bash
pnpm test        # watch
pnpm test:run    # once
pnpm test:coverage
pnpm typecheck
```
setup.ts mocks: `@tauri-apps/api/core` (invoke) | `@tauri-apps/api/event` (listen, emit)
Store reset: `useTaskStore.setState({ tasks:{}, selectedTaskId:null })`

## Scripts
```bash
pnpm dev      # :1420
pnpm build
pnpm preview
pnpm tauri
```

## Adding Features
1. Types: Zod schema in types/, export from types/index.ts
2. API: wrapper in lib/tauri.ts or lib/api/
3. Store: Zustand+immer in stores/
4. Hook: TanStack Query in hooks/
5. Component: with co-located test
6. **Tests FIRST (TDD mandatory)**

## Code Quality Rules

### Continuous Improvement (MANDATORY)
When modifying existing files during task execution:
1. **Review the modified file** against the standards below
2. **Refactor issues found** as part of the same task (not a separate PR)
3. **Scope appropriately** — fix what you touch, don't refactor unrelated code

This ensures code quality improves incrementally with each task.

### File Size (STRICT)
| File Type | Max Lines | Action at Threshold |
|-----------|-----------|---------------------|
| Component (.tsx) | 500 | Extract sub-components or hooks |
| Custom Hook | 300 | Split into focused pieces |
| Presentational Component | 200 | Pure display only |

**Refactoring trigger at 400 lines** — plan extraction before hitting limit.

### Extraction Triggers
| Signal | Action |
|--------|--------|
| >3 useState/useCallback/useMemo in component | Extract custom hook |
| >4 props on component | Consider composition pattern |
| >3 conditional render branches | Extract sub-components |
| JSX nesting >3 levels | Extract middle layer component |
| Handler function >10 lines | Extract to hook or helper |
| Component >400 lines | Mandatory extraction before merge |

### Single Responsibility
A component does **ONE** of:
- **Display UI** (presentational — props only, no hooks)
- **Manage State** (container — holds useState/store access)
- **Coordinate** (composition — orchestrates children)

NOT multiple. Avoid god components.

### Composition Over Props
```tsx
// ❌ WRONG: Prop explosion
<TaskModal task={task} showChat showHistory showContext chatContext={ctx} />

// ✅ CORRECT: Composition
<TaskModal task={task}>
  <TaskModal.Context />
  <TaskModal.History />
  <TaskModal.Chat context={ctx} />
</TaskModal>
```

### Custom Hooks Pattern
Extract logic from components into hooks:
```tsx
// ❌ WRONG: Logic mixed with rendering
function ChatPanel() {
  const [messages, setMessages] = useState([]);
  const [isLoading, setIsLoading] = useState(false);
  // 50+ lines of message handling, event listeners, queue logic...
  return <div>...</div>;
}

// ✅ CORRECT: Hook extracts logic
function ChatPanel() {
  const { messages, isLoading, sendMessage } = useChatMessages(contextId);
  return <div>...</div>;  // Just rendering
}
```

### Event Handlers
- Always use `useCallback` for handlers passed to children
- Never inline complex handlers (>3 lines) in JSX
- Extract to custom hook if handler needs multiple state updates

```tsx
// ✅ CORRECT: Memoized, passed to child
const handleSubmit = useCallback(() => {
  validate();
  submit();
}, [validate, submit]);
<Form onSubmit={handleSubmit} />

// ❌ WRONG: Inline complex handler
<Form onSubmit={() => {
  validate();
  submit();
  clearForm();
  showToast();
}} />
```

### Documentation (MANDATORY for exports)
```tsx
/**
 * TaskCard - Draggable card for Kanban board
 *
 * Shows task summary with status badge, priority stripe.
 * Pulsing border animation when task is executing.
 *
 * @prop task - Task data to display
 * @prop onSelect - Called when card is clicked
 * @prop isDraggable - Whether drag is enabled (false during execution)
 */
export function TaskCard({ task, onSelect, isDraggable }: TaskCardProps) {}
```

### Import Organization (strict order)
```typescript
// 1. React & framework
import { useState, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";

// 2. Third-party (alphabetical)
import { useQuery } from "@tanstack/react-query";
import { create } from "zustand";

// 3. Internal utilities (@/)
import { api } from "@/lib/tauri";
import { cn } from "@/lib/utils";

// 4. Stores
import { useChatStore } from "@/stores/chatStore";

// 5. Types (use `import type`)
import type { Task } from "@/types/task";

// 6. Components (general → specific)
import { Button } from "@/components/ui/button";
import { TaskCard } from "./TaskCard";

// 7. Local files (relative, last)
import { useLocalHook } from "./hooks";
```

### Files Needing Refactoring (Priority)
| File | Lines | Refactor Strategy |
|------|-------|-------------------|
| `ExtensibilityView.tsx` | 1,239 | Split into WorkflowsTab, ArtifactsTab, ResearchTab, MethodologiesTab |
| `IdeationView.tsx` | 1,198 | Extract ProposalPanel, SessionSelector, PlanDisplayContainer |
| `ChatPanel.tsx` | 1,044 | Extract ResizeHandler, MessageList, QueueManager hooks |
| `IntegratedChatPanel.tsx` | 1,021 | Extract handlers to useIntegratedChat hook |
| `App.tsx` | 845 | Extract NavigationSidebar, ModalRegistry, ContextProviders |
| `DiffViewer.tsx` | 966 | Split rendering, highlighting, caching logic |
| `SettingsView.tsx` | 827 | Extract each settings panel to separate component |
| `TaskDetailModal.tsx` | 678 | Split Header, Content, Footer regions |

### Pre-Commit Quality Check
```bash
# Add to your workflow before committing:
npm run lint
npm run typecheck

# Check no component exceeds 500 lines (warning)
find src/components -name "*.tsx" -exec wc -l {} + | awk '$1 > 500 {print "⚠️  OVER 500:", $2, "("$1" lines)"}'
```
