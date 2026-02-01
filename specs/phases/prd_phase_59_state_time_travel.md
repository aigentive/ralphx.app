# RalphX - Phase 59: State Time Travel

## Overview

Add the ability to view historical states of a task - see exactly what happened during execution, review, escalation, etc. Users can "rewind" to any past state and see the task details + conversation as they were at that point.

This feature enables debugging and understanding of task history by allowing users to click on any past state in a timeline and view the task detail + chat as it was at that point.

**Reference Plan:**
- `specs/plans/state_time_travel_feature.md` - Detailed implementation plan with component designs and execution order

## Goals

1. Enable viewing historical task states via timeline navigation
2. Filter chat messages by historical state for focused context
3. Render state-appropriate detail views using existing view registry
4. Provide clear UX for history mode with banner and return button

## Dependencies

### Phase 58 (Wire TaskRerunDialog) - Required

| Dependency | Why Needed |
|------------|------------|
| TaskDetailOverlay | History mode state management added here |
| View Registry | Existing state-to-component mapping used for historical views |

### Existing Infrastructure - Available

| Component | Already Available |
|-----------|------------------|
| `task_state_history` table | Stores all state transitions with timestamps |
| `activity_events.internal_status` | Messages tagged by state for filtering |
| View Registry | Maps InternalStatus to detail components |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/state_time_travel_feature.md`
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

**Recommended execution order (from dependency graph):**
```
Task 1 (backend)
    ↓
Task 2 (hook)
    ↓
Task 3 (StateTimelineNav)
    ↓
Task 4 (TaskDetailOverlay)
    ↓
