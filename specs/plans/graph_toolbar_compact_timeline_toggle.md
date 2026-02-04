---
name: graph-toolbar-sidebar
overview: Add a compact floating graph toolbar, a graph-only right-panel toggle (Cmd+L + navbar icon), and recenter-on-selection when breakpoints or sidebar visibility change.
todos:
  - id: breakpoint-hook
    content: Add shared nav-compact breakpoint hook and use in graph view.
    status: pending
  - id: toolbar-compact
    content: Make FloatingGraphFilters icon-only in compact mode.
    status: pending
  - id: panel-toggle
    content: Add graph right-panel toggle state and auto-hide logic.
    status: pending
  - id: navbar-shortcut
    content: Add graph-only navbar icon and Cmd+L toggle.
    status: pending
  - id: recenter-on-change
    content: Recenter selection on breakpoint/panel visibility changes.
    status: pending
isProject: false
---

# Plan: Graph Toolbar Compact + Timeline Toggle

## Scope

- Add a compact (icon-only) mode to the floating graph toolbar that activates at the same breakpoint as navbar compact mode.
- Introduce a graph-only right-panel visibility toggle (Cmd+L + navbar icon) that auto-hides at the compact breakpoint and can hide the panel even when chat is showing.
- Recenter the graph on the active selection when breakpoint changes or the right panel visibility changes.

## Key files to change

- [src/components/TaskGraph/controls/FloatingGraphFilters.tsx](/Users/lazabogdan/Code/ralphx/src/components/TaskGraph/controls/FloatingGraphFilters.tsx)
- [src/components/TaskGraph/TaskGraphView.tsx](/Users/lazabogdan/Code/ralphx/src/components/TaskGraph/TaskGraphView.tsx)
- [src/components/layout/GraphSplitLayout.tsx](/Users/lazabogdan/Code/ralphx/src/components/layout/GraphSplitLayout.tsx)
- [src/hooks/useAppKeyboardShortcuts.ts](/Users/lazabogdan/Code/ralphx/src/hooks/useAppKeyboardShortcuts.ts)
- [src/App.tsx](/Users/lazabogdan/Code/ralphx/src/App.tsx)
- [src/stores/uiStore.ts](/Users/lazabogdan/Code/ralphx/src/stores/uiStore.ts)
- [src/components/TaskGraph/hooks/useGraphSelectionController.ts](/Users/lazabogdan/Code/ralphx/src/components/TaskGraph/hooks/useGraphSelectionController.ts)

## Implementation steps

### Task 1: Shared breakpoint detection (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(hooks): add useNavCompactBreakpoint hook`

- Add a small hook (e.g., `useNavCompactBreakpoint`) in [src/hooks](/Users/lazabogdan/Code/ralphx/src/hooks) that returns `isNavCompact` based on the same breakpoint used by the navbar labels (Tailwind `xl` → `min-width: 1280px`).
- Use this hook in the graph view to drive both toolbar compact mode and right-panel auto-hide.

### Task 2: Floating graph toolbar compact mode
**Dependencies:** Task 1
**Atomic Commit:** `feat(graph): add compact mode to FloatingGraphFilters`

- Extend `FloatingGraphFilters` to accept an `isCompact` prop and render icon-only buttons with tooltips (per your choice) when compact.
- Keep the same controls; only change layout/label visibility and width for compact mode.

### Task 3: Graph right-panel toggle state + auto-hide (BLOCKING)
**Dependencies:** Task 1
**Atomic Commit:** `feat(graph): add right-panel toggle state and auto-hide`

- Add UI state in [src/stores/uiStore.ts](/Users/lazabogdan/Code/ralphx/src/stores/uiStore.ts) for `graphRightPanelUserOpen` (default true) and actions to toggle/set it.
- Compute `graphRightPanelVisible = graphRightPanelUserOpen && !isNavCompact` in `TaskGraphView` so breakpoint auto-hides without losing user preference.
- Update `GraphSplitLayout` to accept a `rightPanelVisible` prop and render the right panel (and resize handle/separator) only when visible, letting the graph canvas span full width when hidden.

### Task 4: Navbar icon + Cmd+L (graph-only)
**Dependencies:** Task 3
**Atomic Commit:** `feat(graph): add navbar toggle icon and Cmd+L shortcut`

- Add an icon-only button after Reviews in [src/App.tsx](/Users/lazabogdan/Code/ralphx/src/App.tsx). Render it only when `currentView === "graph"`; tooltip shows `⌘L`.
- Add `Cmd+L` handling in [src/hooks/useAppKeyboardShortcuts.ts](/Users/lazabogdan/Code/ralphx/src/hooks/useAppKeyboardShortcuts.ts) and wire to a new `toggleGraphRightPanel` callback passed from `App` (guarded by `currentView === "graph"` and input focus rules).

### Task 5: Recenter on selection when layout changes
**Dependencies:** Task 1, Task 3
**Atomic Commit:** `feat(graph): recenter selection on layout changes`

- Expose a `focusSelectionInView` (or `recenterSelection`) function from `useGraphSelectionController` so `TaskGraphView` can call it on breakpoint changes and right-panel visibility toggles.
- Add an effect in `TaskGraphView` to call recenter when `graphRightPanelVisible` or `isNavCompact` changes and a selection exists.

## Tests

- Add/adjust tests to cover:
- `Cmd+L` only toggles on graph view and ignores focused inputs.
- Graph right panel visibility respects breakpoint auto-hide and user toggle.
- Floating toolbar renders icon-only in compact mode (snapshot or DOM assertions).

## Notes

- This plan keeps the compact breakpoint aligned with the existing navbar behavior (Tailwind `xl`).
- The right panel can be hidden even when chat is showing, as requested.

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)

### Task Dependency Graph

```
Task 1 (breakpoint hook)
   ├──→ Task 2 (toolbar compact)
   ├──→ Task 3 (panel toggle state) ──→ Task 4 (navbar icon + Cmd+L)
   └──→ Task 5 (recenter on change)
        ↑
Task 3 ─┘
```

**Parallelization opportunity:** Tasks 2 and 3 can run in parallel after Task 1 completes.