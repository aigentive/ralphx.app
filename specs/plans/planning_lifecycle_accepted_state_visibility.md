# Plan: Planning Lifecycle & Accepted State Visibility

## Terminology

| Old | New | Rationale |
|-----|-----|-----------|
| Ideation | **Planning** | Clearer intent |
| Session | **Plan** | Simple, universal |
| Apply | **Accept** | Implies approval/commitment |
| Converted | **Accepted** | Matches action |

## Mental Model

**Plan = Complete work scope** with dependency graph. Accept all or nothing.

1. **Active**: User creates proposals, arranges dependencies, refines
2. **Accept**: Transition entire plan to tasks (preserves dependency graph)
3. **Accepted**: Historical record, links to created tasks, read-only chat
4. **New work = New plan**: Don't reuse accepted plans

The goal is to **refine before execution**, not after. The dependency graph and proposal relationships are the plan's structure - they should transfer intact.

## Current Behavior (Problem)

- User can select individual proposals and apply them
- Creates weird partial states (some applied, some not)
- Dependency graph can break if dependent proposal isn't applied
- Confusing UX - what's left to do?
- No way to view accepted plans

## Proposed Behavior

- **Accept Plan** button (not per-proposal selection)
- Button **blocked** until dependency graph is complete (visual feedback)
- Accepts ALL proposals at once with full dependency graph
- Plan transitions to "Accepted" immediately
- Can still chat in accepted plans (read-only MCP access)
- Clear before/after: Active → Accepted

## Problems to Solve

1. **Plan disappears**: Accepted plans vanish from UI
2. **No feedback after accept**: Can't see what became tasks
3. **Partial apply complexity**: Current UX allows confusing states
4. **Worker lacks context**: Can't see dependencies/tier data

## Solution Overview

### Part 1: Accept Entire Plan (Behavior Change)

Replace per-proposal selection with plan-level accept:
- Single **"Accept Plan"** button in toolbar
- **Blocked** until dependency graph is complete (track & display state)
- Accepts all proposals with dependency graph
- Confirmation modal showing what will be created
- Plan immediately becomes "Accepted"

### Part 2: History Section in PlanBrowser

Add collapsible "History" section showing accepted + archived plans:
- Collapsed by default
- Status badges: "Accepted" / "Archived"
- Click to view plan (read-only mode)

### Part 3: Accepted Plan View

When viewing an accepted plan:
- All proposals show links to their created tasks
- No edit/delete/accept actions
- **Chat still works** but with read-only MCP access
- Clear visual indication this is historical record

### Part 4: Read-only Chat in Accepted Plans

Allow continued conversation but block modifications:
- Option A: Block update tool calls at MCP/HTTP server level
- Option B: Customize allowed tools list for Claude CLI in this scenario
- Either way: read access to what happened, no modify access

---

## Changes

### Task 0: Terminology Migration (Converted → Accepted) (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(ideation): rename Converted status to Accepted with migration`

Update status enum across backend and frontend + database migration:
- `src-tauri/src/domain/entities/ideation/types.rs` - Rename `Converted` → `Accepted` in enum
- `src/types/ideation.ts` - Update status values: "converted" → "accepted"
- Migration - Update existing "converted" rows to "accepted"

This MUST be a single task because:
- Backend enum rename without frontend update → type mismatch
- Frontend type update without migration → existing data broken

---

### Task 1: ProposalsToolbar.tsx - Accept Plan Button
**Dependencies:** Task 0
**Atomic Commit:** `feat(ideation): replace per-proposal selection with Accept Plan button`

Remove per-proposal selection. Single "Accept Plan" button with graph validation:

```tsx
// Track dependency graph completeness
const isGraphComplete = useDependencyGraphComplete(proposals);

<Button
  onClick={onAcceptPlan}
  disabled={proposals.length === 0 || isReadOnly || !isGraphComplete}
>
  <Check size={16} />
  Accept Plan ({proposals.length} tasks)
</Button>

// Visual feedback when blocked
{!isGraphComplete && (
  <Tooltip content="Complete dependency graph before accepting">
    <AlertCircle className="text-warning" />
  </Tooltip>
)}
```

### Task 2: PlanBrowser.tsx (rename from SessionBrowser) (BLOCKING)
**Dependencies:** Task 0
**Atomic Commit:** `refactor(ideation): rename SessionBrowser to PlanBrowser with history section`

Rename file and update all imports + add history section:

