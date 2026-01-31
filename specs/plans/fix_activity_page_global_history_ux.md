# Plan: Fix Activity Page - Enable Global History & Improve UX

## Problem Summary

1. **History button disabled** - `ViewModeToggle` is disabled when `!taskId && !sessionId` (line 290)
2. **No global history** - API only has `list_by_task_id` and `list_by_session_id`, no `list_all`
3. **Poor Live mode visibility** - Users don't notice the Live/History toggle
4. **Empty state doesn't guide user** - No hint about History mode

## User Intent (Clarified)

- History mode should show ALL activity events, regardless of whether opened with task/session context
- Each event already shows its origin (task or session) via ActivityContext component
- Optional filtering by task/session should be available via dropdowns
- Live mode should have a pulsating orange icon when receiving events
- Empty state in Live mode should hint about History button
- Default to History mode; only auto-switch to Live when events arrive AND user hasn't manually chosen History

## Solution Overview

### Backend Changes (src-tauri/)

**1. Add `list_all` to repository trait**
- File: `src/domain/repositories/activity_event_repository.rs`
- New method with optional task_id/session_id filters

**2. Implement in SQLite repo**
- File: `src/infrastructure/sqlite/sqlite_activity_event_repo.rs`
- Query all events, optionally filter by task_id or session_id

**3. Implement in Memory repo**
- File: `src/infrastructure/memory/memory_activity_event_repo.rs`

**4. Add Tauri command**
- File: `src/commands/activity_commands.rs`
- New command: `list_all_activity_events`

### Frontend API Changes (src/api/)

**5. Extend filter type**
- File: `activity-events.types.ts`
- Add optional `taskId?: string` and `sessionId?: string` to filter

**6. Add API wrapper**
- File: `activity-events.ts`
- New method: `activityEventsApi.all.list()`

### Frontend Hook Changes (src/hooks/)

**7. Add useAllActivityEvents hook**
- File: `useActivityEvents.ts`
- New hook for global activity with optional filters

### Frontend UI Changes (src/components/activity/)

**8. Enable ViewModeToggle globally**
- File: `ActivityView.tsx:290`
- Remove `disabled={!taskId && !sessionId}`

**9. Add Task/Session filter dropdowns (searchable combobox)**
- File: `ActivityFilters.tsx`
- New `TaskFilter` and `SessionFilter` components using shadcn Combobox pattern
- Show recent items in dropdown, with search/filter as user types

**10. Update ActivityView to use global query**
- File: `ActivityView.tsx`
- Use `useAllActivityEvents` when no context provided
- Pass optional taskId/sessionId filters from dropdown

**11. Improve Live button visibility**
- File: `ActivityFilters.tsx`
- Add `isReceiving` prop to ViewModeToggle
- Pulsating orange animation when Live mode has incoming events

**12. Improve empty state**
- File: `ActivityFilters.tsx`
- Add hint about History button in Live mode empty state

**13. Smart mode behavior**
- Default to History mode
- Track `userLockedMode` state to respect manual choice
- Only auto-switch to Live if user hasn't manually chosen History

## Critical Files

| File | Changes |
|------|---------|
| `src-tauri/src/domain/repositories/activity_event_repository.rs` | Add `list_all` method |
| `src-tauri/src/infrastructure/sqlite/sqlite_activity_event_repo.rs` | Implement `list_all` |
| `src-tauri/src/infrastructure/memory/memory_activity_event_repo.rs` | Implement `list_all` |
| `src-tauri/src/commands/activity_commands.rs` | Add `list_all_activity_events` command |
| `src/api/activity-events.types.ts` | Extend filter with taskId/sessionId |
| `src/api/activity-events.ts` | Add `all.list()` method |
| `src/hooks/useActivityEvents.ts` | Add `useAllActivityEvents` hook |
| `src/components/activity/ActivityView.tsx` | Enable history, smart mode logic |
| `src/components/activity/ActivityFilters.tsx` | Filters, pulsating Live, empty state |

## Task Breakdown

### Phase 1: Backend - Global Activity Query

#### Task 1: Add `list_all` to repository trait and implement in all repos (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(activity): add list_all method to activity event repository`

> **Compilation Unit Note:** Adding a trait method requires ALL implementors to implement it in the same task, otherwise the code won't compile.

Files:
- `src-tauri/src/domain/repositories/activity_event_repository.rs` - trait definition
- `src-tauri/src/infrastructure/sqlite/sqlite_activity_event_repo.rs` - SQLite impl
- `src-tauri/src/infrastructure/memory/memory_activity_event_repo.rs` - Memory impl

#### Task 2: Add Tauri command and register
**Dependencies:** Task 1
**Atomic Commit:** `feat(activity): add list_all_activity_events command`

Files:
- `src-tauri/src/commands/activity_commands.rs` - command
- `src-tauri/src/lib.rs` - register

### Phase 2: Frontend API & Hooks

#### Task 3: Extend ActivityEventFilter type (BLOCKING)
**Dependencies:** Task 2
**Atomic Commit:** `feat(api): extend ActivityEventFilter with taskId/sessionId`

Files:
- `src/api/activity-events.types.ts`
- `src/api/activity-events.schemas.ts` (if exists)

