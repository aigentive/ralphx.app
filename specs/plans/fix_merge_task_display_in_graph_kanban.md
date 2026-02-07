# Plan: Fix Merge Task Display in Graph + Kanban

## Context

When a plan is applied from an ideation session, an auto-created merge task (category `plan_merge`) is added with dependencies on ALL plan tasks. This causes two problems:

1. **Graph:** Kahn's algorithm assigns the merge task to `max_tier + 1`, creating a phantom tier. Since `buildTierGroups()` only creates tier sub-groups when `tierNumbers.length > 1` (line 65), the merge task **triggers tier grouping that wouldn't otherwise happen** — showing "TIER 0 Foundation" + "TIER 1 Core" for a plan that should be flat.

2. **Kanban:** Merge tasks appear as regular task cards in the Blocked column, cluttering the board with system-managed tasks that aren't user-actionable until all plan tasks complete.

**Root cause:** Merge tasks are auto-generated housekeeping but are treated identically to user-created tasks in both views.

---

## Part 1: Graph — Exclude Merge Tasks from Tier Groups

No backend changes needed — `TaskGraphNode.category` already exists (`task-graph.types.ts:10`).

### Task 1: Filter merge tasks in `buildTierGroups()` and update tests (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `fix(graph): exclude plan_merge tasks from tier group calculation`

**File:** `src/components/TaskGraph/groups/tierGroupUtils.ts`

In `buildTierGroups()`, skip nodes with `category === "plan_merge"` when building the `tiers` map (lines 52-62):

```typescript
for (const taskId of planEntry.taskIds) {
  const node = nodeMap.get(taskId);
  if (!node) continue;
  if (node.category === "plan_merge") continue;  // ← ADD
  // ... rest of tier grouping
}
```

**Effect:** Merge tasks no longer inflate tier count. Single-tier plans stay flat. Multi-tier plans keep original structure.

**Tests** (`src/components/TaskGraph/groups/tierGroupUtils.test.ts`):
- Test: 1 real task + 1 merge task → no tier sub-groups
- Test: 2-tier tasks + merge task → only 2 tiers (merge task excluded)

---

## Part 2: Kanban — Hide Merge Tasks by Default

Follow the existing `showArchived` pattern (`src/stores/uiStore.ts`). Client-side filtering is sufficient since merge tasks are few (1 per plan).

### Task 2: Add `showMergeTasks` state to UI store, filter in Column, add toggle in TaskBoard
**Dependencies:** Task 1
**Atomic Commit:** `feat(kanban): hide merge tasks by default with toggle`

**2.1 Add UI store state**

**File:** `src/stores/uiStore.ts`
- Add `showMergeTasks: boolean` (default: `false`) to `UiState` interface
- Add `setShowMergeTasks: (show: boolean) => void` to `UiActions` interface
- Add implementation in store (same pattern as `showArchived` / `setShowArchived`)

**2.2 Filter merge tasks in Column rendering**

**File:** `src/components/tasks/TaskBoard/Column.tsx`
- Read `showMergeTasks` from UI store
- If `false`, filter out tasks where `task.category === "plan_merge"` before rendering
- Apply filter in the `tasks` useMemo (after flattening/sorting, before return)

**2.3 Add toggle in Kanban toolbar**

**File:** `src/components/tasks/TaskBoard/TaskBoard.tsx`
- Add a "Show merge tasks" toggle near the existing "Show archived tasks" toggle
- Display count of hidden merge tasks (same pattern as archived count)
- Only show toggle when merge tasks exist (count > 0)
- Use `GitMerge` icon from lucide-react (consistent with merge semantics)

---

## Files to Modify

| File | Change | Task |
|------|--------|------|
| `src/components/TaskGraph/groups/tierGroupUtils.ts` | Skip `plan_merge` nodes in `buildTierGroups()` | 1 |
| `src/components/TaskGraph/groups/tierGroupUtils.test.ts` | Add merge-task filtering tests | 1 |
| `src/stores/uiStore.ts` | Add `showMergeTasks` state + setter | 2 |
| `src/components/tasks/TaskBoard/Column.tsx` | Filter merge tasks client-side | 2 |
| `src/components/tasks/TaskBoard/TaskBoard.tsx` | Add toggle for merge task visibility | 2 |

## Merge Task Node in Graph

The merge task still renders as a node in the plan group — just not inside a tier sub-group. Since tier sub-groups are optional visual containers, the merge task appears as a flat node within its plan group.

## Compilation Unit Validation

- **Task 1:** Additive guard (`continue` in loop) + new tests. No renames, no signature changes. Compiles independently. ✅
- **Task 2:** Additive state (`showMergeTasks`) + additive consumers. All in same task to ensure store state exists when Column/TaskBoard reference it. ✅

## Verification

**Graph:**
1. Plan with 1 proposal → apply with feature branch → graph shows flat plan group (no phantom tier)
2. Plan with 3 proposals across 2 tiers → graph shows TIER 0, TIER 1 only (no TIER 2 for merge)

**Kanban:**
3. Apply a plan → merge task is NOT visible in Kanban columns by default
4. Toggle "Show merge tasks" → merge task appears in Blocked column
5. Toggle off → merge task hidden again

**General:**
6. `npm run typecheck` passes
7. `npm test -- tierGroupUtils` passes
8. `npm run lint` passes

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)