```tsx
interface PlanBrowserProps {
  plans: Plan[];              // Active plans
  historyPlans: Plan[];       // Accepted + Archived
  // ... rest unchanged
}

// After active plans list, add collapsible History
<Collapsible defaultOpen={false}>
  <CollapsibleTrigger>
    <History size={14} />
    History ({historyPlans.length})
  </CollapsibleTrigger>
  <CollapsibleContent>
    {historyPlans.map(plan => (
      <PlanItem
        plan={plan}
        isHistory={true}  // No context menu
        statusBadge={plan.status}  // "accepted" | "archived"
      />
    ))}
  </CollapsibleContent>
</Collapsible>
```

### Task 3: PlanningView.tsx (rename from IdeationView) (BLOCKING)
**Dependencies:** Task 2
**Atomic Commit:** `refactor(ideation): rename IdeationView to PlanningView with read-only support`

Rename file and update all imports + add history/read-only logic:

```tsx
const activePlans = useMemo(
  () => plans.filter((p) => p.status === "active"),
  [plans]
);

const historyPlans = useMemo(
  () => plans.filter((p) => p.status !== "active"),
  [plans]
);

const isReadOnly = plan?.status !== "active";

<PlanBrowser
  plans={activePlans}
  historyPlans={historyPlans}
  ...
/>

<ProposalsToolbar isReadOnly={isReadOnly} ... />
```

### Task 4: ProposalCard.tsx - Show Task Link (Accepted Plans)
**Dependencies:** Task 0
**Atomic Commit:** `feat(ideation): add task link to proposals in accepted plans`

When viewing accepted plan, proposals show link to created task:

```tsx
{proposal.createdTaskId && (
  <Button variant="ghost" size="sm" onClick={() => navigateToTask(proposal.createdTaskId)}>
    View Task →
  </Button>
)}

// In accepted plans: no edit/delete menu
```

### Task 5: AcceptModal.tsx (rename from ApplyModal)
**Dependencies:** Task 0
**Atomic Commit:** `refactor(ideation): rename ApplyModal to AcceptModal with full plan accept`

Rename file and update all imports + update modal logic:
- Remove proposal checkboxes
- Show all proposals that will become tasks
- Keep dependency graph preview
- Keep target column selector

### Task 6: Remove Redundant Sort Control
**Dependencies:** Task 1
**Atomic Commit:** `refactor(ideation): remove sort dropdown, tier order is canonical`

The TieredProposalList displays proposals in tier hierarchy (based on dependency graph scoring). Tier order IS the canonical order.

- Remove sort dropdown from toolbar
- Remove selection checkboxes from TieredProposalList
- Keep filter by status if useful, but sorting is implicit

### Task 7: Read-only Chat for Accepted Plans
**Dependencies:** Task 0
**Atomic Commit:** `feat(plugin): add read-only chat mode for accepted plans`

Allow chat to continue but block modifications via **CLI allowed-tools**:

- When spawning Claude CLI for accepted plan chat, pass `--allowed-tools` flag
- Exclude: `update_proposal`, `remove_proposal`, `add_proposal`, `create_proposal`, etc.
- Include: `get_*` tools for read access (get_proposal, get_plan, etc.)

**Implementation:**
- Modify chat spawn logic to check plan status
- If accepted: add `--allowed-tools` with read-only list
- If active: normal full access

### Task 8: Expose Dependency Context to Worker (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(backend): add dependency context to TaskContext for worker`

When worker calls `get_task_context`, include:
- `blocked_by: Task[]` - tasks that must complete before this one
- `blocks: Task[]` - tasks waiting on this one
- `tier: u32` - execution tier (from dependency scoring)
- `priority_score: i32` - numerical priority (0-100)

**Two levels of ordering (important distinction):**
- **Tier/Dep graph**: Order of TASKS (which task to work on first) - cross-task
- **Task steps**: Order within ONE task (step 1, step 2, etc.) - within-task

Worker needs both. Dep context says "don't start Task B until Task A done". Steps say "within Task A, do these steps in order".

**Changes needed:**
1. **TaskContext entity** (`task_context.rs`) - add dependency fields
2. **TaskContextService** (`task_context_service.rs`) - query dependencies
3. **TaskProposalSummary** - include priority_score
4. **Worker agent prompt** (`ralphx-plugin/agents/worker.md`) - add guidance on using dep context

### Task 9: Update Worker Agent Prompt
**Dependencies:** Task 8
**Atomic Commit:** `docs(plugin): add dependency context guidance to worker agent`

