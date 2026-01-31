# Web Target for RalphX - Browser Testing with Mocked Backend

## Goal

Enable browser automation (Playwright) to test UI behavior, styling, and component rendering by running the React app standalone with mocked Tauri backend.

**Current state:** `npm run dev` (or Vite dev server) works, but crashes on any Tauri `invoke()` call.

**Target state:** Browser loads app, all UI renders, mock data populates views, automation can interact with components.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│  Browser (Playwright)                                       │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  React App                                            │  │
│  │  ┌─────────────┐    ┌─────────────┐                   │  │
│  │  │ Components  │───▶│ TanStack    │                   │  │
│  │  │             │    │ Query Hooks │                   │  │
│  │  └─────────────┘    └──────┬──────┘                   │  │
│  │                            │                          │  │
│  │                     ┌──────▼──────┐                   │  │
│  │                     │ src/api/*   │                   │  │
│  │                     └──────┬──────┘                   │  │
│  │                            │                          │  │
│  │              ┌─────────────┴─────────────┐            │  │
│  │              │                           │            │  │
│  │       ┌──────▼──────┐           ┌───────▼───────┐     │  │
│  │       │ Mock Layer  │           │ Real Tauri    │     │  │
│  │       │ (web mode)  │           │ (native mode) │     │  │
│  │       └─────────────┘           └───────────────┘     │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

---

## Current Architecture (Favorable)

| Area | Status | Web Impact |
|------|--------|------------|
| API calls | Centralized in `src/api/*.ts` | Easy to mock |
| Data fetching | TanStack Query hooks | No changes needed |
| State management | Zustand stores | Independent of backend |
| Zod schemas | Pure TypeScript | No changes needed |
| Test mocks | Exist in `test/setup.ts` | Foundation to build on |

### 3. Coupling Points to Address

| Component | Count | Files | Effort |
|-----------|-------|-------|--------|
| `invoke()` calls | ~30 | `src/api/*.ts` | Mock functions |
| `listen()` events | ~15 | `src/hooks/useEvents*.ts` | **Needs abstraction** |
| Tauri plugins | ~8 | Various | Mock/remove |

---

## Recommended Approach: Mock Layer + Event Abstraction

### Phase 1: API Mock Layer

Create parallel mock implementations:

```
src/
├── api/
│   ├── tasks.ts          # Real Tauri calls
│   ├── projects.ts
│   └── ...
├── api-mock/              # NEW: Mock implementations
│   ├── tasks.ts          # Returns mock data
│   ├── projects.ts
│   └── ...
└── lib/
    └── tauri.ts          # Switch via env var
```

**Switch mechanism:**
```typescript
// src/lib/tauri.ts
const useMocks = import.meta.env.VITE_USE_MOCKS === 'true';
export const api = useMocks ? mockApi : realApi;
```

### Phase 2: Event Abstraction

Abstract Tauri events behind a context:

```typescript
// src/providers/EventProvider.tsx
interface EventContext {
  subscribe: (event: string, handler: (payload: unknown) => void) => () => void;
  emit: (event: string, payload: unknown) => void; // For mock mode
}

// Real implementation: uses Tauri listen()
// Mock implementation: in-memory event bus
```

**Files to update:** 15 hooks in `src/hooks/useEvents*.ts`

### Phase 3: Tauri Plugin Mocks

Mock or no-op these imports:
- `@tauri-apps/plugin-dialog`
- `@tauri-apps/plugin-fs`
- `@tauri-apps/plugin-process`
- etc.

---

## Build Configuration

### Vite Configuration

```typescript
// vite.config.ts
export default defineConfig(({ mode }) => ({
  define: {
    'import.meta.env.VITE_USE_MOCKS': mode === 'web' ? 'true' : 'false',
  },
  // ... rest of config
}));
```

### NPM Scripts

```json
{
  "scripts": {
    "dev": "vite",
    "dev:web": "vite --mode web",
    "build:web": "vite build --mode web --outDir dist-web"
  }
}
```

---

## Mock Data Strategy

**Existing foundation:** `src/test/mock-data.ts` (240 lines of factories)

```typescript
// Already have:
createMockTask(), createMockTasks()
createMockProject(), createMockProjects()
createMockExecution()
// etc.
```

**For web target:** Extend with:
- Persistent mock state (localStorage or in-memory)
- CRUD operations that update mock state
- Mock event emission on state changes

---

## Browser Automation Testing Benefits

1. **Visual regression testing** - Screenshot comparisons
2. **Component interaction testing** - Click, type, navigate
3. **Styling verification** - Check design system compliance
4. **Accessibility testing** - Screen reader, keyboard nav
5. **Fast feedback loop** - No native app boot time

---

## Maintenance Model (Low Overhead)

**Key insight:** RalphX already has schema-driven types and mock factories. Mock drift is prevented by TypeScript.

```
Backend Change Flow:
┌────────────┐    ┌─────────────┐    ┌──────────────┐    ┌───────────┐
│ Rust struct│───▶│ Zod Schema  │───▶│ Mock Factory │───▶│ Mock API  │
│ changes    │    │ must update │    │ must update  │    │ auto-works│
└────────────┘    └─────────────┘    └──────────────┘    └───────────┘
                        │                   │
                        ▼                   ▼
                   TypeScript          TypeScript
                   error if            error if
                   mismatch            mismatch
```

**Why agents don't need separate mock updates:**
- Mock API returns `createMockTask()` output
- `createMockTask()` returns `Task` type
- If backend adds field → schema adds field → factory must add field → type error until fixed
- Once factory is fixed, mock API automatically includes new field

**Actual maintenance work:**
- Schema changes: Already required for frontend to work with backend
- Factory changes: ~5 lines per new field, caught by TypeScript
- Mock API logic: Rarely changes (just calls factories)

---

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Event timing | Add configurable delays in mock emitter |
| Mock drift | **Prevented by shared types** - TypeScript catches mismatches |
| Missing edge cases | Add specific scenarios to factories as needed |

---

## Implementation Tasks

## Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Mock fidelity | **Basic - Read-only** | Visual testing needs rendered UI, not working CRUD. Add mutations later if needed. |
| Persistence | **In-memory** | Predictable state for tests. Resets to seed data on reload. |

**"Basic - Read-only" means:**
- `list*` / `get*` → Return factory-generated mock data ✓
- `create*` / `update*` / `delete*` → Return success (no-op), don't update state
- UI renders data, forms submit without error, but state doesn't persist
- **Sufficient for:** Visual regression tests, styling checks, layout verification
- **Add later if needed:** Full CRUD with state updates

---

### Task 1: Mock API Module (Core) (BLOCKING)

**Dependencies:** None
**Atomic Commit:** `feat(api-mock): create mock API module for web target`

Create `src/api-mock/index.ts` that mirrors `src/api/` interface:

```typescript
// src/api-mock/index.ts
import { createMockTask, createMockProject } from '@/test/mock-data';

const mockTasks = new Map<string, Task>();
const mockProjects = new Map<string, Project>();

// Initialize with seed data
seedMockData();

export const mockApi = {
  tasks: {
    list: async ({ projectId }) => ({
      tasks: Array.from(mockTasks.values()).filter(t => t.projectId === projectId)
    }),
    get: async ({ id }) => mockTasks.get(id),
    create: async (input) => { /* add to mockTasks, return created */ },
    update: async (id, changes) => { /* update mockTasks */ },
    // ... etc
  },
  projects: { /* similar */ },
  // ... other domains
};
```

**Files:** `src/api-mock/index.ts`, `src/api-mock/tasks.ts`, `src/api-mock/projects.ts`, etc.

### Task 2: Environment Switch (BLOCKING)

**Dependencies:** Task 1
**Atomic Commit:** `feat(lib): add Tauri detection and mock API switching`

Update `src/lib/tauri.ts` to conditionally use mocks:

```typescript
// src/lib/tauri.ts
import { realApi } from './tauri-real';
import { mockApi } from '@/api-mock';

