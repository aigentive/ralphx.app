# RalphX - Phase 55: Web Target for Browser Testing

## Overview

Enable browser automation (Playwright) to test UI behavior, styling, and component rendering by running the React app standalone with a mocked Tauri backend. This phase creates the infrastructure to run the frontend in a pure browser environment without the native Tauri shell.

**Current state:** `npm run dev` works but crashes on any Tauri `invoke()` call.

**Target state:** Browser loads app, all UI renders, mock data populates views, Playwright automation can interact with components.

**Reference Plan:**
- `specs/plans/web_target_browser_testing.md` - Detailed implementation plan with architecture diagrams and mock data strategy

## Goals

1. Create a mock API layer that mirrors the real Tauri API interface
2. Abstract Tauri event system behind a provider for mock/real switching
3. Enable `npm run dev:web` to run frontend in pure browser mode
4. Set up Playwright for visual regression testing

## Dependencies

### Phase 54 (Blocked Reason Feature) - None Required

This phase is self-contained and adds new functionality without modifying existing features.

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/web_target_browser_testing.md`
2. Understand the architecture and mock/real switching mechanism
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Write functional tests where appropriate
4. Run linters for modified code only (frontend: `npm run lint && npm run typecheck`)
5. Commit with descriptive message

---

## Git Workflow (Parallel Agent Coordination)

**Before each commit, follow the commit lock protocol at `.claude/rules/commit-lock.md`**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls

**Commit message conventions** (see `.claude/rules/git-workflow.md`):
- Features stream: `feat:` / `fix:` / `docs:`
- Refactor stream: `refactor(scope):`

**Task Execution Order:**
- Tasks with `"blockedBy": []` can start immediately
- Before starting a task, check `blockedBy` - all listed tasks must have `"passes": true`
- Execute tasks in ID order when dependencies are satisfied

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/web_target_browser_testing.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "frontend",
    "description": "Create mock API module that mirrors src/api/ interface",
    "plan_section": "Task 1: Mock API Module (Core)",
    "blocking": [2],
    "blockedBy": [],
    "atomic_commit": "feat(api-mock): create mock API module for web target",
    "steps": [
      "Read specs/plans/web_target_browser_testing.md section 'Task 1: Mock API Module'",
      "Create src/api-mock/ directory structure",
      "Implement mock API functions using existing createMock* factories from src/test/mock-data.ts",
      "Create index.ts that exports mockApi object matching real API interface",
      "Ensure all list/get operations return factory-generated mock data",
      "Create/update/delete operations should return success (no-op for read-only mode)",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(api-mock): create mock API module for web target"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "frontend",
    "description": "Add Tauri detection and conditional mock API switching",
    "plan_section": "Task 2: Environment Switch",
    "blocking": [5, 6],
    "blockedBy": [1],
    "atomic_commit": "feat(lib): add Tauri detection and mock API switching",
    "steps": [
      "Read specs/plans/web_target_browser_testing.md section 'Task 2: Environment Switch'",
      "Create src/lib/tauri-detection.ts with isWebMode check using window.__TAURI_INTERNALS__",
      "Update API consumers to use conditional import based on isWebMode",
      "Ensure graceful fallback when Tauri is not available",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(lib): add Tauri detection and mock API switching"
    ],
    "passes": false
  },
  {
    "id": 3,
    "category": "frontend",
    "description": "Create EventProvider for Tauri event abstraction",
    "plan_section": "Task 3: Event Provider",
    "blocking": [4],
    "blockedBy": [],
    "atomic_commit": "feat(providers): create EventProvider for Tauri event abstraction",
    "steps": [
      "Read specs/plans/web_target_browser_testing.md section 'Task 3: Event Provider'",
      "Create src/lib/event-bus.ts with TauriEventBus and MockEventBus classes",
      "Create src/providers/EventProvider.tsx with context and useEventBus hook",
      "TauriEventBus: wraps real Tauri listen()/emit()",
      "MockEventBus: in-memory event emitter for browser mode",
      "Provider automatically selects bus based on window.__TAURI_INTERNALS__",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(providers): create EventProvider for Tauri event abstraction"
    ],
    "passes": false
  },
  {
    "id": 4,
    "category": "frontend",
    "description": "Migrate all event hooks to use EventProvider",
    "plan_section": "Task 4: Update Event Hooks",
    "blocking": [],
    "blockedBy": [3],
    "atomic_commit": "refactor(hooks): migrate event hooks to EventProvider",
    "steps": [
      "Read specs/plans/web_target_browser_testing.md section 'Task 4: Update Event Hooks'",
      "Identify all useEvents*.ts hooks in src/hooks/",
      "Replace direct @tauri-apps/api/event imports with useEventBus hook",
      "Update subscription pattern: bus.subscribe(event, handler)",
      "CRITICAL: Update ALL event hooks in this single task to maintain compilation",
      "Add EventProvider to App.tsx provider tree",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(hooks): migrate event hooks to EventProvider"
    ],
    "passes": false
  },
  {
    "id": 5,
    "category": "frontend",
    "description": "Add Tauri plugin mocks for web mode via Vite aliases",
    "plan_section": "Task 5: Tauri Plugin Mocks",
    "blocking": [6],
    "blockedBy": [2],
    "atomic_commit": "feat(mocks): add Tauri plugin mocks for web mode",
    "steps": [
      "Read specs/plans/web_target_browser_testing.md section 'Task 5: Tauri Plugin Mocks'",
      "Create src/mocks/tauri-plugins.ts with no-op implementations",
      "Mock @tauri-apps/plugin-dialog (open, save, message, confirm)",
      "Mock @tauri-apps/plugin-fs (if used)",
      "Mock @tauri-apps/plugin-process (if used)",
      "Update vite.config.ts to use conditional aliases in web mode",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(mocks): add Tauri plugin mocks for web mode"
    ],
    "passes": false
  },
  {
    "id": 6,
    "category": "frontend",
    "description": "Add dev:web npm script and Vite web mode configuration",
    "plan_section": "Task 6: NPM Scripts and Vite Config",
    "blocking": [7],
    "blockedBy": [2, 5],
    "atomic_commit": "chore(build): add dev:web script and web mode Vite config",
    "steps": [
      "Read specs/plans/web_target_browser_testing.md section 'Task 6: NPM Scripts and Vite Config'",
      "Add 'dev:web': 'vite --mode web' to package.json scripts",
      "Add 'build:web': 'vite build --mode web --outDir dist-web' to package.json",
      "Update vite.config.ts to handle mode === 'web' for mock aliases",
      "Test that npm run dev:web starts without Tauri errors",
      "Run npm run lint && npm run typecheck",
      "Commit: chore(build): add dev:web script and web mode Vite config"
    ],
    "passes": false
  },
  {
    "id": 7,
    "category": "frontend",
    "description": "Set up Playwright and create initial visual regression test",
    "plan_section": "Task 7: Playwright Setup and Initial Test",
    "blocking": [],
    "blockedBy": [6],
    "atomic_commit": "test(visual): add Playwright setup and kanban board test",
    "steps": [
      "Read specs/plans/web_target_browser_testing.md section 'Task 7: Playwright Setup and Initial Test'",
      "Install Playwright: npm install -D @playwright/test",
      "Create playwright.config.ts with webServer configuration for dev:web",
      "Create tests/visual/ directory",
      "Create tests/visual/kanban.spec.ts with initial test",
      "Test should verify task cards render with mock data",
      "Add screenshot comparison for visual regression",
      "Run npm run lint && npm run typecheck",
      "Commit: test(visual): add Playwright setup and kanban board test"
    ],
    "passes": false
  }
]
```

