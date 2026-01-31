# RalphX - Phase 57: Activity Page Global History & UX

## Overview

This phase enables global history view for the Activity page and improves the UX with better mode visibility and filtering. Currently, the History button is disabled when there's no task/session context, and there's no way to view all activity events across the system. Users also don't notice the Live/History toggle and get no guidance when the Live mode is empty.

**Reference Plan:**
- `specs/plans/fix_activity_page_global_history_ux.md` - Detailed implementation plan with code snippets and architecture decisions

## Goals

1. Enable global activity history view (show ALL events regardless of context)
2. Add optional task/session filtering via searchable dropdowns
3. Improve Live mode visibility with pulsating indicator when receiving events
4. Add smart mode behavior that respects user's manual mode selection

## Dependencies

### Phase 48 (Activity Screen Enhancement) - Required

| Dependency | Why Needed |
|------------|------------|
| ActivityEventRepository | Existing repository pattern for activity events |
| Cursor-based pagination | list_all will use same pagination pattern |
| ActivityView component | Base component to enhance with global history |

### Phase 52 (Activity Screen UI Improvements) - Required

| Dependency | Why Needed |
|------------|------------|
| ViewModeToggle component | Component to enable globally |
| ActivityFilters | Component to add task/session dropdowns |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/fix_activity_page_global_history_ux.md`
2. Understand the architecture and component structure
3. Then proceed with the specific task

Each task follows this pattern:

1. Read the relevant section in the implementation plan
2. Implement according to the plan's specifications
3. Write functional tests where appropriate
4. Run linters for modified code only (backend: `cargo clippy`, frontend: `npm run lint && npm run typecheck`)
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

**Task Dependency Graph:**
```
Task 1 (Backend: trait + impls)
    ↓
Task 2 (Backend: command)
    ↓
Task 3 (Frontend: types)
    ↓
Task 4 (Frontend: API wrapper)
    ↓
Task 5 (Frontend: hook)
    ↓
Task 6 (UI: enable global history) ──────────────────────┐
    ↓                      ↓                      ↓      │
Task 7 (TaskFilter)   Task 8 (SessionFilter)  Task 10-12 │
    ↓                      ↓                   (UX Polish)
    └──────────┬───────────┘
               ↓
         Task 9 (Wire filters)
