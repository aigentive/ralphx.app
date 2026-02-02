# Plan: Graph Page Layout Parity with Kanban

## Summary

Transform the Graph page to use a split-screen layout matching the Kanban page, with floating timeline sidebar, task detail + chat integration, execution control bar, and floating filter controls.

## Current State

### Graph Page (`src/components/TaskGraph/TaskGraphView.tsx`)
```
┌─────────────────────────────────────────────────────────┐
│ GraphControls (status, plans, layout, mode, grouping)   │
├─────────────────────────────────────┬───────────────────┤
│                                     │ ExecutionTimeline │
│          ReactFlow Canvas           │   (320px/40px)    │
│                                     │                   │
└─────────────────────────────────────┴───────────────────┘
+ TaskDetailOverlay (modal overlay when task selected)
```

### Kanban Page (`src/components/layout/KanbanSplitLayout.tsx`)
```
┌─────────────────────────────────────┬───────────────────┐
│                                     │                   │
│  TaskBoard + TaskDetailOverlay      │  IntegratedChat   │
│           (40-75%)                  │    (25-60%)       │
│                                     │                   │
├─────────────────────────────────────┴───────────────────┤
│              ExecutionControlBar (footer)               │
└─────────────────────────────────────────────────────────┘
```

## Target State

```
┌─────────────────────────────────────┬───────────────────┐
│                                     │                   │
│ [Floating         ReactFlow         │ FloatingTimeline  │ ← No task selected
│  Filters]         Canvas            │  (glass panel)    │
│ (left,                              ├───────────────────┤
│  stacked)                           │ IntegratedChat    │ ← Task selected (replaces timeline)
│                                     │                   │
├─────────────────────────────────────┴───────────────────┤
│              ExecutionControlBar (footer)               │
└─────────────────────────────────────────────────────────┘
```

**Key behaviors:**
1. **No task selected:** Timeline visible in right panel (floating, rounded glass container)
2. **Task selected:** TaskDetailOverlay on left (over canvas), Chat on right (replaces timeline)
3. **Filters:** Floating glass menu, left side of canvas, stacked vertically, centered horizontally
4. **Execution bar:** Fixed at bottom with same Tahoe glass styling

## Implementation Plan

### Task 1: Create GraphSplitLayout Component (BLOCKING)

**Dependencies:** None
**Atomic Commit:** `feat(layout): create GraphSplitLayout component`

**File:** `src/components/layout/GraphSplitLayout.tsx` (new, ~160 LOC)

**Structure:**
```tsx
interface GraphSplitLayoutProps {
  children: React.ReactNode;  // ReactFlow canvas
  projectId: string;
  footer?: React.ReactNode;   // ExecutionControlBar
  timelineContent: React.ReactNode;  // FloatingTimeline
}
```

**Width constraints (same as updated Kanban):**
```tsx
const MIN_LEFT_PERCENT = 65;  // 65% left = 35% right (max right panel)
const MAX_LEFT_PERCENT = 75;  // 75% left = 25% right (min right panel)
const DEFAULT_LEFT_PERCENT = 70;  // ~30% right panel
```

**Behavior:**
- Right panel always visible (no toggle like Kanban)
- Right panel content switches based on `selectedTaskId`:
  - `null` → Show `timelineContent` (FloatingTimeline)
  - `string` → Show `IntegratedChatPanel`
- TaskDetailOverlay rendered inside left section (same as Kanban)
- Footer in left section at bottom
- Resize handle between panels

**Key difference from KanbanSplitLayout:**
- Kanban: Chat toggleable (can hide completely)
- Graph: Right panel always visible, content switches (timeline ↔ chat)

### Task 2: Create FloatingTimeline Wrapper (BLOCKING)

**Dependencies:** None
**Atomic Commit:** `feat(graph): create FloatingTimeline wrapper component`

**File:** `src/components/TaskGraph/timeline/FloatingTimeline.tsx` (new, ~40 LOC)

**Purpose:** Wrap existing ExecutionTimeline content in glass container styling

