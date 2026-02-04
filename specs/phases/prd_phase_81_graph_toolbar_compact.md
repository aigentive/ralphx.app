# RalphX - Phase 81: Graph Toolbar Compact + Timeline Toggle

## Overview

This phase adds responsive behavior to the graph view's floating toolbar and right panel. When the viewport narrows (at the same breakpoint as navbar compact mode), the toolbar switches to icon-only mode and the right panel auto-hides. Users can manually toggle the right panel visibility via a navbar icon or Cmd+L keyboard shortcut. The graph automatically recenters on the active selection when layout changes occur.

**Reference Plan:**
- `specs/plans/graph_toolbar_compact_timeline_toggle.md` - Detailed implementation plan with dependency graph and task specifications

## Goals

1. Add compact (icon-only) mode to floating graph toolbar at navbar breakpoint
2. Implement graph right-panel toggle with auto-hide at compact breakpoint
3. Add Cmd+L keyboard shortcut and navbar icon for panel toggle (graph-only)
4. Recenter graph selection when breakpoint or panel visibility changes

## Dependencies

### Phase 73 (Graph Page Layout Parity with Kanban) - Required

| Dependency | Why Needed |
|------------|------------|
| `GraphSplitLayout` | Base layout component to extend with panel visibility prop |
| `TaskGraphView` | Container component that will orchestrate breakpoint detection |
| `FloatingGraphFilters` | Toolbar component to extend with compact mode |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/graph_toolbar_compact_timeline_toggle.md`
2. Understand the task dependency graph (Task 1 blocks 2, 3; Task 3 blocks 4)
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
- Task 1 must complete first (no dependencies)
- Tasks 2 and 3 can run in parallel after Task 1
- Task 4 requires Task 3
- Task 5 requires Tasks 1 and 3

---

## Task List

**IMPORTANT: Work on ONE task per iteration.**

**BEFORE STARTING:**
1. Find the first task with `"passes": false`
2. **Read the ENTIRE implementation plan** at `specs/plans/graph_toolbar_compact_timeline_toggle.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "frontend",
    "description": "Add shared useNavCompactBreakpoint hook for responsive breakpoint detection",
    "plan_section": "Task 1: Shared breakpoint detection",
    "blocking": [2, 3, 5],
    "blockedBy": [],
    "atomic_commit": "feat(hooks): add useNavCompactBreakpoint hook",
    "steps": [
      "Read specs/plans/graph_toolbar_compact_timeline_toggle.md section 'Task 1'",
      "Create src/hooks/useNavCompactBreakpoint.ts with isNavCompact return value",
      "Use Tailwind xl breakpoint (min-width: 1280px) for detection",
      "Export from src/hooks/index.ts",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(hooks): add useNavCompactBreakpoint hook"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "frontend",
    "description": "Add compact mode to FloatingGraphFilters with icon-only buttons",
    "plan_section": "Task 2: Floating graph toolbar compact mode",
    "blocking": [],
    "blockedBy": [1],
    "atomic_commit": "feat(graph): add compact mode to FloatingGraphFilters",
    "steps": [
      "Read specs/plans/graph_toolbar_compact_timeline_toggle.md section 'Task 2'",
      "Add isCompact prop to FloatingGraphFilters component",
      "Render icon-only buttons with tooltips when isCompact is true",
      "Adjust width/layout for compact mode while keeping all controls",
      "Wire useNavCompactBreakpoint in TaskGraphView to pass isCompact prop",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(graph): add compact mode to FloatingGraphFilters"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "frontend",
    "description": "Add graph right-panel toggle state with auto-hide at compact breakpoint",
    "plan_section": "Task 3: Graph right-panel toggle state + auto-hide",
    "blocking": [4, 5],
    "blockedBy": [1],
    "atomic_commit": "feat(graph): add right-panel toggle state and auto-hide",
    "steps": [
      "Read specs/plans/graph_toolbar_compact_timeline_toggle.md section 'Task 3'",
      "Add graphRightPanelUserOpen state (default true) to uiStore.ts with toggle/set actions",
      "Compute graphRightPanelVisible = graphRightPanelUserOpen && !isNavCompact in TaskGraphView",
      "Add rightPanelVisible prop to GraphSplitLayout",
      "Hide right panel and resize handle when rightPanelVisible is false",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(graph): add right-panel toggle state and auto-hide"
    ],
    "passes": false
  },
  {
    "id": 4,
    "category": "frontend",
    "description": "Add navbar toggle icon and Cmd+L keyboard shortcut for graph panel",
    "plan_section": "Task 4: Navbar icon + Cmd+L (graph-only)",
    "blocking": [],
    "blockedBy": [3],
    "atomic_commit": "feat(graph): add navbar toggle icon and Cmd+L shortcut",
    "steps": [
      "Read specs/plans/graph_toolbar_compact_timeline_toggle.md section 'Task 4'",
      "Add icon-only button after Reviews in App.tsx, visible only when currentView === 'graph'",
      "Add tooltip showing ⌘L for the button",
      "Add Cmd+L handler in useAppKeyboardShortcuts.ts",
      "Wire toggleGraphRightPanel callback from App, guarded by currentView === 'graph' and input focus",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(graph): add navbar toggle icon and Cmd+L shortcut"
    ],
    "passes": false
  },
  {
    "id": 5,
    "category": "frontend",
    "description": "Recenter graph selection when layout changes occur",
    "plan_section": "Task 5: Recenter on selection when layout changes",
    "blocking": [],
    "blockedBy": [1, 3],
    "atomic_commit": "feat(graph): recenter selection on layout changes",
    "steps": [
      "Read specs/plans/graph_toolbar_compact_timeline_toggle.md section 'Task 5'",
      "Expose focusSelectionInView or recenterSelection function from useGraphSelectionController",
      "Add effect in TaskGraphView to call recenter when graphRightPanelVisible or isNavCompact changes",
      "Only recenter when a selection exists",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(graph): recenter selection on layout changes"
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
| **Shared breakpoint hook** | Single source of truth for xl breakpoint detection, reusable across navbar and graph |
| **User preference + auto-hide composition** | `visible = userOpen && !isCompact` preserves user choice across breakpoint changes |
| **Graph-only Cmd+L** | Avoids conflicts with other views' keyboard shortcuts |
| **Recenter on layout change** | Prevents selection from being hidden when panel collapses/expands |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Frontend - Run `npm run test`
- [ ] useNavCompactBreakpoint returns correct value at breakpoint boundary
- [ ] FloatingGraphFilters renders icon-only in compact mode
- [ ] Cmd+L only fires when graph view is active

### Build Verification (run only for modified code)
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`npm run build`)

### Manual Testing
- [ ] Resize window below xl breakpoint → toolbar becomes icon-only
- [ ] Resize window below xl breakpoint → right panel auto-hides
- [ ] Click navbar icon → panel toggles (when in graph view)
- [ ] Press Cmd+L → panel toggles (when in graph view, no input focused)
- [ ] Cmd+L does nothing when not on graph view
- [ ] Select a task, toggle panel → graph recenters to keep selection visible
- [ ] Select a task, resize past breakpoint → graph recenters

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] useNavCompactBreakpoint hook is imported and used in TaskGraphView
- [ ] FloatingGraphFilters receives isCompact prop and renders conditionally
- [ ] GraphSplitLayout receives rightPanelVisible prop and hides panel correctly
- [ ] Navbar icon renders only when currentView === "graph"
- [ ] Cmd+L handler calls uiStore.toggleGraphRightPanel

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