const isWebMode = !window.__TAURI_INTERNALS__;

export const api = isWebMode ? mockApi : realApi;
```

**Note:** `window.__TAURI_INTERNALS__` exists only in Tauri WebView, not in browser.

**Files:** `src/lib/tauri.ts`

### Task 3: Event Provider (BLOCKING)

**Dependencies:** None (parallel with Tasks 1-2)
**Atomic Commit:** `feat(providers): create EventProvider for Tauri event abstraction`

Create context to abstract Tauri event system:

```typescript
// src/providers/EventProvider.tsx
const EventContext = createContext<EventBus | null>(null);

export function EventProvider({ children }) {
  const bus = useMemo(() =>
    window.__TAURI_INTERNALS__
      ? new TauriEventBus()
      : new MockEventBus(),
    []
  );
  return <EventContext.Provider value={bus}>{children}</EventContext.Provider>;
}
```

**Files:** `src/providers/EventProvider.tsx`, `src/lib/event-bus.ts`

### Task 4: Update Event Hooks

**Dependencies:** Task 3
**Atomic Commit:** `refactor(hooks): migrate event hooks to EventProvider`

Refactor `src/hooks/useEvents*.ts` to use EventProvider:

```typescript
// Before:
import { listen } from '@tauri-apps/api/event';