Add section to `ralphx-plugin/agents/worker.md` explaining how to use dependency context:

```markdown
## Task Dependencies

When you call `get_task_context`, check the dependency information:

- **blocked_by**: Tasks that must complete BEFORE you can start this task
  - If not empty: STOP. Do not proceed. Report "blocked by [task names]"
- **blocks**: Tasks waiting for THIS task to complete
  - For context: your work unblocks these downstream tasks
- **tier**: Execution tier (lower = earlier in dependency chain)
  - Tier 1 tasks have no blockers
  - Higher tiers depend on lower tiers

### Decision Flow
1. Call `get_task_context`
2. Check `blocked_by`:
   - If NOT empty → Cannot proceed, task is blocked
   - If empty → Proceed with execution
3. Use `tier` to understand priority context
4. Work through task `steps` in order
```

### Task 10: Track Dependency Graph Completeness (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(hooks): add useDependencyGraphComplete hook`

Add hook/state to track if dependency graph is complete:
- All proposals have tier assigned
- No orphan proposals (unless explicitly marked as independent)
- Dependencies form valid DAG (no cycles)

```tsx
// New hook
function useDependencyGraphComplete(proposals: Proposal[]): boolean {
  // Check all proposals have valid tier
  // Check no dangling dependencies
  // Return true if ready to accept
}
```

### Task 11: Just-in-Time Execution Orchestration (BLOCKING)
**Dependencies:** Task 8
**Atomic Commit:** `feat(backend): add automatic blocking/unblocking based on dependency graph`

**Current state:** `InternalStatus::Blocked` exists but not integrated with task dependencies. Blocking is manual.

**New behavior:** Automatic blocking/unblocking based on dependency graph.

#### On Plan Accept:
```rust
for task in created_tasks {
  let blockers = get_blocking_tasks(task.id);
  if blockers.is_empty() {
    task.internal_status = Ready;  // Tier 1 - no blockers
  } else {
    task.internal_status = Blocked;
    task.blocked_reason = format!("Waiting for: {}", blocker_names.join(", "));
  }
}
```

#### On Task Completion:
```rust
fn on_task_completed(task_id: TaskId) {
  // Find tasks this one was blocking
  let unblocked = get_tasks_blocked_by(task_id);

  for task in unblocked {
    // Check if ALL blockers are now complete
    let remaining_blockers = get_incomplete_blockers(task.id);
    if remaining_blockers.is_empty() {
      task.internal_status = Ready;
      task.blocked_reason = None;
      emit_event("task:unblocked", task.id);
    } else {
      // Update reason with remaining blockers
      task.blocked_reason = format!("Waiting for: {}", remaining_blocker_names.join(", "));
    }
  }
}
```

#### Integration Points:
1. **ApplyService** (`apply_service.rs`) - set initial Blocked/Ready based on deps
2. **TransitionHandler** (`transition_handler.rs`) - on Done transition, check/unblock dependents
3. **TaskContext** - include blocker info for worker visibility

#### UI Updates:
- Blocked tasks show "Waiting for: [task names]" in card
- When auto-unblocked, task moves from Blocked group → Fresh Tasks group
- Event triggers toast: "Task X is now ready"

---

## Files to Modify

### Frontend (UI Changes)

| File | Change |
|------|--------|
| `src/components/Ideation/ProposalsToolbar.tsx` | Remove selection & sort, "Accept Plan" button with graph validation |
| `src/components/Ideation/SessionBrowser.tsx` → `PlanBrowser.tsx` | Rename, add `historyPlans` prop, collapsible History section |
| `src/components/Ideation/IdeationView.tsx` → `PlanningView.tsx` | Rename, compute history plans, pass `isReadOnly` flag |
| `src/components/Ideation/ProposalCard.tsx` | Add task link for accepted, disable actions when accepted |
| `src/components/Ideation/ApplyModal.tsx` → `AcceptModal.tsx` | Rename, remove selection, show full plan accept |
| `src/components/Ideation/TieredProposalList.tsx` | Remove selection checkboxes |
| `src/hooks/useDependencyGraphComplete.ts` | New hook to track graph completeness |

### Backend (Worker Context + Orchestration)

| File | Change |
|------|--------|
| `src-tauri/src/domain/entities/task_context.rs` | Add `blocked_by`, `blocks`, `tier` fields |
| `src-tauri/src/application/task_context_service.rs` | Query task dependencies, compute tier |
| `src-tauri/src/domain/entities/ideation/proposal.rs` | Add priority_score to TaskProposalSummary |
| `src-tauri/src/application/apply_service.rs` | Set initial Blocked/Ready based on deps on accept |
| `src-tauri/src/domain/services/transition_handler.rs` | On Done, check/unblock dependent tasks |

