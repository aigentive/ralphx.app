# src/CLAUDE.md — Frontend

Quality standards: @../.claude/rules/code-quality-standards.md
Task detail views: @../.claude/rules/task-detail-views.md

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

### API Layer Patterns
See @.claude/rules/api-layer.md for Tauri conventions, schemas, transforms, and mocking.

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

### Multi-Stream Workflow
Quality work is now split into dedicated streams. See `.claude/rules/stream-*.md`:
- **features**: PRD tasks + P0 gap fixes
- **refactor**: P1 large file splits (>500 LOC)
- **polish**: P2/P3 cleanup, lint, type fixes

**Targets:** `any` types, naming, error handling, dead code, repeated logic, lint

### File Size Limits
**See:** `.claude/rules/code-quality-standards.md` (single source of truth)

Quick reference: Component 500 max (refactor at 400), Hook 300 max, Presentational 200 max.

### Single Responsibility
Component does ONE of: Display UI | Manage State | Coordinate children

### Document Patterns Inline
When introducing a new architectural pattern, add a one-liner here. Pattern name + rule only.
Example: "View Registry Pattern" — see @../.claude/rules/task-detail-views.md

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