**Task field definitions:**
- `id`: Sequential integer starting at 1
- `blocking`: Task IDs that cannot start until THIS task completes
- `blockedBy`: Task IDs that must complete before THIS task can start (inverse of blocking)
- `atomic_commit`: Commit message for this task

---

## Key Architecture Decisions

| Decision | Rationale |
|----------|-----------|
| **Mock layer parallel to real API** | Maintains type safety - mockApi and realApi share same interface |
| **__TAURI_INTERNALS__ detection** | Native browser check without build-time configuration |
| **EventProvider abstraction** | Single point of control for event system, easy mock/real switching |
| **Read-only mock fidelity** | Sufficient for visual testing, avoids complex state management |
| **Vite mode-based switching** | Clean separation of web vs native builds |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Frontend - Run `npm run test`
- [ ] Mock API returns valid data for all list/get operations
- [ ] EventProvider correctly selects mock bus in browser mode
- [ ] No TypeScript errors in mock modules

### Build Verification (run only for modified code)
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`npm run build`)
- [ ] Web build succeeds (`npm run build:web`)

### Manual Testing
- [ ] `npm run dev:web` starts Vite dev server
- [ ] Browser loads without Tauri-related errors in console
- [ ] Kanban board renders with mock tasks
- [ ] Navigation between views works
- [ ] Forms submit without error (even if no-op)

### Playwright Verification
- [ ] `npx playwright test` runs successfully
- [ ] Visual regression screenshots captured
- [ ] Tests pass consistently

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] isWebMode detection returns true in browser, false in Tauri
- [ ] Mock API is selected when isWebMode is true
- [ ] EventProvider wraps app and provides correct bus type
- [ ] All event hooks successfully migrated to useEventBus

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
