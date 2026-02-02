# Task Graph View - React Flow Based Primary View

## Problem Statement

**Core issue:** After creating a plan, breaking it into tasks, and launching execution - there's no way to see overall progress through the task lifecycle.

The current Kanban board has critical limitations:

1. **No lifecycle visibility** - Can't see: what's done, what needs action, what's blocking, what was accepted/merged/escalated
2. **No dependency visualization** - Tasks have dependencies in backend (`TaskDependencyRepository`) but Kanban shows a flat list with no "what blocks what" visibility
3. **No plan-to-completion tracking** - After accepting a plan, tasks scatter across columns with no way to see "Plan X: 3/7 complete, 2 blocked, 1 needs review"
4. **Overwhelming at scale** - 100+ tasks across 21 statuses creates cognitive overload
5. **Lost context** - `sourceProposalId` and `planArtifactId` exist but aren't exposed in task responses

## Proposed Solution

A **React Flow-based Task Graph View** as the primary orchestration view:

- **Nodes** = Tasks (color-coded by status)
- **Edges** = Dependencies (A → B means A must complete before B)
- **Groups** = Visual regions for tasks from the same plan
- **Layout** = Hierarchical by tier (like `TieredProposalList` but for executing tasks)

Kanban remains available as a secondary view for column-based work management.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│ TaskGraphView                                                   │
├─────────────────────────────────────────────────────────────────┤
│ GraphControls (filters, layout, grouping)                       │
├─────────────────────────────────────────────────────────────────┤
│ ┌─────────────────────────────────────────────────────────────┐ │
│ │ ReactFlow                                                   │ │
│ │  ┌─────────────────────────────────────────────────────┐   │ │
│ │  │ PlanGroup: "Feature X Plan"                         │   │ │
│ │  │  ┌──────────┐     ┌──────────┐     ┌──────────┐    │   │ │
│ │  │  │ Task A   │────→│ Task B   │────→│ Task C   │    │   │ │
│ │  │  │ [ready]  │     │ [blocked]│     │ [blocked]│    │   │ │
│ │  │  └──────────┘     └──────────┘     └──────────┘    │   │ │
│ │  └─────────────────────────────────────────────────────┘   │ │
│ │  ┌─────────────────────────────────────────────────────┐   │ │
│ │  │ PlanGroup: "Bug Fix Plan"                           │   │ │
│ │  │  ┌──────────┐     ┌──────────┐                      │   │ │
│ │  │  │ Task D   │────→│ Task E   │                      │   │ │
│ │  │  │[executing]     │ [blocked]│                      │   │ │
│ │  │  └──────────┘     └──────────┘                      │ │ │
│ │  └─────────────────────────────────────────────────────┘   │ │
│ │                                                             │ │
│ │  MiniMap │ Controls │ Legend                                │ │
│ └─────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
         │
         ▼ (click node)
┌─────────────────────────────────────────────────────────────────┐
│ TaskDetailOverlay (reused from Kanban)                          │
└─────────────────────────────────────────────────────────────────┘
```

---

## Component Structure

```
src/components/TaskGraph/
├── index.ts
├── TaskGraphView.tsx              # Main container with split layout
├── hooks/
│   ├── useTaskGraph.ts            # Fetches graph data
│   ├── useTaskGraphLayout.ts      # Dagre layout computation
│   ├── useTaskGraphFilters.ts     # Filtering/grouping
│   └── useExecutionTimeline.ts    # Timeline events
├── nodes/
│   ├── TaskNode.tsx               # Primary node (180px)
│   ├── TaskNodeCompact.tsx        # Compact for 50+ tasks
│   ├── TaskNodeContextMenu.tsx    # Quick actions menu
│   └── nodeStyles.ts              # Status color mapping
├── edges/
│   ├── DependencyEdge.tsx         # Custom edge styling
│   └── edgeStyles.ts
├── groups/
│   ├── PlanGroup.tsx              # Visual region for plan
│   ├── PlanGroupHeader.tsx        # Progress bar + status summary
│   └── groupUtils.ts
├── timeline/
│   ├── ExecutionTimeline.tsx      # Side panel timeline
│   ├── TimelineEntry.tsx          # Individual event
│   └── timelineFilters.ts
└── controls/
    ├── GraphControls.tsx          # Filters, layout toggle
    ├── GraphMiniMap.tsx           # Custom minimap
    └── GraphLegend.tsx            # Status legend
