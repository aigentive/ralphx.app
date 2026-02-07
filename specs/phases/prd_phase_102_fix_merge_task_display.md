# RalphX - Phase 102: Fix Merge Task Display in Graph + Kanban

## Overview

Auto-created merge tasks (category `plan_merge`) cause two display issues: they inflate tier counts in the task graph (creating phantom tiers that trigger unwanted tier grouping), and they clutter the Kanban Blocked column with system-managed tasks that aren't user-actionable. This phase fixes both views by filtering merge tasks from tier calculations and hiding them from the Kanban board by default with a toggle.

**Reference Plan:**
- `specs/plans/fix_merge_task_display_in_graph_kanban.md` - Detailed implementation plan with code snippets and verification steps

## Goals

1. Prevent merge tasks from inflating tier count in graph tier grouping
2. Hide merge tasks from Kanban board by default
3. Add user toggle to show/hide merge tasks in Kanban (matching existing `showArchived` pattern)

## Dependencies

### Phase 100 (Plan Merge Auto-Promote) - Required

| Dependency | Why Needed |
|------------|------------|
| `plan_merge` category on tasks | Merge tasks must be identifiable by category for filtering |
| Auto-promote merge tasks | Merge tasks exist in Blocked column (the problem this phase fixes) |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/fix_merge_task_display_in_graph_kanban.md`
2. Understand the architecture and component structure
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

**Task Execution Order:**
- Tasks with `"blockedBy": []` can start immediately
- Before starting a task, check `blockedBy` - all listed tasks must have `"passes": true`
- Execute tasks in ID order when dependencies are satisfied

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/fix_merge_task_display_in_graph_kanban.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "frontend",
    "description": "Filter merge tasks from tier group calculation in buildTierGroups() and add tests",
    "plan_section": "Task 1: Filter merge tasks in buildTierGroups() and update tests",
    "blocking": [2],
    "blockedBy": [],
    "atomic_commit": "fix(graph): exclude plan_merge tasks from tier group calculation",
    "steps": [
      "Read specs/plans/fix_merge_task_display_in_graph_kanban.md section 'Task 1'",
      "In tierGroupUtils.ts buildTierGroups(), add `if (node.category === 'plan_merge') continue;` guard after the `if (!node) continue;` check (line 55)",
      "In tierGroupUtils.test.ts, add test: 1 real task + 1 merge task (tier 1) → no tier sub-groups (single effective tier)",
      "In tierGroupUtils.test.ts, add test: 2-tier tasks + merge task (tier 2) → only 2 tiers returned (merge excluded)",
      "Run npm run typecheck && npm test -- tierGroupUtils",
      "Run npm run lint",
      "Commit: fix(graph): exclude plan_merge tasks from tier group calculation"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "frontend",
    "description": "Add showMergeTasks UI store state, filter merge tasks in Column, add toggle in TaskBoard toolbar",
    "plan_section": "Task 2: Add showMergeTasks state to UI store, filter in Column, add toggle in TaskBoard",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "feat(kanban): hide merge tasks by default with toggle",
    "steps": [
      "Read specs/plans/fix_merge_task_display_in_graph_kanban.md section 'Task 2'",
      "In uiStore.ts: add showMergeTasks: boolean (default: false) to UiState, setShowMergeTasks action to UiActions, implement in store (same pattern as showArchived/setShowArchived)",
      "In Column.tsx: read showMergeTasks from useUiStore, filter tasks where task.category === 'plan_merge' when showMergeTasks is false (in the tasks useMemo after flattening/sorting)",
      "In TaskBoard.tsx: add showMergeTasks/setShowMergeTasks from useUiStore, compute merge task count from all columns, add Toggle with GitMerge icon near the existing Archive toggle (only visible when merge tasks exist)",
      "Run npm run typecheck",
      "Run npm run lint",
      "Commit: feat(kanban): hide merge tasks by default with toggle"
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
| **Client-side filtering (not backend)** | Merge tasks are few (1 per plan), client-side filter is simpler and avoids API changes |
| **Follow showArchived pattern** | Consistent UX for toggling visibility of system-managed entities |
| **Filter in tier groups, not in node rendering** | Merge task still renders as a graph node — just excluded from tier sub-group calculation to prevent phantom tiers |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Frontend - Run `npm run test`
- [ ] `tierGroupUtils.test.ts` — merge task with single-tier plan produces no tier groups
- [ ] `tierGroupUtils.test.ts` — merge task with multi-tier plan produces only real tiers

### Build Verification (run only for modified code)
- [ ] Frontend: `npm run lint && npm run typecheck` passes

### Manual Testing
- [ ] Plan with 1 proposal → apply with feature branch → graph shows flat plan group (no phantom tier)
- [ ] Plan with 3 proposals across 2 tiers → graph shows TIER 0, TIER 1 only (no TIER 2 for merge)
- [ ] Apply a plan → merge task is NOT visible in Kanban columns by default
- [ ] Toggle "Show merge tasks" → merge task appears in Blocked column
- [ ] Toggle off → merge task hidden again

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] `showMergeTasks` toggle in TaskBoard reads/writes uiStore correctly
- [ ] Column.tsx filters tasks based on `showMergeTasks` state
- [ ] Toggle only appears when merge tasks exist (count > 0)

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
