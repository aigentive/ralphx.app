# RalphX - Phase 48: Activity Screen Enhancement

## Overview

Enhance the activity screen with persistent storage, server-side pagination with infinite scroll, filtering capabilities, and capture missing event types (thinking blocks, tool results). Also improve the "agent is executing" widget UX across all chat types by unifying the status badge and activity indicator.

**Reference Plan:**
- `specs/plans/activity_screen_enhancement_plan.md` - Detailed implementation plan with SQL schema, pagination strategy, and component structure

## Goals

1. **Capture missing events** - Parse and emit thinking blocks and tool results from Claude's stream
2. **Persist activity events** - Store events to SQLite with task/session context and status snapshots
3. **Server-side pagination** - Cursor-based pagination with infinite scroll for browsing historical events
4. **Unified status widget** - Combine status badge and activity indicator into single `StatusActivityBadge` component

## Dependencies

### Phase 47 (Review System Fix) - Required

| Dependency | Why Needed |
|------------|------------|
| Status constants consolidated | Activity events reference task status values |
| Review system stable | Events capture review-related status transitions |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/activity_screen_enhancement_plan.md`
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

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/activity_screen_enhancement_plan.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Add Thinking variant to StreamEvent and parse thinking blocks from Claude stream",
    "plan_section": "Part 1: Capture Missing Event Types",
    "blocking": [8],
    "blockedBy": [],
    "atomic_commit": "feat(streaming): add Thinking variant and parse thinking blocks",
    "steps": [
      "Read specs/plans/activity_screen_enhancement_plan.md section 'Part 1: Capture Missing Event Types'",
      "Add `Thinking(String)` variant to StreamEvent enum in stream_processor.rs",
      "In StreamProcessor::process_message(), detect content_block with type='thinking' and emit StreamEvent::Thinking",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(streaming): add Thinking variant and parse thinking blocks"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "backend",
    "description": "Emit thinking and tool_result events to AGENT_MESSAGE for activity stream",
    "plan_section": "Part 1: Capture Missing Event Types",
    "blocking": [8],
    "blockedBy": [1],
    "atomic_commit": "feat(streaming): emit thinking and tool_result as AGENT_MESSAGE events",
    "steps": [
      "Read specs/plans/activity_screen_enhancement_plan.md section 'Part 1: Capture Missing Event Types'",
      "In chat_service_streaming.rs, add handler for StreamEvent::Thinking to emit AGENT_MESSAGE with type='thinking'",
      "Modify existing ToolResultReceived handler to ALSO emit to AGENT_MESSAGE with type='tool_result'",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(streaming): emit thinking and tool_result as AGENT_MESSAGE events"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "backend",
    "description": "Create ActivityEvent entity definition",
    "plan_section": "Task 2.1: Create activity_events entity",
    "blocking": [4, 5],
    "blockedBy": [],
    "atomic_commit": "feat(domain): add ActivityEvent entity",
    "steps": [
      "Read specs/plans/activity_screen_enhancement_plan.md section 'Task 2.1: Create activity_events entity'",
      "Create src-tauri/src/domain/entities/activity_event.rs with ActivityEvent struct",
      "Include fields: id, task_id, ideation_session_id, internal_status, event_type, role, content, metadata, created_at",
      "Add ActivityEventType enum (Thinking, ToolCall, ToolResult, Text, Error)",
      "Export from src-tauri/src/domain/entities/mod.rs",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(domain): add ActivityEvent entity"
    ],
    "passes": true
  },
  {
    "id": 4,
    "category": "backend",
    "description": "Create ActivityEventRepository trait",
    "plan_section": "Task 2.2: Create repository trait",
    "blocking": [6],
    "blockedBy": [3],
    "atomic_commit": "feat(domain): add ActivityEventRepository trait",
    "steps": [
      "Read specs/plans/activity_screen_enhancement_plan.md section 'Task 2.2: Create repository trait'",
      "Create src-tauri/src/domain/repositories/activity_event_repo.rs",
      "Define trait with: save(), list_by_task_id(), list_by_session_id() with pagination params",
      "Add ActivityEventFilter struct for filtering by event_type, role, status",
      "Export from src-tauri/src/domain/repositories/mod.rs",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(domain): add ActivityEventRepository trait"
    ],
    "passes": true
  },
  {
    "id": 5,
    "category": "backend",
    "description": "Add activity_events database migration",
    "plan_section": "Task 2.3: Add database migration",
    "blocking": [6],
    "blockedBy": [3],
    "atomic_commit": "feat(migrations): add activity_events table",
    "steps": [
      "Read specs/plans/activity_screen_enhancement_plan.md section 'Task 2.3: Add database migration'",
      "Create src-tauri/src/infrastructure/sqlite/migrations/v3_add_activity_events.rs",
      "Use CREATE TABLE IF NOT EXISTS with schema from plan",
      "Add indexes for task_id, session_id, event_type, created_at DESC",
      "Register in MIGRATIONS array in mod.rs, bump SCHEMA_VERSION",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(migrations): add activity_events table"
    ],
    "passes": true
  },
  {
    "id": 6,
    "category": "backend",
    "description": "Implement SQLite repository for activity events",
    "plan_section": "Task 2.4: Implement SQLite repository",
    "blocking": [7],
    "blockedBy": [4, 5],
    "atomic_commit": "feat(sqlite): implement ActivityEventRepository",
    "steps": [
      "Read specs/plans/activity_screen_enhancement_plan.md section 'Task 2.4: Implement SQLite repository'",
      "Create src-tauri/src/infrastructure/sqlite/sqlite_activity_event_repo.rs",
      "Implement save() with INSERT statement",
      "Implement list methods with cursor-based pagination using (created_at, id) tuple",
      "Add filtering support for event_type, role, internal_status",
      "Export from src-tauri/src/infrastructure/sqlite/mod.rs",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(sqlite): implement ActivityEventRepository"
    ],
    "passes": true
  },
  {
    "id": 7,
    "category": "backend",
    "description": "Wire ActivityEventRepository to app state",
    "plan_section": "Task 2.5: Wire repository to app state",
    "blocking": [8, 9],
    "blockedBy": [6],
    "atomic_commit": "feat(app): wire ActivityEventRepository to app state",
    "steps": [
      "Read specs/plans/activity_screen_enhancement_plan.md section 'Task 2.5: Wire repository to app state'",
      "Add activity_event_repo field to AppState struct",
      "Initialize SqliteActivityEventRepository in create_app_state()",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(app): wire ActivityEventRepository to app state"
    ],
    "passes": true
  },
  {
    "id": 8,
    "category": "backend",
    "description": "Persist activity events when emitting stream events",
    "plan_section": "Task 2.7: Persist events when emitting",
    "blocking": [],
    "blockedBy": [2, 7],
    "atomic_commit": "feat(streaming): persist activity events to database",
    "steps": [
      "Read specs/plans/activity_screen_enhancement_plan.md section 'Task 2.7: Persist events when emitting'",
      "In chat_service_streaming.rs, inject activity_event_repo via app_state",
      "When emitting AGENT_MESSAGE events, also save to activity_event_repo",
      "Capture current internal_status from task/session state",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(streaming): persist activity events to database"
    ],
    "passes": true
  },
  {
    "id": 9,
    "category": "backend",
    "description": "Add Tauri commands for activity event pagination and filtering",
    "plan_section": "Task 2.6: Add Tauri commands for pagination/filtering",
    "blocking": [10],
    "blockedBy": [7],
    "atomic_commit": "feat(commands): add activity event pagination commands",
    "steps": [
      "Read specs/plans/activity_screen_enhancement_plan.md section 'Task 2.6: Add Tauri commands for pagination/filtering'",
      "Create src-tauri/src/commands/activity_commands.rs",
      "Add list_task_activity_events(task_id, cursor, limit, filter) command",
      "Add list_session_activity_events(session_id, cursor, limit, filter) command",
      "Return { events: Vec<ActivityEvent>, cursor: Option<String>, has_more: bool }",
      "Export from mod.rs, register in lib.rs",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(commands): add activity event pagination commands"
    ],
    "passes": true
  },
  {
    "id": 10,
    "category": "frontend",
    "description": "Add activity events API wrapper with Zod schemas",
    "plan_section": "Task 3.1: Add API wrapper with Zod schemas",
    "blocking": [11],
    "blockedBy": [9],
    "atomic_commit": "feat(api): add activity events API wrapper",
    "steps": [
      "Read specs/plans/activity_screen_enhancement_plan.md section 'Task 3.1: Add API wrapper with Zod schemas'",
      "Create src/api/activity-events.ts",
      "Define ActivityEventSchema with snake_case fields matching backend",
      "Add transform to camelCase for frontend types",
      "Add listTaskActivityEvents() and listSessionActivityEvents() functions",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(api): add activity events API wrapper"
    ],
    "passes": true
  },
  {
    "id": 11,
    "category": "frontend",
    "description": "Add TanStack Query infinite query hook for activity events",
    "plan_section": "Task 3.2: Add TanStack Query infinite hook",
    "blocking": [12],
    "blockedBy": [10],
    "atomic_commit": "feat(hooks): add useActivityEvents infinite query hook",
    "steps": [
      "Read specs/plans/activity_screen_enhancement_plan.md section 'Task 3.2: Add TanStack Query infinite hook'",
      "Create src/hooks/useActivityEvents.ts",
      "Implement useTaskActivityEvents(taskId, filter) with useInfiniteQuery",
      "Implement useSessionActivityEvents(sessionId, filter) with useInfiniteQuery",
      "Handle getNextPageParam using cursor from response",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(hooks): add useActivityEvents infinite query hook"
    ],
    "passes": true
  },
  {
    "id": 12,
    "category": "frontend",
    "description": "Enhance ActivityView with infinite scroll and historical mode",
    "plan_section": "Task 3.3: Enhance ActivityView with infinite scroll",
    "blocking": [16],
    "blockedBy": [11],
    "atomic_commit": "feat(activity): add infinite scroll and historical mode",
    "steps": [
      "Read specs/plans/activity_screen_enhancement_plan.md section 'Task 3.3: Enhance ActivityView with infinite scroll'",
      "Import useInView from react-intersection-observer",
      "Add mode toggle: real-time (Zustand) vs historical (database)",
      "In historical mode, use useTaskActivityEvents with infinite scroll",
      "Add sentinel div with ref={loadMoreRef} at end of list",
      "Add status filter dropdown alongside existing type filters",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(activity): add infinite scroll and historical mode"
    ],
    "passes": true
  },
  {
    "id": 13,
    "category": "frontend",
    "description": "Add activityFilter state to uiStore",
    "plan_section": "Task 4.1: Add activityFilter to uiStore",
    "blocking": [14, 16],
    "blockedBy": [],
    "atomic_commit": "feat(store): add activityFilter state to uiStore",
    "steps": [
      "Read specs/plans/activity_screen_enhancement_plan.md section 'Task 4.1: Add activityFilter to uiStore'",
      "Add activityFilter: { taskId: string | null, sessionId: string | null } to uiStore",
      "Add setActivityFilter(filter) action",
      "Add clearActivityFilter() action",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(store): add activityFilter state to uiStore"
    ],
    "passes": true
  },
  {
    "id": 14,
    "category": "frontend",
    "description": "Create StatusActivityBadge unified component",
    "plan_section": "Task 4.2: Create StatusActivityBadge component",
    "blocking": [15],
    "blockedBy": [13],
    "atomic_commit": "feat(chat): create StatusActivityBadge component",
    "steps": [
      "Read specs/plans/activity_screen_enhancement_plan.md section 'Task 4.2: Create StatusActivityBadge component'",
      "Create src/components/Chat/StatusActivityBadge.tsx",
      "Props: isAgentActive, agentType, contextType, contextId",
      "Implement behavior by state: hidden when idle+no activity, muted when idle+has activity, badge when active",
      "onClick: setActivityFilter({taskId/sessionId}) then setCurrentView('activity')",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(chat): create StatusActivityBadge component"
    ],
    "passes": true
  },
  {
    "id": 15,
    "category": "frontend",
    "description": "Integrate StatusActivityBadge in chat panels, remove WorkerExecutingIndicator",
    "plan_section": "Task 4.3: Integrate StatusActivityBadge in chat panels",
    "blocking": [],
    "blockedBy": [14],
    "atomic_commit": "feat(chat): replace Badge + WorkerExecutingIndicator with StatusActivityBadge",
    "steps": [
      "Read specs/plans/activity_screen_enhancement_plan.md section 'Task 4.3: Integrate StatusActivityBadge in chat panels'",
      "In IntegratedChatPanel.tsx: replace Badge with StatusActivityBadge, remove WorkerExecutingIndicator usage",
      "In ChatPanel.tsx: same changes",
      "In ChatMessageList.tsx: remove isExecutionMode prop and WorkerExecutingIndicator rendering",
      "Remove WorkerExecutingIndicator component if no longer used",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(chat): replace Badge + WorkerExecutingIndicator with StatusActivityBadge"
    ],
    "passes": true
  },
  {
    "id": 16,
    "category": "frontend",
    "description": "Wire ActivityView to read filter from uiStore for context-aware navigation",
    "plan_section": "Task 4.4: Wire ActivityView to read filter from store",
    "blocking": [],
    "blockedBy": [12, 13],
    "atomic_commit": "feat(activity): read filter from uiStore for context-aware navigation",
    "steps": [
      "Read specs/plans/activity_screen_enhancement_plan.md section 'Task 4.4: Wire ActivityView to read filter from store'",
      "In App.tsx, pass uiStore.activityFilter to ActivityView",
      "In ActivityView, auto-select historical mode when filter.taskId or filter.sessionId is set",
      "Clear filter when user manually changes filters",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(activity): read filter from uiStore for context-aware navigation"
    ],
    "passes": true
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
| **Cursor-based pagination** | More efficient than offset for large datasets, no missed/duplicate items on insert |
| **Polymorphic context (task_id XOR session_id)** | Events belong to exactly one context; CHECK constraint enforces this |
| **Dual-mode ActivityView** | Real-time for live monitoring, historical for debugging past executions |
| **Unified StatusActivityBadge** | Reduces component redundancy, consistent UX across all chat types |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] ActivityEvent entity serializes correctly
- [ ] Migration creates table with all indexes
- [ ] Repository pagination returns correct cursors
- [ ] Thinking blocks are parsed from stream

### Frontend - Run `npm run test`
- [ ] API wrapper transforms snake_case to camelCase
- [ ] Infinite query hook loads next pages correctly
- [ ] StatusActivityBadge renders in all states

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`cargo build --release` / `npm run build`)

### Manual Testing
- [ ] Start worker execution, verify thinking blocks appear in activity
- [ ] Start worker execution, verify tool results appear in activity
- [ ] Refresh app, verify historical events load from database
- [ ] Generate 100+ events, verify infinite scroll loads more
- [ ] Filter by status/type, verify correct results
- [ ] In ideation chat, verify activity link accessible when agent running
- [ ] In task chat, verify activity link accessible when agent running
- [ ] Click activity from task chat, verify events filtered to that task

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] StatusActivityBadge onClick navigates to Activity view with filter set
- [ ] ActivityView reads filter from uiStore and auto-selects historical mode
- [ ] list_task_activity_events command is called by frontend API wrapper
- [ ] Thinking blocks flow: stream → StreamProcessor → chat_service → AGENT_MESSAGE event → save to DB

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