### Plugin (Worker + Chat)

| File | Change |
|------|--------|
| `ralphx-plugin/agents/worker.md` | Add dependency context usage guidance |
| Chat spawn logic | Pass `--allowed-tools` for accepted plan chats |

### Database/Types (Terminology)

| File | Change |
|------|--------|
| `src-tauri/src/domain/entities/ideation/types.rs` | Rename `Converted` → `Accepted` in enum |
| `src/types/ideation.ts` | Update status values: "converted" → "accepted" |
| Migration | Update existing "converted" rows to "accepted" |

---

## UI Behavior Summary

| State | Plan List | Proposals | Chat | Actions |
|-------|-----------|-----------|------|---------|
| **Active** | Main list | Full edit | Full access | "Accept Plan" (blocked until graph complete) |
| **Accepted** | History section | Show task links | Read-only MCP | View only |
| **Archived** | History section | As they were | Read-only MCP | View only |

---

## Verification

### UI Flow
1. Create plan with 3 proposals + dependencies
2. "Accept Plan" button is **disabled** (graph incomplete)
3. Complete dependency graph → button becomes enabled
4. Click "Accept Plan" → confirmation shows all 3 proposals (no selection)
5. Confirm → all 3 become tasks with preserved dependencies
6. Plan immediately moves to History section (status: accepted)
7. Click plan in History → view read-only with task links
8. Chat still works but modify tools are blocked
9. Click "View Task" → navigates to task in kanban
10. Archive an active plan → appears in History with "Archived" badge

### Worker Context
11. Move a created task to "Executing" state
12. Run worker agent via plugin
13. Worker calls `get_task_context` → verify response includes:
    - `blocked_by: []` or list of blocking tasks
    - `blocks: []` or list of dependent tasks
    - `tier: 1` (or computed tier from dependency graph)
14. Worker can see what must complete before it and what depends on it

### Read-only Chat
15. Open accepted plan
16. Send message in chat
17. Try to use modify tool (e.g., update_proposal) → should be blocked
18. Use read tool (e.g., get_proposal) → should work

### Just-in-Time Orchestration
19. Accept plan with 3 tasks: A (tier 1), B blocked by A (tier 2), C blocked by B (tier 3)
20. A → Ready, B → Blocked ("Waiting for: A"), C → Blocked ("Waiting for: A, B")
21. Complete task A → B auto-transitions to Ready, C updates reason ("Waiting for: B")
22. Complete task B → C auto-transitions to Ready
23. UI shows tasks moving from Blocked group → Fresh Tasks group
24. Toast notifications: "Task B is now ready", "Task C is now ready"

---

## Task Dependency Graph

```
Task 0: Terminology Migration (BLOCKING)
├── Task 1: ProposalsToolbar Accept Button
│   └── Task 6: Remove Sort Control
├── Task 2: PlanBrowser (rename)
│   └── Task 3: PlanningView (rename)
├── Task 4: ProposalCard Task Link
├── Task 5: AcceptModal (rename)
└── Task 7: Read-only Chat

Task 8: Expose Dependency Context (BLOCKING)
├── Task 9: Update Worker Agent Prompt
└── Task 11: Just-in-Time Orchestration

Task 10: useDependencyGraphComplete Hook (BLOCKING)
└── (consumed by Task 1)
```

**Parallel Execution Groups:**
- Group A (Backend foundation): Task 0, Task 8, Task 10 - can run in parallel
- Group B (Frontend UI): Tasks 1-7 - after Task 0 completes
- Group C (Backend orchestration): Task 11 - after Task 8 completes
- Group D (Plugin): Task 9 - after Task 8 completes

---

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)

### Compilation Unit Notes for This Plan

1. **Task 0 is critical** - terminology change spans backend enum + frontend type + migration. These CANNOT be split.

2. **File renames (Tasks 2, 3, 5)** - each rename must include updating all imports in the same commit:
   - SessionBrowser → PlanBrowser: grep for imports, update all
   - IdeationView → PlanningView: grep for imports, update all
   - ApplyModal → AcceptModal: grep for imports, update all

3. **Tasks 8 + 11 share integration points** but are separate compilation units:
   - Task 8 adds new fields to TaskContext (additive)
   - Task 11 wires orchestration using those fields (consumes Task 8)
