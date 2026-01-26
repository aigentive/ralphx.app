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
│  ├─ Task/              # TaskBoard/ (dnd-kit), TaskDetailPanel, TaskContextPanel (Ph17)
│  └─ ui/                # StatusBadge, shadcn components
├─ hooks/                # useTasks, useChat, useEvents, use*Mutation
├─ lib/
│  ├─ api/               # Additional wrappers (workflows, artifacts, task-context)
│  ├─ tauri.ts           # typedInvoke with Zod validation
│  ├─ queryClient.ts     # TanStack config
│  └─ validation.ts, formatters.ts
├─ providers/            # EventProvider (global Tauri events)
├─ stores/               # *Store.ts (Zustand+immer): task, project, ui, ideation,
│                        # proposal, chat, qa, review
├─ styles/globals.css    # @theme inline, design tokens
├─ test/                 # setup.ts (mocks Tauri), store-utils.ts
├─ types/                # Zod schemas: task, project, status (14 states),
│                        # chat-conversation, permission, ideation, ideation-config,
│                        # task-context (Ph17)
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