**Changes:**
```tsx
// Outer wrapper with Tahoe glass styling
<div className="h-full p-2" style={{ backgroundColor: "hsl(220 10% 8%)" }}>
  <div style={{
    height: "100%",
    borderRadius: "10px",
    background: "hsla(220 10% 10% / 0.92)",
    backdropFilter: "blur(20px) saturate(180%)",
    border: "1px solid hsla(220 20% 100% / 0.08)",
    boxShadow: "0 4px 16px hsla(220 20% 0% / 0.4), 0 12px 32px hsla(220 20% 0% / 0.3)",
  }}>
    {/* Existing timeline content */}
  </div>
</div>
```

**ExecutionTimeline update:** Extract core content to be composable (no outer width/collapse logic when used inside FloatingTimeline).

### Task 3: Create FloatingGraphFilters Component (BLOCKING)

**Dependencies:** None
**Atomic Commit:** `feat(graph): create FloatingGraphFilters component`

**File:** `src/components/TaskGraph/controls/FloatingGraphFilters.tsx` (new, ~200 LOC)

**Position:** Absolute, left: 16px, top: 50%, transform: translateY(-50%)

**Layout:**
```
┌─────────────────┐
│ [Status Filter] │
├─────────────────┤
│ [Plan Filter]   │
├─────────────────┤
│ [Layout TB/LR]  │
├─────────────────┤
│ [Mode Std/Cpt]  │
├─────────────────┤
│ [Grouping ▼]    │
└─────────────────┘
```

**Styling:**
- Same Tahoe glass container
- Each filter: full-width button/dropdown
- Compact icons with tooltips for space efficiency
- Stacked vertically, gap between items

**Props:**
```tsx
interface FloatingGraphFiltersProps {
  filters: GraphFilters;
  onFiltersChange: (filters: GraphFilters) => void;
  layoutDirection: LayoutDirection;
  onLayoutDirectionChange: (dir: LayoutDirection) => void;
  nodeMode: NodeMode | null;
  onNodeModeChange: (mode: NodeMode | null) => void;
  grouping: GroupingOption;
  onGroupingChange: (opt: GroupingOption) => void;
  planGroups: PlanGroupInfo[];  // For plan filter options
}
```

### Task 4: Refactor TaskGraphView for New Layout (BLOCKING)

**Dependencies:** Task 1, Task 2, Task 3
**Atomic Commit:** `refactor(graph): integrate split layout with floating controls`

**File:** `src/components/TaskGraph/TaskGraphView.tsx` (major refactor)

**Before (current structure):**
```tsx
<div className="h-full w-full relative flex flex-col">
  <GraphControls ... />  {/* Top bar with filters */}
  <div className="flex-1 flex min-h-0">
    <div className="flex-1 h-full relative">
      <ReactFlow ... />
    </div>
    <ExecutionTimeline ... />  {/* Right sidebar */}
  </div>
</div>
{selectedTaskId && <TaskDetailOverlay ... />}
```

**After (new structure):**
```tsx
<GraphSplitLayout
  projectId={projectId}
  footer={footer}
  timelineContent={<FloatingTimeline ... />}
>
  <div className="h-full w-full relative">
    <ReactFlow ... />
    <FloatingGraphFilters ... />  {/* Positioned over canvas */}
    <GraphLegend />
  </div>
</GraphSplitLayout>
```

**Changes:**
- Remove GraphControls from JSX
- Remove ExecutionTimeline from main flex layout
- Add footer prop to component interface
- Wrap canvas in GraphSplitLayout
- Add FloatingGraphFilters as absolute-positioned overlay
- Keep all existing state (filters, layoutDirection, grouping, nodeMode)

### Task 5: Wire in App.tsx

**Dependencies:** Task 4
**Atomic Commit:** `feat(app): wire ExecutionControlBar to Graph page`

**File:** `src/App.tsx` (lines 718-720)

**Before:**
```tsx
{currentView === "graph" && (
  <TaskGraphView projectId={currentProjectId} />
)}
```