┌───┴───┐
Task 5  Task 6  (can run in parallel)
```

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/state_time_travel_feature.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Add get_task_state_transitions command",
    "plan_section": "Task 6: Backend - Add State Transitions Query",
    "blocking": [2],
    "blockedBy": [],
    "atomic_commit": "feat(backend): add get_task_state_transitions command",
    "steps": [
      "Read specs/plans/state_time_travel_feature.md section 'Task 6: Backend'",
      "Check if task_state_history query already exists in task_commands.rs",
      "If not, add get_task_state_transitions command that queries task_state_history table",
      "Return chronological list of StateTransition (id, from_status, to_status, timestamp, reason)",
      "Run cargo clippy --all-targets --all-features -- -D warnings && cargo test",
      "Commit: feat(backend): add get_task_state_transitions command"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "frontend",
    "description": "Create useTaskStateTransitions hook",
    "plan_section": "Task 5: Extend useTaskStateHistory Hook",
    "blocking": [3],
    "blockedBy": [1],
    "atomic_commit": "feat(hooks): add useTaskStateTransitions hook for history timeline",
    "steps": [
      "Read specs/plans/state_time_travel_feature.md section 'Task 5: Extend useTaskStateHistory Hook'",
      "Create src/hooks/useTaskStateTransitions.ts with StateTransition interface",
      "Add API wrapper for get_task_state_transitions command",
      "Export useTaskStateTransitions hook that fetches and returns ordered transitions",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(hooks): add useTaskStateTransitions hook for history timeline"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "frontend",
    "description": "Create StateTimelineNav component",
    "plan_section": "Task 1: Create StateTimelineNav Component",
    "blocking": [4],
    "blockedBy": [2],
    "atomic_commit": "feat(tasks): add StateTimelineNav component for history navigation",
    "steps": [
      "Read specs/plans/state_time_travel_feature.md section 'Task 1: Create StateTimelineNav Component'",
      "Create src/components/tasks/StateTimelineNav.tsx with props: taskId, currentStatus, onStateSelect, selectedState?",
      "Use useTaskStateTransitions hook to fetch state history",
      "Render clickable state badges in chronological order",
      "Highlight selected state vs current state",
      "Show timestamps on hover",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(tasks): add StateTimelineNav component for history navigation"
    ],
    "passes": true
  },
  {
    "id": 4,
    "category": "frontend",
    "description": "Add history mode state and banner to TaskDetailOverlay",
    "plan_section": "Task 2: Add History Mode State to TaskDetailOverlay",
    "blocking": [5, 6],
    "blockedBy": [3],
    "atomic_commit": "feat(tasks): add history mode state and banner to TaskDetailOverlay",
    "steps": [
      "Read specs/plans/state_time_travel_feature.md section 'Task 2: Add History Mode State'",
      "Add historyState useState to TaskDetailOverlay: { status: InternalStatus; timestamp: string } | null",
      "Compute isHistoryMode and viewStatus from historyState",
      "Insert StateTimelineNav below header",
      "Add history mode banner with 'Return to Current' button when isHistoryMode",
      "Pass historyState and viewStatus to child components (TaskDetailPanel, TaskChatPanel)",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(tasks): add history mode state and banner to TaskDetailOverlay"
    ],
    "passes": false
  },
  {
    "id": 5,
    "category": "frontend",
    "description": "Add viewAsStatus prop to TaskDetailPanel for history mode",
    "plan_section": "Task 3: Extend TaskDetailPanel for History Mode",
    "blocking": [],
    "blockedBy": [4],
    "atomic_commit": "feat(tasks): add viewAsStatus prop to TaskDetailPanel for history mode",
    "steps": [
      "Read specs/plans/state_time_travel_feature.md section 'Task 3: Extend TaskDetailPanel'",
      "Add optional viewAsStatus prop to TaskDetailPanel interface",
      "When viewAsStatus provided, use it for view registry lookup instead of task.internalStatus",
      "Pass isHistorical boolean to child detail views so they render read-only (no action buttons)",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(tasks): add viewAsStatus prop to TaskDetailPanel for history mode"
    ],
    "passes": false
  },
  {
    "id": 6,
    "category": "frontend",
    "description": "Add historical message filtering to TaskChatPanel",
    "plan_section": "Task 4: Add Historical Message Filtering to TaskChatPanel",
    "blocking": [],
    "blockedBy": [4],
    "atomic_commit": "feat(tasks): add historical message filtering to TaskChatPanel",
    "steps": [
      "Read specs/plans/state_time_travel_feature.md section 'Task 4: Add Historical Message Filtering'",
      "Add optional historicalStatus prop to TaskChatPanel interface",
      "Modify useTaskChat hook to accept historicalStatus parameter",
      "When historicalStatus provided, filter activity_events by internal_status field",
      "Disable message input when in historical mode (read-only)",
      "Add 'Historical view' indicator in chat header",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(tasks): add historical message filtering to TaskChatPanel"
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
| **Reuse view registry for historical views** | No new components needed - existing state-to-view mapping works for history |
| **Filter chat by internal_status** | activity_events already tagged with status, enabling precise message filtering |
| **Single historyState in overlay** | Centralized state management, child components receive only what they need |
| **Backend query for transitions** | Leverage existing task_state_history table for chronological timeline |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] get_task_state_transitions returns chronological transitions for task

### Frontend - Run `npm run test`
- [ ] StateTimelineNav renders all states from transitions
- [ ] useTaskStateTransitions hook correctly fetches and transforms data

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`cargo build --release` / `npm run build`)

### Manual Testing
- [ ] Open a completed task (e.g., Approved status)
- [ ] Timeline shows: Ready -> Executing -> Reviewing -> Approved
- [ ] Click "Executing" in timeline
- [ ] Banner appears: "Viewing historical state: Executing"
- [ ] Task detail shows ExecutionTaskDetail view
- [ ] Chat shows only messages from executing phase
- [ ] Click "Return to Current" -> back to Approved view
- [ ] Click current state (Approved) -> also exits history mode

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] StateTimelineNav is imported AND rendered in TaskDetailOverlay
- [ ] onStateSelect callback properly sets historyState
- [ ] viewAsStatus prop is passed from overlay to panel
- [ ] historicalStatus prop is passed from overlay to chat
- [ ] useTaskStateTransitions hook is used by StateTimelineNav

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