#### Task 4: Add API wrapper for list_all
**Dependencies:** Task 3
**Atomic Commit:** `feat(api): add activityEventsApi.all.list() wrapper`

Files:
- `src/api/activity-events.ts`

#### Task 5: Add useAllActivityEvents hook (BLOCKING)
**Dependencies:** Task 4
**Atomic Commit:** `feat(hooks): add useAllActivityEvents hook`

Files:
- `src/hooks/useActivityEvents.ts`

### Phase 3: UI - Enable Global History

#### Task 6: Enable ViewModeToggle globally and use global query
**Dependencies:** Task 5
**Atomic Commit:** `feat(activity): enable global history view`

Files:
- `src/components/activity/ActivityView.tsx` - remove disabled condition, use useAllActivityEvents
- `src/components/activity/ActivityFilters.tsx` - update ViewModeToggle

### Phase 4: UI - Optional Filtering

#### Task 7: Add TaskFilter searchable dropdown
**Dependencies:** Task 6
**Atomic Commit:** `feat(activity): add task filter dropdown`

Files:
- `src/components/activity/ActivityFilters.tsx` or new `src/components/activity/TaskFilter.tsx`

#### Task 8: Add SessionFilter searchable dropdown
**Dependencies:** Task 6
**Atomic Commit:** `feat(activity): add session filter dropdown`

Files:
- `src/components/activity/ActivityFilters.tsx` or new `src/components/activity/SessionFilter.tsx`

#### Task 9: Wire filters to the query
**Dependencies:** Task 7, Task 8
**Atomic Commit:** `feat(activity): wire filters to global activity query`

Files:
- `src/components/activity/ActivityView.tsx`

### Phase 5: UX Polish

#### Task 10: Add pulsating Live indicator
**Dependencies:** Task 6
**Atomic Commit:** `feat(activity): add pulsating animation for live mode`

Files:
- `src/components/activity/ActivityFilters.tsx` - isReceiving prop, animation
- `src/stores/activityStore.ts` (if needed for lastEventTime)

#### Task 11: Improve empty state with History hint
**Dependencies:** Task 6
**Atomic Commit:** `feat(activity): add history hint to live mode empty state`

Files:
- `src/components/activity/ActivityFilters.tsx` or `src/components/activity/ActivityView.tsx`

#### Task 12: Add userLockedMode to prevent auto-switch
**Dependencies:** Task 6
**Atomic Commit:** `feat(activity): respect user's manual mode selection`

Files:
- `src/components/activity/ActivityView.tsx`

## Verification

1. Open Activity page directly (no task/session context)
2. History button should be enabled
3. History mode should show all past events with task/session origin visible
4. Filter by task/session should narrow results
5. Live mode should work with pulsating indicator when events arrive
6. Empty state in Live should mention History button
7. Manual History selection should not auto-switch to Live

## Design Notes

- Accent color: `#ff6b35` (warm orange) - used for pulsating Live indicator
- Pulsation: subtle glow animation, not intrusive
- Filter dropdowns: searchable combobox (recent items + type to filter)

## Implementation Details

### Backend: `list_all` method signature

```rust
/// List all activity events with cursor-based pagination
async fn list_all(
    &self,
    cursor: Option<&str>,
    limit: u32,
    filter: Option<&ActivityEventFilter>,
) -> AppResult<ActivityEventPage>;
```

The existing `ActivityEventFilter` already supports `event_types`, `roles`, `statuses`. We'll add optional `task_id` and `session_id` fields to it.

### Frontend: Searchable Combobox Pattern

Use shadcn/ui `Command` + `Popover` for searchable dropdown:
- On open: show recent tasks/sessions (last 10-15)
- On type: filter by title/name matching input
- On select: update filter state, refetch query

### Activity Store: Track last event time

Add `lastEventTime` to activityStore for detecting "isReceiving":
```typescript
lastEventTime: number | null;
// Update in addMessage()
```

In ViewModeToggle, compare `Date.now() - lastEventTime < 5000` to determine pulsating state.

### Smart Mode Logic

```typescript
// State
const [userLockedMode, setUserLockedMode] = useState(false);

// Default
const defaultMode: ViewMode = "historical"; // Changed from conditional

// Handle mode change
const handleViewModeChange = (mode: ViewMode) => {
  setViewMode(mode);
  setUserLockedMode(true); // User explicitly chose
};

// Auto-switch effect (only if not locked)
useEffect(() => {
  if (!userLockedMode && realtimeMessages.length > 0 && viewMode === "historical") {
    setViewMode("realtime");
  }
}, [realtimeMessages.length, userLockedMode, viewMode]);
```

## Testing Checklist

- [ ] Backend: `cargo test` passes for new repository methods
- [ ] Backend: `cargo clippy` clean
- [ ] Frontend: `npm run typecheck` passes
- [ ] Frontend: `npm run lint` passes
- [ ] Manual: Open Activity from sidebar → History enabled, shows all events
- [ ] Manual: Filter by task → only that task's events
- [ ] Manual: Switch to Live → pulsating icon when events arrive
- [ ] Manual: Switch to History manually → stays on History even with new events

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)

### Task Dependency Graph

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

### Parallel Execution Opportunities

- Tasks 7, 8, 10, 11, 12 can run in parallel after Task 6 completes
- Task 9 must wait for both Task 7 and Task 8