```

---

## Backend API Changes

### New Command: `get_task_dependency_graph`

```rust
#[derive(Serialize)]
pub struct TaskDependencyGraphResponse {
    pub nodes: Vec<TaskGraphNode>,
    pub edges: Vec<TaskGraphEdge>,
    pub plan_groups: Vec<PlanGroupInfo>,  // NEW: Plan grouping
    pub critical_path: Vec<String>,
    pub has_cycles: bool,
}

#[derive(Serialize)]
pub struct TaskGraphNode {
    pub task_id: String,
    pub title: String,
    pub internal_status: String,
    pub priority: i32,
    pub in_degree: u32,              // Number of blockers
    pub out_degree: u32,             // Number of dependents
    pub tier: u32,                   // Computed tier level
    pub plan_artifact_id: Option<String>,  // NEW: Link to plan
    pub source_proposal_id: Option<String>,
}

#[derive(Serialize)]
pub struct PlanGroupInfo {
    pub plan_artifact_id: String,
    pub session_id: String,
    pub session_title: Option<String>,
    pub task_ids: Vec<String>,
    pub status_summary: StatusSummary,  // counts by status
}
```

**Implementation:**
- Leverages existing `TaskDependencyRepository`
- Joins with `plan_artifacts` and `ideation_sessions` tables
- Critical path: longest path via topological sort + DP

---

## Visual Design

### Node Status Colors (21 Statuses)

| Status Group | Statuses | Border Color | Background |
|--------------|----------|--------------|------------|
| **Idle** | backlog, ready | `--text-muted` | `--bg-surface` |
| **Blocked** | blocked | `hsl(45 90% 55%)` | amber/10% |
| **Executing** | executing, re_executing | `--accent-primary` | orange/15% + glow |
| **QA** | qa_* | `hsl(280 60% 55%)` | purple/12% |
| **Review** | pending_review, reviewing, review_passed, escalated, revision_needed | `hsl(220 80% 60%)` | blue/12% |
| **Merge** | pending_merge, merging, merge_conflict | `hsl(180 60% 50%)` | cyan/12% |
| **Complete** | approved, merged | `hsl(145 60% 45%)` | green/12% |
| **Terminal** | failed, cancelled | `hsl(0 70% 55%)` | red/12% |

### Edge Styles

| Type | Style |
|------|-------|
| Normal | Dashed `--text-muted` 1px |
| Critical Path | Solid `--accent-primary` 2px + glow |
| Active (executing source) | Animated dotted |

### Plan Group Regions

- Subtle background `--bg-elevated/50%`
- Rounded border with plan title header
- Collapsible (minimize to header only)
- Status summary badge (e.g., "2/5 complete")

---

## Layout Algorithm

Using `@dagrejs/dagre` for hierarchical layout:

```typescript
const layoutConfig = {
  rankdir: "TB",     // Top-to-bottom (or "LR" for horizontal)
  nodesep: 60,       // Horizontal spacing
  ranksep: 80,       // Vertical spacing between tiers
  marginx: 40,
  marginy: 40,
};
```

**Grouping by Plan:**
1. Compute layout for entire graph
2. Calculate bounding box for each plan's tasks
3. Add padding for group region
4. React Flow's `groupNode` type for visual containers

---

## Filtering & Grouping

### Filter Options
- **By status**: Multi-select (executing, blocked, etc.)
- **By plan**: Select specific plan(s)
- **Show completed**: Toggle approved/merged visibility
- **Search**: Filter by task title

### Grouping Options
- **By plan** (default): Tasks grouped by originating session
- **By tier**: Horizontal tiers like TieredProposalList
- **By status**: Group by current status
- **None**: Flat dependency graph

---

## Integration Points

### View Switching
```typescript
// src/stores/uiStore.ts
type ViewType = "kanban" | "graph" | "ideation" | "activity" | "settings";
```

Add to Navigation: `{ id: "graph", label: "Graph", icon: Network }`

### TaskDetailOverlay (Reuse)
Click node → `openModal("task-detail", { task })` → same overlay as Kanban

### Real-time Updates
```typescript
useEffect(() => {
  const unsub = eventBus.subscribe("task:updated", () => {
    queryClient.invalidateQueries({ queryKey: ["task-graph", projectId] });
  });
  return unsub;
}, [projectId]);
```

---

## Performance Considerations

| Technique | When |
|-----------|------|
| **Compact nodes** | > 50 tasks → switch to TaskNodeCompact |
| **Viewport culling** | React Flow handles via `onlyRenderVisibleElements` |
| **Layout caching** | Cache computed positions by graph hash |
| **Lazy plan groups** | Collapse non-focused plans to summary |
| **Debounced updates** | Batch rapid state changes |

---

## Dependencies

```json
{
  "@xyflow/react": "^12.0.0",
  "@dagrejs/dagre": "^1.1.0"
}
```

---

## Phased Implementation

### Phase A: Foundation (Backend + Basic View) (BLOCKING)

**Phase Dependencies:** None

#### Task A.1: Add `get_task_dependency_graph` backend command (BLOCKING)
**Dependencies:** None
**Atomic Commit:** `feat(task-graph): add get_task_dependency_graph command`
**Files:** `src-tauri/src/commands/task_commands/query.rs`, `src-tauri/src/application/task_service.rs`

Add the new Tauri command that returns `TaskDependencyGraphResponse` with nodes, edges, plan_groups, critical_path, and has_cycles. Leverages existing `TaskDependencyRepository` and joins with `plan_artifacts`/`ideation_sessions` tables.

#### Task A.2: Create frontend API layer for task graph
**Dependencies:** Task A.1
**Atomic Commit:** `feat(api): add task-graph API layer`
**Files:** `src/api/task-graph.ts`, `src/api/task-graph.schemas.ts`, `src/api/task-graph.transforms.ts`, `src/api/task-graph.types.ts`

Create the API wrapper using the standard pattern: Zod schema (snake_case) → Transform → TS types (camelCase).

#### Task A.3: Install React Flow and dagre dependencies
**Dependencies:** None
**Atomic Commit:** `chore(deps): add @xyflow/react and @dagrejs/dagre`
**Files:** `package.json`

Install `@xyflow/react` ^12.0.0 and `@dagrejs/dagre` ^1.1.0.

#### Task A.4: Create `TaskGraphView` with basic React Flow setup (BLOCKING)
**Dependencies:** Task A.2, Task A.3
**Atomic Commit:** `feat(task-graph): create TaskGraphView with React Flow`
**Files:** `src/components/TaskGraph/index.ts`, `src/components/TaskGraph/TaskGraphView.tsx`, `src/components/TaskGraph/hooks/useTaskGraph.ts`

Basic React Flow canvas with simple nodes from the graph API.

#### Task A.5: Implement dagre layout computation
**Dependencies:** Task A.4
**Atomic Commit:** `feat(task-graph): implement dagre hierarchical layout`
**Files:** `src/components/TaskGraph/hooks/useTaskGraphLayout.ts`

Use dagre for TB (top-to-bottom) hierarchical layout with configurable spacing.

#### Task A.6: Wire TaskGraphView to navigation
**Dependencies:** Task A.4
**Atomic Commit:** `feat(navigation): add Graph view option`
**Files:** `src/stores/uiStore.ts`, `src/components/Navigation.tsx`

Add "graph" to ViewType union and add Graph nav item with Network icon.

#### Task A.7: Integrate TaskDetailOverlay on node click
**Dependencies:** Task A.4
**Atomic Commit:** `feat(task-graph): wire node click to TaskDetailOverlay`
**Files:** `src/components/TaskGraph/TaskGraphView.tsx`

Click node → `openModal("task-detail", { task })` using existing overlay.

---

### Phase B: Custom Nodes & Status Styling

**Phase Dependencies:** Phase A

#### Task B.1: Create status color mapping (BLOCKING)
**Dependencies:** None (parallel with Phase A after Task A.4)
**Atomic Commit:** `feat(task-graph): add status color mapping`
**Files:** `src/components/TaskGraph/nodes/nodeStyles.ts`

Define border/background colors for all 21 status states per visual design spec.

#### Task B.2: Create custom `TaskNode` component
**Dependencies:** Task B.1
**Atomic Commit:** `feat(task-graph): create custom TaskNode component`
**Files:** `src/components/TaskGraph/nodes/TaskNode.tsx`

Primary task node (180px) with status coloring, title, and status badge.

#### Task B.3: Create custom `DependencyEdge` component
**Dependencies:** Task B.1
**Atomic Commit:** `feat(task-graph): create custom DependencyEdge`
**Files:** `src/components/TaskGraph/edges/DependencyEdge.tsx`, `src/components/TaskGraph/edges/edgeStyles.ts`

Custom edge with critical path styling (solid accent + glow) vs normal (dashed muted).

#### Task B.4: Add `GraphLegend` component
**Dependencies:** Task B.1
**Atomic Commit:** `feat(task-graph): add status legend`
**Files:** `src/components/TaskGraph/controls/GraphLegend.tsx`

Legend showing status colors grouped by category (Idle, Blocked, Executing, etc.).

#### Task B.5: Wire custom nodes/edges to React Flow
**Dependencies:** Task B.2, Task B.3
**Atomic Commit:** `feat(task-graph): wire custom nodes and edges`
**Files:** `src/components/TaskGraph/TaskGraphView.tsx`

Register TaskNode and DependencyEdge as custom node/edge types.

---

### Phase C: Plan Grouping & Progress

**Phase Dependencies:** Phase B

#### Task C.1: Add `PlanGroupInfo` to backend response
**Dependencies:** Task A.1
**Atomic Commit:** `feat(task-graph): add plan group info to graph response`
**Files:** `src-tauri/src/commands/task_commands/query.rs`, `src-tauri/src/application/task_service.rs`

Include plan_artifact_id, session_id, session_title, task_ids, and status_summary in response.

#### Task C.2: Update frontend types/transforms for plan groups
**Dependencies:** Task C.1
**Atomic Commit:** `feat(api): add plan group types to task-graph API`
**Files:** `src/api/task-graph.schemas.ts`, `src/api/task-graph.transforms.ts`, `src/api/task-graph.types.ts`

Add PlanGroupInfo types and transforms.

#### Task C.3: Create `PlanGroupHeader` component
**Dependencies:** Task C.2
**Atomic Commit:** `feat(task-graph): create PlanGroupHeader`
**Files:** `src/components/TaskGraph/groups/PlanGroupHeader.tsx`

Header with plan title, progress bar, and status breakdown badges.

#### Task C.4: Create `PlanGroup` visual region component
**Dependencies:** Task C.3
**Atomic Commit:** `feat(task-graph): create PlanGroup region`
**Files:** `src/components/TaskGraph/groups/PlanGroup.tsx`, `src/components/TaskGraph/groups/groupUtils.ts`

Visual region container using React Flow's group node type.

#### Task C.5: Implement plan grouping logic in layout
**Dependencies:** Task C.4
**Atomic Commit:** `feat(task-graph): add plan grouping to layout`
**Files:** `src/components/TaskGraph/hooks/useTaskGraphLayout.ts`

Calculate bounding boxes for plan groups, add "Ungrouped" region for standalone tasks.

#### Task C.6: Add collapse/expand for plan groups
**Dependencies:** Task C.4
**Atomic Commit:** `feat(task-graph): add plan group collapse/expand`
**Files:** `src/components/TaskGraph/groups/PlanGroup.tsx`, `src/components/TaskGraph/TaskGraphView.tsx`

Toggle to minimize plan group to header only.

---

### Phase D: Execution Timeline

**Phase Dependencies:** Phase A (can run in parallel with B/C)

#### Task D.1: Add backend endpoint for timeline events
**Dependencies:** Task A.1
**Atomic Commit:** `feat(task-graph): add get_task_timeline_events command`
**Files:** `src-tauri/src/commands/task_commands/query.rs`, `src-tauri/src/application/task_service.rs`

Query task state history (existing infrastructure) to return timeline events.

#### Task D.2: Create frontend API for timeline events
**Dependencies:** Task D.1
**Atomic Commit:** `feat(api): add timeline events API`
**Files:** `src/api/task-graph.ts`, `src/api/task-graph.schemas.ts`, `src/api/task-graph.types.ts`

Add timeline event types and API wrapper.

#### Task D.3: Create `TimelineEntry` component
**Dependencies:** Task D.2
**Atomic Commit:** `feat(task-graph): create TimelineEntry component`
**Files:** `src/components/TaskGraph/timeline/TimelineEntry.tsx`

Individual timeline event with timestamp, task reference, and description.

#### Task D.4: Create `ExecutionTimeline` panel component
**Dependencies:** Task D.3
**Atomic Commit:** `feat(task-graph): create ExecutionTimeline panel`
**Files:** `src/components/TaskGraph/timeline/ExecutionTimeline.tsx`, `src/components/TaskGraph/hooks/useExecutionTimeline.ts`

Collapsible side panel showing chronological task execution history.

#### Task D.5: Implement timeline-to-node interaction
**Dependencies:** Task D.4, Task A.4
**Atomic Commit:** `feat(task-graph): wire timeline entry to node highlight`
**Files:** `src/components/TaskGraph/TaskGraphView.tsx`, `src/components/TaskGraph/timeline/ExecutionTimeline.tsx`

Click timeline entry → highlights node + scrolls to it in graph.

#### Task D.6: Add timeline event filters
**Dependencies:** Task D.4
**Atomic Commit:** `feat(task-graph): add timeline event filters`
**Files:** `src/components/TaskGraph/timeline/timelineFilters.ts`, `src/components/TaskGraph/timeline/ExecutionTimeline.tsx`

Filter by event type (status changes, reviews, escalations).

#### Task D.7: Wire real-time updates for timeline
**Dependencies:** Task D.4
**Atomic Commit:** `feat(task-graph): add real-time timeline updates`
**Files:** `src/components/TaskGraph/hooks/useExecutionTimeline.ts`

Subscribe to task:updated events and refresh timeline.

---

### Phase E: Quick Actions & Interactivity

**Phase Dependencies:** Phase B

#### Task E.1: Create `TaskNodeContextMenu` component
**Dependencies:** Task B.2
**Atomic Commit:** `feat(task-graph): create TaskNodeContextMenu`
**Files:** `src/components/TaskGraph/nodes/TaskNodeContextMenu.tsx`

Right-click menu with status-appropriate actions per the spec.

#### Task E.2: Wire context menu to task nodes
**Dependencies:** Task E.1
**Atomic Commit:** `feat(task-graph): wire context menu to nodes`
**Files:** `src/components/TaskGraph/nodes/TaskNode.tsx`

Show context menu on right-click, trigger appropriate actions.

#### Task E.3: Create `GraphControls` component
**Dependencies:** Task A.4
**Atomic Commit:** `feat(task-graph): create GraphControls`
**Files:** `src/components/TaskGraph/controls/GraphControls.tsx`

Filters (by status, by plan), layout toggle (TB ↔ LR), grouping options.

#### Task E.4: Create filter/grouping hooks
**Dependencies:** Task E.3
**Atomic Commit:** `feat(task-graph): add filter and grouping hooks`
**Files:** `src/components/TaskGraph/hooks/useTaskGraphFilters.ts`

State management for filters and grouping options.

#### Task E.5: Create custom `GraphMiniMap` component
**Dependencies:** Task B.1
**Atomic Commit:** `feat(task-graph): create custom MiniMap with status colors`
**Files:** `src/components/TaskGraph/controls/GraphMiniMap.tsx`

MiniMap showing nodes colored by status.

#### Task E.6: Handle cross-plan edge rendering
**Dependencies:** Task C.4
**Atomic Commit:** `feat(task-graph): handle cross-plan edges`
**Files:** `src/components/TaskGraph/TaskGraphView.tsx`, `src/components/TaskGraph/edges/DependencyEdge.tsx`

Ensure edges crossing plan group boundaries render correctly.

---

### Phase F: Polish & Performance

**Phase Dependencies:** Phases B, C, D, E

#### Task F.1: Create `TaskNodeCompact` component
**Dependencies:** Task B.2
**Atomic Commit:** `feat(task-graph): create TaskNodeCompact for large graphs`
**Files:** `src/components/TaskGraph/nodes/TaskNodeCompact.tsx`

Compact node variant for graphs with 50+ tasks.

#### Task F.2: Implement auto-switch to compact mode
**Dependencies:** Task F.1
**Atomic Commit:** `feat(task-graph): auto-switch to compact nodes at 50+ tasks`
**Files:** `src/components/TaskGraph/TaskGraphView.tsx`

Detect task count and switch node type automatically.

#### Task F.3: Implement layout caching
**Dependencies:** Task A.5
**Atomic Commit:** `perf(task-graph): cache layout by graph hash`
**Files:** `src/components/TaskGraph/hooks/useTaskGraphLayout.ts`

Cache computed positions by graph hash to avoid re-layout on minor updates.

#### Task F.4: Add micro-interactions and animations
**Dependencies:** Task A.4
**Atomic Commit:** `feat(task-graph): add animations and micro-interactions`
**Files:** `src/components/TaskGraph/TaskGraphView.tsx`, `src/components/TaskGraph/nodes/TaskNode.tsx`

Smooth transitions, hover effects, status change animations.

#### Task F.5: Add keyboard navigation
**Dependencies:** Task A.4
**Atomic Commit:** `feat(task-graph): add keyboard navigation`
**Files:** `src/components/TaskGraph/TaskGraphView.tsx`

Arrow keys to navigate through dependencies, Enter to open detail.

#### Task F.6: Implement lazy loading for collapsed groups
**Dependencies:** Task C.6
**Atomic Commit:** `perf(task-graph): lazy load collapsed plan groups`
**Files:** `src/components/TaskGraph/groups/PlanGroup.tsx`

Don't render nodes in collapsed groups until expanded.

---

## Key Files to Modify/Create

| File | Action |
|------|--------|
| `src-tauri/src/commands/task_commands/query.rs` | Add `get_task_dependency_graph` |
| `src-tauri/src/application/task_service.rs` | Graph assembly logic |
| `src/api/task-graph.ts` | New API layer |
| `src/components/TaskGraph/*` | New component directory |
| `src/stores/uiStore.ts` | Add "graph" view type |
| `src/components/Navigation.tsx` | Add Graph nav item |

---

## Design Decisions (Confirmed)

| Decision | Choice |
|----------|--------|
| **Default view** | Graph is primary, Kanban is secondary |
| **Standalone tasks** | "Ungrouped" visual region |
| **Cross-plan deps** | Supported - edges can cross plan boundaries |
| **Plan progress** | Plan-level progress bar in group header |
| **Quick actions** | Right-click menu on nodes |
| **Execution timeline** | Side panel showing chronological history |

---

## Execution Timeline Panel

A collapsible side panel showing chronological task execution history:

```
┌─────────────────────────────────────────────┐
│ Execution Timeline                    [−]   │
├─────────────────────────────────────────────┤
│ ● 10:45 Task C → merged                     │
│   "Feature implementation"                  │
│ ● 10:30 Task B → review_passed              │
│   Approved by AI reviewer                   │
│ ● 10:15 Task D → escalated                  │
│   "Needs human decision on API design"      │
│ ● 10:00 Task A → executing                  │
│   Worker agent started                      │
│ ● 09:45 Plan "Feature X" accepted           │
│   Created 5 tasks from 5 proposals          │
└─────────────────────────────────────────────┘
```

**Features:**
- Click timeline entry → highlights node + scrolls to it
- Filter by event type (status changes, reviews, escalations)
- Shows plan-level events (accepted, all tasks complete)
- Real-time updates as tasks progress

**Data source:** Task state history + activity events (existing infrastructure)

---

## Plan Group Header (Enhanced)

```
┌─────────────────────────────────────────────────────────────────┐
│ Plan: "Feature X Implementation"                    [−] [⋮]    │
│ ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ 60%                       │
│ ✓3 done  ⏳1 executing  ⚠1 blocked  👁1 review                 │
├─────────────────────────────────────────────────────────────────┤
│    [Task nodes...]                                              │
└─────────────────────────────────────────────────────────────────┘
```

**Header elements:**
- Plan title (linked to planning session)
- Progress bar (completed / total)
- Status breakdown badges (action-oriented)
- Collapse toggle and context menu

---

## Quick Actions Context Menu

Right-click on node shows status-appropriate actions:

| Current Status | Available Actions |
|----------------|-------------------|
| ready | Start Execution, Block (with reason) |
| blocked | Unblock, View Blockers |
| executing | View Agent Chat |
| pending_review | View Work Summary |
| review_passed | Approve, Request Changes |
| escalated | Approve, Reject, Request Changes |
| revision_needed | View Feedback |
| merge_conflict | View Conflicts, Mark Resolved |

---

## Verification

After implementation:

### Core Functionality
- [ ] Graph renders 100+ tasks without performance issues
- [ ] All 21 status states have distinct visual representation
- [ ] Dependencies show correctly with directionality (A blocks B)
- [ ] Critical path is highlighted with accent color

### Plan Grouping
- [ ] Plan groups visually contain their tasks
- [ ] Plan header shows title, progress bar, status breakdown
- [ ] "Ungrouped" region for standalone tasks
- [ ] Cross-plan edges render correctly (crossing boundaries)
- [ ] Plan groups can collapse/expand

### Execution Timeline
- [ ] Timeline shows chronological execution history
- [ ] Click timeline entry highlights node + scrolls to it
- [ ] Plan-level events shown (plan accepted, all tasks complete)
- [ ] Real-time updates as tasks change status

### Interactivity
- [ ] Click node opens TaskDetailOverlay with correct task
- [ ] Right-click shows status-appropriate quick actions
- [ ] Quick actions trigger correct state transitions
- [ ] Filters reduce visible nodes appropriately
- [ ] Layout direction toggle works (TB ↔ LR)

### Integration
- [ ] Graph is default view, Kanban accessible as secondary
- [ ] Real-time updates when task status changes
- [ ] Existing TaskDetailOverlay works without modification
- [ ] Navigation shows Graph option with correct icon

---

## Commit Lock Workflow (Parallel Agent Coordination)

**See `.claude/rules/commit-lock.md` for the complete atomic commit protocol.**
**See `.claude/rules/task-planning.md` for task design and compilation unit rules.**

Key points:
- All commit operations (check + acquire + commit + release) must be in a SINGLE Bash command
- Never separate the lock check and acquisition into different tool calls
- Each task must be a complete compilation unit (code compiles after each task)

### Task Dependency Summary

```
Phase A (Foundation) - BLOCKING
├── A.1 Backend command (BLOCKING) ──────────────────────────┐
│   └── A.2 Frontend API ──┐                                 │
│                          ├── A.4 TaskGraphView (BLOCKING) ─┤
├── A.3 Dependencies ──────┘     │                           │
│                                ├── A.5 Dagre layout        │
│                                ├── A.6 Navigation          │
│                                └── A.7 TaskDetailOverlay   │
│                                                            │
Phase B (Nodes & Styling)                                    │
├── B.1 Status colors (BLOCKING) ────────────────────────────┤
│   ├── B.2 TaskNode ────┐                                   │
│   ├── B.3 DependencyEdge ──┬── B.5 Wire to React Flow      │
│   └── B.4 GraphLegend      │                               │
│                            │                               │
Phase C (Plan Grouping)      │                               │
├── C.1 Backend (from A.1) ──│───────────────────────────────┘
│   └── C.2 Frontend types ──┤
│       └── C.3 PlanGroupHeader
│           └── C.4 PlanGroup ─┬── C.5 Layout grouping
│                              └── C.6 Collapse/expand
│
Phase D (Timeline) - parallel with B/C
├── D.1 Backend endpoint (from A.1)
│   └── D.2 Frontend API
│       └── D.3 TimelineEntry
│           └── D.4 ExecutionTimeline ─┬── D.5 Node highlight
│                                      ├── D.6 Filters
│                                      └── D.7 Real-time
│
Phase E (Interactivity)
├── E.1 TaskNodeContextMenu (from B.2)
│   └── E.2 Wire to nodes
├── E.3 GraphControls (from A.4)
│   └── E.4 Filter hooks
├── E.5 GraphMiniMap (from B.1)
└── E.6 Cross-plan edges (from C.4)

Phase F (Polish)
├── F.1 TaskNodeCompact (from B.2)
│   └── F.2 Auto-switch
├── F.3 Layout caching (from A.5)
├── F.4 Animations (from A.4)
├── F.5 Keyboard nav (from A.4)
└── F.6 Lazy loading (from C.6)
```

### Compilation Unit Notes

All tasks in this plan follow additive patterns or clear layer boundaries:
- **Backend → Frontend** tasks are properly sequenced (A.1 → A.2, C.1 → C.2, D.1 → D.2)
- **Component → Integration** tasks are properly sequenced (B.2 + B.3 → B.5)
- No tasks rename or modify existing fields/types that would break compilation
- Each task can be compiled and tested independently