```

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/fix_activity_page_global_history_ux.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Add list_all to repository trait and implement in all repos",
    "plan_section": "Task 1: Add list_all to repository trait and implement in all repos",
    "blocking": [2],
    "blockedBy": [],
    "atomic_commit": "feat(activity): add list_all method to activity event repository",
    "steps": [
      "Read specs/plans/fix_activity_page_global_history_ux.md section 'Task 1'",
      "Add list_all method signature to ActivityEventRepository trait",
      "Add optional task_id and session_id fields to ActivityEventFilter struct",
      "Implement list_all in SqliteActivityEventRepo with optional WHERE clauses",
      "Implement list_all in MemoryActivityEventRepo",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(activity): add list_all method to activity event repository"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Add Tauri command and register",
    "plan_section": "Task 2: Add Tauri command and register",
    "blocking": [3],
    "blockedBy": [1],
    "atomic_commit": "feat(activity): add list_all_activity_events command",
    "steps": [
      "Read specs/plans/fix_activity_page_global_history_ux.md section 'Task 2'",
      "Add list_all_activity_events command to activity_commands.rs",
      "Register command in lib.rs invoke_handler",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(activity): add list_all_activity_events command"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "frontend",
    "description": "Extend ActivityEventFilter type with taskId/sessionId",
    "plan_section": "Task 3: Extend ActivityEventFilter type",
    "blocking": [4],
    "blockedBy": [2],
    "atomic_commit": "feat(api): extend ActivityEventFilter with taskId/sessionId",
    "steps": [
      "Read specs/plans/fix_activity_page_global_history_ux.md section 'Task 3'",
      "Add optional taskId and sessionId to ActivityEventFilter in activity-events.types.ts",
      "Update schema in activity-events.schemas.ts if it exists",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(api): extend ActivityEventFilter with taskId/sessionId"
    ],
    "passes": false
  },
  {
    "id": 4,
    "category": "frontend",
    "description": "Add API wrapper for list_all",
    "plan_section": "Task 4: Add API wrapper for list_all",
    "blocking": [5],
    "blockedBy": [3],
    "atomic_commit": "feat(api): add activityEventsApi.all.list() wrapper",
    "steps": [
      "Read specs/plans/fix_activity_page_global_history_ux.md section 'Task 4'",
      "Add all.list() method to activityEventsApi in activity-events.ts",
      "Use typedInvokeWithTransform with appropriate schema",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(api): add activityEventsApi.all.list() wrapper"
    ],
    "passes": false
  },
  {
    "id": 5,
    "category": "frontend",
    "description": "Add useAllActivityEvents hook",
    "plan_section": "Task 5: Add useAllActivityEvents hook",
    "blocking": [6],
    "blockedBy": [4],
    "atomic_commit": "feat(hooks): add useAllActivityEvents hook",
    "steps": [
      "Read specs/plans/fix_activity_page_global_history_ux.md section 'Task 5'",
      "Add useAllActivityEvents hook to useActivityEvents.ts",
      "Support optional taskId/sessionId filter parameters",
      "Follow existing useActivityEvents pattern with TanStack Query",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(hooks): add useAllActivityEvents hook"
    ],
    "passes": false
  },
  {
    "id": 6,
    "category": "frontend",
    "description": "Enable ViewModeToggle globally and use global query",
    "plan_section": "Task 6: Enable ViewModeToggle globally and use global query",
    "blocking": [7, 8, 10, 11, 12],
    "blockedBy": [5],
    "atomic_commit": "feat(activity): enable global history view",
    "steps": [
      "Read specs/plans/fix_activity_page_global_history_ux.md section 'Task 6'",
      "Remove disabled={!taskId && !sessionId} from ViewModeToggle in ActivityView.tsx",
      "Update ActivityView to use useAllActivityEvents when no context provided",
      "Change default mode to 'historical' instead of conditional",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(activity): enable global history view"
    ],
    "passes": false
  },
  {
    "id": 7,
    "category": "frontend",
    "description": "Add TaskFilter searchable dropdown",
    "plan_section": "Task 7: Add TaskFilter searchable dropdown",
    "blocking": [9],
    "blockedBy": [6],
    "atomic_commit": "feat(activity): add task filter dropdown",
    "steps": [
      "Read specs/plans/fix_activity_page_global_history_ux.md section 'Task 7'",
      "Create TaskFilter component using shadcn Command + Popover pattern",
      "Fetch recent tasks (last 10-15) to show in dropdown",
      "Implement search/filter as user types",
      "Add to ActivityFilters or create new file",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(activity): add task filter dropdown"
    ],
    "passes": false
  },
  {
    "id": 8,
    "category": "frontend",
    "description": "Add SessionFilter searchable dropdown",
    "plan_section": "Task 8: Add SessionFilter searchable dropdown",
    "blocking": [9],
    "blockedBy": [6],
    "atomic_commit": "feat(activity): add session filter dropdown",
    "steps": [
      "Read specs/plans/fix_activity_page_global_history_ux.md section 'Task 8'",
      "Create SessionFilter component using shadcn Command + Popover pattern",
      "Fetch recent sessions (last 10-15) to show in dropdown",
      "Implement search/filter as user types",
      "Add to ActivityFilters or create new file",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(activity): add session filter dropdown"
    ],
    "passes": false
  },
  {
    "id": 9,
    "category": "frontend",
    "description": "Wire filters to the global activity query",
    "plan_section": "Task 9: Wire filters to the query",
    "blocking": [],
    "blockedBy": [7, 8],
    "atomic_commit": "feat(activity): wire filters to global activity query",
    "steps": [
      "Read specs/plans/fix_activity_page_global_history_ux.md section 'Task 9'",
      "Add filter state to ActivityView for selected task/session",
      "Pass filter values to useAllActivityEvents hook",
      "Ensure filter changes trigger query refetch",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(activity): wire filters to global activity query"
    ],
    "passes": false
  },
  {
    "id": 10,
    "category": "frontend",
    "description": "Add pulsating Live indicator",
    "plan_section": "Task 10: Add pulsating Live indicator",
    "blocking": [],
    "blockedBy": [6],
    "atomic_commit": "feat(activity): add pulsating animation for live mode",
    "steps": [
      "Read specs/plans/fix_activity_page_global_history_ux.md section 'Task 10'",
      "Add lastEventTime to activityStore for detecting isReceiving",
      "Add isReceiving prop to ViewModeToggle component",
      "Implement pulsating orange (#ff6b35) animation when receiving events",
      "Use Date.now() - lastEventTime < 5000 to determine pulsating state",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(activity): add pulsating animation for live mode"
    ],
    "passes": false
  },
  {
    "id": 11,
    "category": "frontend",
    "description": "Improve empty state with History hint",
    "plan_section": "Task 11: Improve empty state with History hint",
    "blocking": [],
    "blockedBy": [6],
    "atomic_commit": "feat(activity): add history hint to live mode empty state",
    "steps": [
      "Read specs/plans/fix_activity_page_global_history_ux.md section 'Task 11'",
      "Update empty state in Live mode to mention History button",
      "Add clear call-to-action directing user to History view",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(activity): add history hint to live mode empty state"
    ],
    "passes": false
  },
  {
    "id": 12,
    "category": "frontend",
    "description": "Add userLockedMode to prevent auto-switch",
    "plan_section": "Task 12: Add userLockedMode to prevent auto-switch",
    "blocking": [],
    "blockedBy": [6],
    "atomic_commit": "feat(activity): respect user's manual mode selection",
    "steps": [
      "Read specs/plans/fix_activity_page_global_history_ux.md section 'Task 12'",
      "Add userLockedMode state to track manual mode selection",
      "Update handleViewModeChange to set userLockedMode = true",
      "Modify auto-switch effect to check userLockedMode before switching",
      "Only auto-switch to Live if user hasn't manually chosen History",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(activity): respect user's manual mode selection"
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
| **Extend ActivityEventFilter instead of new filter type** | Reuses existing filter infrastructure, single source of truth |
| **list_all method instead of modifying existing methods** | Non-breaking change, existing list_by_task_id/session_id remain unchanged |
| **userLockedMode state pattern** | Simple boolean flag prevents complex mode management, respects explicit user choice |
| **lastEventTime for pulsating detection** | Lightweight approach using timestamp comparison vs complex event tracking |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] list_all returns all events when no filter provided
- [ ] list_all with task_id filter returns only that task's events
- [ ] list_all with session_id filter returns only that session's events
- [ ] Pagination works correctly with list_all

### Frontend - Run `npm run test`
- [ ] useAllActivityEvents hook fetches data correctly
- [ ] Filter state updates trigger query refetch

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`cargo build --release` / `npm run build`)

### Manual Testing
- [ ] Open Activity from sidebar (no context) → History enabled, shows all events
- [ ] Filter by task → only that task's events displayed
- [ ] Filter by session → only that session's events displayed
- [ ] Switch to Live mode → pulsating icon appears when events arrive
- [ ] Empty state in Live mode shows History hint
- [ ] Select History mode manually → stays on History even with new events arriving

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] Entry point identified (ViewModeToggle click, filter dropdown selection)
- [ ] useAllActivityEvents hook is called with correct parameters
- [ ] API wrapper calls list_all_activity_events command
- [ ] Backend list_all repository method returns correct data
- [ ] UI reflects filter and mode changes correctly

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
