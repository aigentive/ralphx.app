# RalphX - Phase 73: Graph Page Layout Parity with Kanban

## Overview

Transform the Graph page to use a split-screen layout matching the Kanban page, with floating timeline sidebar, task detail + chat integration, execution control bar, and floating filter controls. This creates visual and functional parity between the two main orchestration views.

**Reference Plan:**
- `specs/plans/graph_page_layout_parity_with_kanban.md` - Detailed implementation plan with component specs and styling

## Goals

1. Create split-screen layout for Graph page matching Kanban's structure
2. Replace top GraphControls bar with floating filter panel over canvas
3. Add ExecutionControlBar footer to Graph page
4. Unify right panel width constraints (25-35%) across both views
5. Reorder navbar to reflect natural workflow: Ideation → Graph → Kanban

## Dependencies

### Phase 72 (Graph Node Kanban Styling Parity) - Required

| Dependency | Why Needed |
|------------|------------|
| TaskNode styling | Graph nodes must be styled before layout changes |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/graph_page_layout_parity_with_kanban.md`
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
Task 1 (GraphSplitLayout) ─┐
Task 2 (FloatingTimeline) ─┼─→ Task 4 (Refactor TaskGraphView) ─→ Task 5 (Wire App.tsx)
Task 3 (FloatingFilters) ──┘

Task 6 (KanbanSplitLayout) ─→ (independent)
Task 7 (Navbar reorder) ────→ (independent)
```

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/graph_page_layout_parity_with_kanban.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "frontend",
    "description": "Create GraphSplitLayout component with resizable panels and task detail integration",
    "plan_section": "Task 1: Create GraphSplitLayout Component",
    "blocking": [4],
    "blockedBy": [],
    "atomic_commit": "feat(layout): create GraphSplitLayout component",
    "steps": [
      "Read specs/plans/graph_page_layout_parity_with_kanban.md section 'Task 1'",
      "Create src/components/layout/GraphSplitLayout.tsx (~160 LOC)",
      "Implement GraphSplitLayoutProps interface with children, projectId, footer, timelineContent",
      "Set width constraints: MIN_LEFT_PERCENT=65, MAX_LEFT_PERCENT=75, DEFAULT_LEFT_PERCENT=70",
      "Implement right panel content switching based on selectedTaskId (timeline ↔ chat)",
      "Add TaskDetailOverlay rendering inside left section",
      "Add resize handle between panels",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(layout): create GraphSplitLayout component"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "frontend",
    "description": "Create FloatingTimeline wrapper with Tahoe glass styling",
    "plan_section": "Task 2: Create FloatingTimeline Wrapper",
    "blocking": [4],
    "blockedBy": [],
    "atomic_commit": "feat(graph): create FloatingTimeline wrapper component",
    "steps": [
      "Read specs/plans/graph_page_layout_parity_with_kanban.md section 'Task 2'",
      "Create src/components/TaskGraph/timeline/FloatingTimeline.tsx (~40 LOC)",
      "Wrap ExecutionTimeline content in glass container styling",
      "Apply Tahoe glass styling: borderRadius 10px, blur 20px, saturate 180%",
      "Update ExecutionTimeline to expose composable core content (no outer width/collapse logic)",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(graph): create FloatingTimeline wrapper component"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "frontend",
    "description": "Create FloatingGraphFilters component with stacked filter controls",
    "plan_section": "Task 3: Create FloatingGraphFilters Component",
    "blocking": [4],
    "blockedBy": [],
    "atomic_commit": "feat(graph): create FloatingGraphFilters component",
    "steps": [
      "Read specs/plans/graph_page_layout_parity_with_kanban.md section 'Task 3'",
      "Create src/components/TaskGraph/controls/FloatingGraphFilters.tsx (~200 LOC)",
      "Position absolute: left 16px, top 50%, transform translateY(-50%)",
      "Implement stacked layout: Status Filter, Plan Filter, Layout TB/LR, Mode Std/Cpt, Grouping dropdown",
      "Apply Tahoe glass container styling",
      "Use compact icons with tooltips for space efficiency",
      "Implement FloatingGraphFiltersProps interface matching GraphControls props",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(graph): create FloatingGraphFilters component"
    ],
    "passes": true
  },
  {
    "id": 4,
    "category": "frontend",
    "description": "Refactor TaskGraphView to use GraphSplitLayout with floating controls",
    "plan_section": "Task 4: Refactor TaskGraphView for New Layout",
    "blocking": [5],
    "blockedBy": [1, 2, 3],
    "atomic_commit": "refactor(graph): integrate split layout with floating controls",
    "steps": [
      "Read specs/plans/graph_page_layout_parity_with_kanban.md section 'Task 4'",
      "Add footer prop to TaskGraphViewProps interface",
      "Remove GraphControls component from JSX",
      "Remove ExecutionTimeline from main flex layout",
      "Wrap ReactFlow canvas in GraphSplitLayout",
      "Add FloatingGraphFilters as absolute-positioned overlay inside canvas container",
      "Pass FloatingTimeline as timelineContent prop",
      "Keep all existing state (filters, layoutDirection, grouping, nodeMode)",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(graph): integrate split layout with floating controls"
    ],
    "passes": true
  },
  {
    "id": 5,
    "category": "frontend",
    "description": "Wire ExecutionControlBar to TaskGraphView in App.tsx",
    "plan_section": "Task 5: Wire in App.tsx",
    "blocking": [],
    "blockedBy": [4],
    "atomic_commit": "feat(app): wire ExecutionControlBar to Graph page",
    "steps": [
      "Read specs/plans/graph_page_layout_parity_with_kanban.md section 'Task 5'",
      "Update TaskGraphView usage in App.tsx to pass footer prop",
      "Pass ExecutionControlBar with all required props (runningCount, maxConcurrent, queuedCount, isPaused, isLoading, onPauseToggle, onStop)",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(app): wire ExecutionControlBar to Graph page"
    ],
    "passes": true
  },
  {
    "id": 6,
    "category": "frontend",
    "description": "Update KanbanSplitLayout width constraints to 25-35% right panel",
    "plan_section": "Task 6: Update KanbanSplitLayout Width Constraints",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "refactor(layout): narrow Kanban right panel width constraints",
    "steps": [
      "Read specs/plans/graph_page_layout_parity_with_kanban.md section 'Task 6'",
      "Update src/components/layout/KanbanSplitLayout.tsx constants:",
      "Change MIN_LEFT_PERCENT from 40 to 65",
      "Change DEFAULT_LEFT_PERCENT from 75 to 70",
      "Keep MAX_LEFT_PERCENT at 75",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(layout): narrow Kanban right panel width constraints"
    ],
    "passes": false
  },
  {
    "id": 7,
    "category": "frontend",
    "description": "Reorder navbar items to Ideation → Graph → Kanban",
    "plan_section": "Task 7: Reorder Navbar Items",
    "blocking": [],
    "blockedBy": [],
    "atomic_commit": "refactor(navbar): reorder to Ideation → Graph → Kanban",
    "steps": [
      "Read specs/plans/graph_page_layout_parity_with_kanban.md section 'Task 7'",
      "Locate navbar component (likely in App.tsx or dedicated component)",
      "Reorder main navigation items: Ideation → Graph → Kanban",
      "This reflects natural workflow: plan ideas → visualize dependencies → execute tasks",
      "Run npm run lint && npm run typecheck",
      "Commit: refactor(navbar): reorder to Ideation → Graph → Kanban"
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
| **Right panel always visible on Graph** | Timeline provides critical execution context; chat appears when task selected |
| **25-35% right panel width** | Narrower than original Kanban (25-60%) for more canvas/board space |
| **Floating filters over canvas** | Removes top bar, maximizes vertical space for graph visualization |
| **Shared width constraints** | Visual consistency between Graph and Kanban split layouts |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Frontend - Run `npm run test`
- [ ] GraphSplitLayout renders with correct panel proportions
- [ ] FloatingTimeline displays with glass styling
- [ ] FloatingGraphFilters all controls functional

### Build Verification (run only for modified code)
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`npm run build`)

### Manual Testing
- [ ] Graph page: Timeline visible on right (floating, rounded glass container)
- [ ] Graph page: Click task node → Task detail appears, chat opens on right, timeline hides
- [ ] Graph page: Close task → Timeline reappears
- [ ] Graph page: Floating filters work on left side of canvas
- [ ] Graph page: Execution bar at bottom
- [ ] Graph page: Keyboard navigation still works (arrow keys, Enter, Escape)
- [ ] Graph page: Resize handle works (25-35% right panel range)
- [ ] Kanban page: Right panel resize respects new 25-35% limits
- [ ] Navbar: Order is Ideation → Graph → Kanban

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] GraphSplitLayout imported AND rendered in TaskGraphView
- [ ] FloatingTimeline imported AND rendered via timelineContent prop
- [ ] FloatingGraphFilters imported AND rendered inside canvas container
- [ ] ExecutionControlBar passed to TaskGraphView footer prop

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.

### Visual Verification
- [ ] Screenshot before/after
- [ ] Tahoe liquid glass styling consistency
- [ ] Accent color (#ff6b35) and SF Pro font