// After:
import { useEventBus } from '@/providers/EventProvider';
const bus = useEventBus();
bus.subscribe('task:event', handler);
```

**Compilation Note:** This task modifies all ~15 event hooks in a single compilation unit. The EventProvider must exist (Task 3) before this task can proceed. All hooks must be updated together to maintain a compilable state.

**Files:** All `src/hooks/useEvents*.ts` files (~15)

### Task 5: Tauri Plugin Mocks

**Dependencies:** Task 2
**Atomic Commit:** `feat(mocks): add Tauri plugin mocks for web mode`

Mock or no-op Tauri plugin imports via Vite aliases in web mode:

```typescript
// vite.config.ts (web mode)
resolve: {
  alias: {
    '@tauri-apps/plugin-dialog': './src/mocks/tauri-plugins.ts',
    '@tauri-apps/plugin-fs': './src/mocks/tauri-plugins.ts',
    // etc.
  }
}
```

**Files:** `src/mocks/tauri-plugins.ts`, `vite.config.ts`

### Task 6: NPM Scripts and Vite Config

**Dependencies:** Tasks 1, 2, 5
**Atomic Commit:** `chore(build): add dev:web script and web mode Vite config`

```json
{
  "scripts": {
    "dev": "vite",
    "dev:web": "VITE_MODE=web vite",
    "test:visual": "npm run dev:web & playwright test"
  }
}
```

**Files:** `package.json`, `vite.config.ts`

### Task 7: Playwright Setup and Initial Test

**Dependencies:** Task 6
**Atomic Commit:** `test(visual): add Playwright setup and kanban board test`

```typescript
// tests/visual/kanban.spec.ts
import { test, expect } from '@playwright/test';

test('kanban board renders with mock tasks', async ({ page }) => {
  await page.goto('http://localhost:5173');

  // Wait for mock data to load
  await page.waitForSelector('[data-testid="task-card"]');

  // Verify task cards visible
  const tasks = await page.locator('[data-testid="task-card"]').count();
  expect(tasks).toBeGreaterThan(0);

  // Screenshot for visual regression
  await expect(page).toHaveScreenshot('kanban-board.png');
});
```

**Files:** `tests/visual/kanban.spec.ts`, `playwright.config.ts`

---

## Task Dependency Graph

```
Task 1 (Mock API) ─────────┬──────────────────────────────────────────┐
                           │                                          │
                           ▼                                          │
Task 2 (Env Switch) ───────┼──────────────────────────────────────────┤
                           │                                          │
                           ▼                                          │
Task 5 (Plugin Mocks) ─────┤                                          │
                           │                                          │
                           ▼                                          │
Task 6 (NPM Scripts) ──────┤                                          │
                           │                                          │
                           ▼                                          │
Task 7 (Playwright) ◀──────┘                                          │
                                                                      │
Task 3 (Event Provider) ──────────────────────────────────────────────┤
                           │                                          │
                           ▼                                          │
Task 4 (Update Hooks) ◀───────────────────────────────────────────────┘
```

**Parallel execution possible:**
- Tasks 1, 3 can run in parallel (no dependencies)
- Tasks 2, 4 must wait for their respective dependencies
- Task 7 is the final integration point

---

## Implementation Order

1. [ ] Create `src/api-mock/` with mock implementations (Task 1)
2. [ ] Add `__TAURI_INTERNALS__` detection in `src/lib/tauri.ts` (Task 2)
3. [ ] Create `EventProvider` context (Task 3 - parallel with 1-2)
4. [ ] Refactor event hooks to use provider (~15 files) (Task 4)
5. [ ] Mock Tauri plugin imports (dialog, fs, etc.) (Task 5)
6. [ ] Add `dev:web` npm script (Task 6)
7. [ ] Write initial Playwright test (Task 7)
8. [ ] Verify end-to-end in browser

---

## Verification

After implementation:
1. `npm run dev` → Opens in browser (not Tauri)
2. Console shows no Tauri-related errors
3. Kanban board renders with mock tasks
4. CRUD operations work (update mock state)
5. Playwright tests pass

---

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)

### Compilation Unit Analysis

All tasks in this plan have been validated as complete compilation units:

| Task | Type | Compiles Independently? |
|------|------|-------------------------|
| Task 1 | Additive (new files) | ✅ Yes - new module, not imported yet |
| Task 2 | Additive (new code path) | ✅ Yes - conditional import, graceful fallback |
| Task 3 | Additive (new provider) | ✅ Yes - new component, not used yet |
| Task 4 | Modification (all hooks) | ✅ Yes - all hooks updated together |
| Task 5 | Additive (mock files) | ✅ Yes - Vite alias, new files |
| Task 6 | Config (scripts) | ✅ Yes - build config only |
| Task 7 | Additive (test files) | ✅ Yes - new test files |

**Note:** Task 4 must update ALL event hooks in a single task to maintain compilation. Splitting hook updates across tasks would create broken intermediate states.
