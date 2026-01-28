# src/CLAUDE.md — Frontend

## Stack
React 19.1 | TS 5.8 | Zustand 5.0+immer | TanStack Query 5.90 | Tailwind 4.1 | Zod 4.3
dnd-kit 6.3 | Vite 7.0 | Vitest 4.0 | Testing Library 16.3 | Tauri API 2.x

## Key Directories
```
src/
├─ api/           # Tauri wrappers
├─ components/    # UI (Chat/, Task/, Ideation/, ui/)
├─ hooks/         # TanStack Query + custom
├─ lib/           # tauri.ts (typedInvoke), queryClient.ts
├─ stores/        # Zustand+immer
├─ styles/        # globals.css (@theme inline)
├─ test/          # setup.ts (Tauri mocks)
└─ types/         # Zod schemas
```

## Patterns

### Zustand + Immer
```typescript
const useTaskStore = create<State & Actions>()(immer((set) => ({
  tasks: {},  // Record<id, Task> for O(1)
  updateTask: (id, changes) => set(s => { Object.assign(s.tasks[id], changes) })
})));
export const selectByStatus = (status) => (s) => Object.values(s.tasks).filter(...)
```

### TanStack Query Keys
```typescript
const taskKeys = { all:["tasks"], list:(pid)=>[...taskKeys.all,"list",pid], detail:(tid)=>[...taskKeys.all,"detail",tid] }
```

### Typed Tauri + Zod
```typescript
async function typedInvoke<T>(cmd, args, schema: z.ZodType<T>): Promise<T> {
  return schema.parse(await invoke(cmd, args))
}
```

### Types via Zod
```typescript
const TaskSchema = z.object({ id:z.string().min(1), ... })
type Task = z.infer<typeof TaskSchema>
```

### Component Organization
```
Component/
├─ index.tsx, Component.tsx, Component.test.tsx
├─ ChildComponent.tsx, hooks.ts
└─ *.test.tsx (co-located)
```

### Event-Driven Updates
EventProvider wraps app with hooks: `useTaskEvents()`, `useSupervisorAlerts()`, `useReviewEvents()`

### Path Aliases
`import { Task } from "@/types/task"` — configured in vite.config.ts + tsconfig.json

## Rules

### Tauri Param Conventions (CRITICAL)
| Context | JS Side | Rust Side |
|---------|---------|-----------|
| Direct params | `{ contextType, contextId }` (camelCase) | `context_type, context_id` |
| Struct fields | `{ input: { context_type, context_id } }` (snake_case) | serde exact-match |

Add `#[serde(rename_all="camelCase")]` to Rust struct for camelCase in JS struct fields.

### TS Config (strict)
```json
{ "strict":true, "noUncheckedIndexedAccess":true, "noImplicitReturns":true, "exactOptionalPropertyTypes":true }
```
Conditional props: `{ required: val, ...(optional !== undefined && { optional }) }`

### Tailwind v4 Config
- NO tailwind.config.js (ignored)
- NO tailwindcss-animate (deprecated)
- `@tailwindcss/vite` in vite.config.ts
- `@theme inline` in globals.css
- `"config":""` in components.json

### CSS Variables
```css
:root { --bg-base:hsl(0 0% 6%); --accent-primary:hsl(14 100% 60%); }
@theme inline { --color-bg-base:var(--bg-base); --color-accent-primary:var(--accent-primary); }
```
Tokens: bg-base|surface|elevated | text-primary|secondary|muted | accent-primary|secondary

### Anti-AI-Slop
NO purple gradients | NO Inter font | Warm orange #ff6b35

## Code Quality

### Proactive Quality Improvement (ENFORCED BY HOOK)
Every code task requires `refactor:` commit. See `.claude/rules/quality-improvement.md` for targets and process.

### File Size Limits
| File Type | Max Lines | Action |
|-----------|-----------|--------|
| Component | 500 | Extract sub-components/hooks |
| Custom Hook | 300 | Split into focused pieces |
| Presentational | 200 | Pure display only |

Refactoring trigger at 400 lines.

### Extraction Triggers
- >3 useState/useCallback in component → extract hook
- >4 props → composition pattern
- >3 conditional branches → extract sub-components
- Handler >10 lines → extract to hook

### Single Responsibility
Component does ONE of: Display UI | Manage State | Coordinate children

### Document Patterns Inline
When introducing a new architectural pattern, add a one-liner here. Pattern name + rule only.
Example: "View Registry Pattern: state-specific views registered in TASK_DETAIL_VIEWS map"

### Composition Over Props
```tsx
// ❌ <TaskModal task={task} showChat showHistory showContext />
// ✅ <TaskModal task={task}><TaskModal.Chat /><TaskModal.History /></TaskModal>
```

### Import Order
1. React & framework
2. Third-party (alphabetical)
3. Internal (@/)
4. Stores
5. Types (`import type`)
6. Components (general → specific)
7. Local (relative)

## Commands
```bash
npm test           # watch mode
npm run test:run   # single run
npm run typecheck  # TS check
npm run lint       # ESLint
```
Note: Dev server via `npm run tauri dev` from project root (user manages manually).

## Task Management (MANDATORY)
Use TaskCreate/TaskUpdate/TaskList for complex work. See `.claude/rules/task-management.md`

## Adding Features
1. Types: Zod schema in types/
2. API: wrapper in lib/tauri.ts
3. Store: Zustand+immer
4. Hook: TanStack Query
5. Component: with co-located test
6. **Tests FIRST (TDD mandatory)**