**After:**
```tsx
{currentView === "graph" && (
  <TaskGraphView
    projectId={currentProjectId}
    footer={
      <ExecutionControlBar
        runningCount={executionStatus.runningCount}
        maxConcurrent={executionStatus.maxConcurrent}
        queuedCount={executionStatus.queuedCount}
        isPaused={executionStatus.isPaused}
        isLoading={isExecutionLoading}
        onPauseToggle={handlePauseToggle}
        onStop={handleStop}
      />
    }
  />
)}
```

### Task 6: Update KanbanSplitLayout Width Constraints

**Dependencies:** None
**Atomic Commit:** `refactor(layout): narrow Kanban right panel width constraints`

**File:** `src/components/layout/KanbanSplitLayout.tsx`

**Changes:**
```tsx
// Before
const MIN_LEFT_PERCENT = 40;
const MAX_LEFT_PERCENT = 75;
const DEFAULT_LEFT_PERCENT = 75;

// After
const MIN_LEFT_PERCENT = 65;  // 65% left = 35% right (max chat width)
const MAX_LEFT_PERCENT = 75;  // 75% left = 25% right (min chat width)
const DEFAULT_LEFT_PERCENT = 70;  // Start with ~30% chat
```

### Task 7: Reorder Navbar Items

**Dependencies:** None
**Atomic Commit:** `refactor(navbar): reorder to Ideation → Graph → Kanban`

**File:** `src/App.tsx` or navbar component (need to locate)

**Changes:**
- Reorder main navigation: Ideation → Graph → Kanban
- Reflects natural workflow: plan ideas → visualize dependencies → execute tasks

## File Modifications Summary

| File | Change Type | Est. LOC |
|------|-------------|----------|
| `src/components/layout/GraphSplitLayout.tsx` | New | ~160 |
| `src/components/TaskGraph/timeline/FloatingTimeline.tsx` | New | ~40 |
| `src/components/TaskGraph/controls/FloatingGraphFilters.tsx` | New | ~200 |
| `src/components/TaskGraph/TaskGraphView.tsx` | Major refactor | Δ-100 |
| `src/components/layout/KanbanSplitLayout.tsx` | Update constants | Δ3 |
| `src/App.tsx` | Navbar order + graph wiring | Δ+15 |

## Design Decisions (Confirmed)

1. **Right panel width:** Resizable, 25-35% (not 25-60%). Apply to BOTH Kanban and Graph.
2. **Timeline:** Always expanded in glass container (no collapse toggle).
3. **Navbar order:** Reorder to Ideation → Graph → Kanban (reflects workflow: plan → visualize → execute).

## Verification

1. **Manual testing - Graph page:**
   - Navigate to Graph page
   - Verify timeline visible on right (floating, rounded glass container)
   - Click a task node → Task detail appears, chat opens on right, timeline hides
   - Close task → Timeline reappears
   - Verify floating filters work on left side of canvas
   - Verify execution bar at bottom
   - Verify keyboard navigation still works (arrow keys, Enter, Escape)
   - Verify resize handle works (25-35% right panel range)

2. **Manual testing - Kanban page:**
   - Verify right panel resize respects new 25-35% limits
   - Existing functionality still works

3. **Manual testing - Navbar:**
   - Verify order is now: Ideation → Graph → Kanban

4. **Lint/typecheck:**
   - `npm run lint && npm run typecheck`

5. **Visual verification:**
   - Screenshot before/after
   - Verify Tahoe liquid glass styling consistency
   - Verify accent color (#ff6b35) and SF Pro font

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)

### Task Dependency Graph

```
Task 1 (GraphSplitLayout) ─┐
Task 2 (FloatingTimeline) ─┼─→ Task 4 (Refactor TaskGraphView) ─→ Task 5 (Wire App.tsx)
Task 3 (FloatingFilters) ──┘

Task 6 (KanbanSplitLayout) ─→ (independent)
Task 7 (Navbar reorder) ────→ (independent)
```

**Parallel execution opportunities:**
- Tasks 1, 2, 3 can run in parallel (no dependencies)
- Tasks 6, 7 can run in parallel with everything (no dependencies)
- Task 4 must wait for Tasks 1, 2, 3
- Task 5 must wait for Task 4
