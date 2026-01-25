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
│   └── chat.ts             # Chat message API
│
├── components/             # React components
│   ├── Chat/               # Chat panel (ChatPanel, ChatMessage, ChatInput)
│   ├── execution/          # Execution control (ExecutionControlBar)
│   ├── Ideation/           # Ideation view (ProposalCard, ProposalList, etc.)
│   ├── modals/             # Modal dialogs (AskUserQuestionModal)
│   ├── qa/                 # QA components (TaskQABadge, QASettingsPanel)
│   ├── reviews/            # Review system (ReviewsPanel, ReviewCard)
│   ├── tasks/              # Task components (TaskBoard/, TaskDetailView)
│   │   └── TaskBoard/      # Kanban board with dnd-kit integration
│   └── ui/                 # Shared UI components (StatusBadge)
│
├── hooks/                  # Custom React hooks
│   ├── use*.ts             # TanStack Query hooks (useTasks, useProjects, etc.)
│   ├── useEvents.ts        # Tauri event listeners
│   └── use*Mutation.ts     # Mutation hooks for data updates
│
├── lib/                    # Utility libraries
│   ├── api/                # Additional Tauri API wrappers (workflows, artifacts, etc.)
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
│   ├── ideationStore.ts    # Ideation session state
│   ├── proposalStore.ts    # Task proposal state
│   ├── chatStore.ts        # Chat panel state
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
│   ├── task.ts             # Task type and schemas
│   ├── project.ts          # Project type and schemas
│   ├── status.ts           # InternalStatus enum (14 statuses)
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
