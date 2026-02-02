# RalphX - Phase 67: Task Graph View

## Overview

A React Flow-based Task Graph View as the primary orchestration view, replacing Kanban as the default view. This phase addresses critical limitations in lifecycle visibility, dependency visualization, and plan-to-completion tracking that the current Kanban board cannot provide.

The Task Graph displays tasks as nodes (color-coded by status), dependencies as edges, and groups tasks by their originating plan with visual regions showing progress.

**Reference Plan:**
- `specs/plans/task_graph_view.md` - Detailed implementation plan with component structure, visual design specs, and phased implementation tasks

## Goals

1. **Dependency Visualization** - Show task dependencies as directed edges with critical path highlighting
2. **Plan Grouping** - Group tasks by originating plan with progress bars and status summaries
3. **Lifecycle Visibility** - All 21 statuses visually distinct with actionable quick menus
4. **Execution Timeline** - Chronological history panel with node-to-timeline interaction
5. **Performance at Scale** - Handle 100+ tasks with compact mode and layout caching

## Dependencies

### Phase 66 (Per-Task Git Branch Isolation) - Required

| Dependency | Why Needed |
|------------|------------|
| Task state machine with merge states | Graph must display pending_merge, merging, merge_conflict, merged statuses |
| TaskDependencyRepository | Backend already tracks dependencies; graph queries this |
| Task state history | Timeline events source from existing infrastructure |

## Implementation Pattern

**CRITICAL: Before starting ANY task:**

1. **Read the full implementation plan** at `specs/plans/task_graph_view.md`
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
2. **Read the ENTIRE implementation plan** at `specs/plans/task_graph_view.md`
3. Locate the relevant section for this task
4. Only then begin implementation

After completing the task: update `"passes": true`, commit, and stop.

```json
[
  {
    "id": 1,
    "category": "backend",
    "description": "Add get_task_dependency_graph backend command",
    "plan_section": "Task A.1: Add get_task_dependency_graph backend command",
    "blocking": [2, 8, 14],
    "blockedBy": [],
    "atomic_commit": "feat(task-graph): add get_task_dependency_graph command",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task A.1'",
      "Create TaskDependencyGraphResponse, TaskGraphNode, TaskGraphEdge structs in query.rs",
      "Add get_task_dependency_graph command that queries TaskDependencyRepository",
      "Join with plan_artifacts and ideation_sessions tables for plan context",
      "Implement critical path computation via topological sort + DP",
      "Implement cycle detection",
      "Run cargo clippy && cargo test",
      "Commit: feat(task-graph): add get_task_dependency_graph command"
    ],
    "passes": true
  },
  {
    "id": 2,
    "category": "frontend",
    "description": "Create frontend API layer for task graph",
    "plan_section": "Task A.2: Create frontend API layer for task graph",
    "blocking": [4],
    "blockedBy": [1],
    "atomic_commit": "feat(api): add task-graph API layer",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task A.2'",
      "Create src/api/task-graph.schemas.ts with Zod schemas (snake_case)",
      "Create src/api/task-graph.transforms.ts with transform functions",
      "Create src/api/task-graph.types.ts with TypeScript types (camelCase)",
      "Create src/api/task-graph.ts with typedInvokeWithTransform wrapper",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(api): add task-graph API layer"
    ],
    "passes": true
  },
  {
    "id": 3,
    "category": "frontend",
    "description": "Install React Flow and dagre dependencies",
    "plan_section": "Task A.3: Install React Flow and dagre dependencies",
    "blocking": [4],
    "blockedBy": [],
    "atomic_commit": "chore(deps): add @xyflow/react and @dagrejs/dagre",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task A.3'",
      "Run npm install @xyflow/react @dagrejs/dagre",
      "Run npm install -D @types/dagre (if needed)",
      "Verify package.json updated",
      "Run npm run typecheck",
      "Commit: chore(deps): add @xyflow/react and @dagrejs/dagre"
    ],
    "passes": true
  },
  {
    "id": 4,
    "category": "frontend",
    "description": "Create TaskGraphView with basic React Flow setup",
    "plan_section": "Task A.4: Create TaskGraphView with basic React Flow setup",
    "blocking": [5, 6, 7, 12, 19, 23, 24, 25],
    "blockedBy": [2, 3],
    "atomic_commit": "feat(task-graph): create TaskGraphView with React Flow",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task A.4'",
      "Create src/components/TaskGraph/index.ts",
      "Create src/components/TaskGraph/hooks/useTaskGraph.ts to fetch graph data",
      "Create src/components/TaskGraph/TaskGraphView.tsx with ReactFlow canvas",
      "Use default nodes initially (custom nodes in Phase B)",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(task-graph): create TaskGraphView with React Flow"
    ],
    "passes": true
  },
  {
    "id": 5,
    "category": "frontend",
    "description": "Implement dagre layout computation",
    "plan_section": "Task A.5: Implement dagre layout computation",
    "blocking": [26],
    "blockedBy": [4],
    "atomic_commit": "feat(task-graph): implement dagre hierarchical layout",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task A.5'",
      "Create src/components/TaskGraph/hooks/useTaskGraphLayout.ts",
      "Implement dagre layout with TB (top-to-bottom) direction",
      "Add configurable spacing (nodesep: 60, ranksep: 80, margin: 40)",
      "Return positioned nodes and edges for React Flow",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(task-graph): implement dagre hierarchical layout"
    ],
    "passes": true
  },
  {
    "id": 6,
    "category": "frontend",
    "description": "Wire TaskGraphView to navigation",
    "plan_section": "Task A.6: Wire TaskGraphView to navigation",
    "blocking": [],
    "blockedBy": [4],
    "atomic_commit": "feat(navigation): add Graph view option",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task A.6'",
      "Add 'graph' to ViewType union in src/stores/uiStore.ts",
      "Add Graph nav item to src/components/Navigation.tsx with Network icon",
      "Wire TaskGraphView to render when view is 'graph'",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(navigation): add Graph view option"
    ],
    "passes": true
  },
  {
    "id": 7,
    "category": "frontend",
    "description": "Integrate TaskDetailOverlay on node click",
    "plan_section": "Task A.7: Integrate TaskDetailOverlay on node click",
    "blocking": [],
    "blockedBy": [4],
    "atomic_commit": "feat(task-graph): wire node click to TaskDetailOverlay",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task A.7'",
      "Add onNodeClick handler to TaskGraphView",
      "Call openModal('task-detail', { task }) to reuse existing overlay",
      "Ensure task data is fetched if not in node data",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(task-graph): wire node click to TaskDetailOverlay"
    ],
    "passes": true
  },
  {
    "id": 8,
    "category": "frontend",
    "description": "Create status color mapping",
    "plan_section": "Task B.1: Create status color mapping",
    "blocking": [9, 10, 11, 21],
    "blockedBy": [],
    "atomic_commit": "feat(task-graph): add status color mapping",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task B.1' and 'Visual Design'",
      "Create src/components/TaskGraph/nodes/nodeStyles.ts",
      "Define border/background colors for all 21 status states per spec",
      "Group colors: Idle, Blocked, Executing, QA, Review, Merge, Complete, Terminal",
      "Export getNodeStyle(status) function",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(task-graph): add status color mapping"
    ],
    "passes": true
  },
  {
    "id": 9,
    "category": "frontend",
    "description": "Create custom TaskNode component",
    "plan_section": "Task B.2: Create custom TaskNode component",
    "blocking": [12, 17, 18, 27],
    "blockedBy": [8],
    "atomic_commit": "feat(task-graph): create custom TaskNode component",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task B.2'",
      "Create src/components/TaskGraph/nodes/TaskNode.tsx",
      "Use React Flow's Handle components for connections",
      "Apply status colors from nodeStyles.ts",
      "Display task title (truncated) and status badge",
      "Set width to 180px per spec",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(task-graph): create custom TaskNode component"
    ],
    "passes": true
  },
  {
    "id": 10,
    "category": "frontend",
    "description": "Create custom DependencyEdge component",
    "plan_section": "Task B.3: Create custom DependencyEdge component",
    "blocking": [12, 22],
    "blockedBy": [8],
    "atomic_commit": "feat(task-graph): create custom DependencyEdge",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task B.3' and 'Edge Styles'",
      "Create src/components/TaskGraph/edges/edgeStyles.ts",
      "Create src/components/TaskGraph/edges/DependencyEdge.tsx",
      "Normal edges: dashed, --text-muted, 1px",
      "Critical path: solid, --accent-primary, 2px + glow",
      "Active (executing source): animated dotted",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(task-graph): create custom DependencyEdge"
    ],
    "passes": true
  },
  {
    "id": 11,
    "category": "frontend",
    "description": "Add GraphLegend component",
    "plan_section": "Task B.4: Add GraphLegend component",
    "blocking": [],
    "blockedBy": [8],
    "atomic_commit": "feat(task-graph): add status legend",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task B.4'",
      "Create src/components/TaskGraph/controls/GraphLegend.tsx",
      "Show status colors grouped by category",
      "Use compact layout (horizontal or collapsible)",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(task-graph): add status legend"
    ],
    "passes": true
  },
  {
    "id": 12,
    "category": "frontend",
    "description": "Wire custom nodes/edges to React Flow",
    "plan_section": "Task B.5: Wire custom nodes/edges to React Flow",
    "blocking": [],
    "blockedBy": [9, 10, 4],
    "atomic_commit": "feat(task-graph): wire custom nodes and edges",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task B.5'",
      "Update TaskGraphView.tsx to register custom nodeTypes",
      "Register custom edgeTypes",
      "Pass node/edge data with status and critical path info",
      "Verify nodes render with correct colors",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(task-graph): wire custom nodes and edges"
    ],
    "passes": true
  },
  {
    "id": 13,
    "category": "backend",
    "description": "Add PlanGroupInfo to backend response",
    "plan_section": "Task C.1: Add PlanGroupInfo to backend response",
    "blocking": [14],
    "blockedBy": [1],
    "atomic_commit": "feat(task-graph): add plan group info to graph response",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task C.1'",
      "Add PlanGroupInfo struct with plan_artifact_id, session_id, session_title, task_ids",
      "Add StatusSummary struct with counts by status",
      "Query ideation_sessions for session_title",
      "Group tasks by plan_artifact_id",
      "Run cargo clippy && cargo test",
      "Commit: feat(task-graph): add plan group info to graph response"
    ],
    "passes": true
  },
  {
    "id": 14,
    "category": "frontend",
    "description": "Update frontend types/transforms for plan groups",
    "plan_section": "Task C.2: Update frontend types/transforms for plan groups",
    "blocking": [15],
    "blockedBy": [13, 2],
    "atomic_commit": "feat(api): add plan group types to task-graph API",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task C.2'",
      "Add PlanGroupInfo schema to task-graph.schemas.ts",
      "Add StatusSummary schema",
      "Add transforms for plan groups",
      "Update TypeScript types",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(api): add plan group types to task-graph API"
    ],
    "passes": true
  },
  {
    "id": 15,
    "category": "frontend",
    "description": "Create PlanGroupHeader component",
    "plan_section": "Task C.3: Create PlanGroupHeader component",
    "blocking": [16],
    "blockedBy": [14],
    "atomic_commit": "feat(task-graph): create PlanGroupHeader",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task C.3' and 'Plan Group Header'",
      "Create src/components/TaskGraph/groups/PlanGroupHeader.tsx",
      "Display plan title (linked to planning session)",
      "Add progress bar (completed / total)",
      "Add status breakdown badges (done, executing, blocked, review)",
      "Add collapse toggle and context menu buttons",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(task-graph): create PlanGroupHeader"
    ],
    "passes": true
  },
  {
    "id": 16,
    "category": "frontend",
    "description": "Create PlanGroup visual region component",
    "plan_section": "Task C.4: Create PlanGroup visual region component",
    "blocking": [17, 22, 28],
    "blockedBy": [15],
    "atomic_commit": "feat(task-graph): create PlanGroup region",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task C.4'",
      "Create src/components/TaskGraph/groups/groupUtils.ts for bounding box calculations",
      "Create src/components/TaskGraph/groups/PlanGroup.tsx",
      "Use React Flow's group node type for visual container",
      "Apply subtle background --bg-elevated/50%",
      "Add rounded border with header",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(task-graph): create PlanGroup region"
    ],
    "passes": true
  },
  {
    "id": 17,
    "category": "frontend",
    "description": "Implement plan grouping logic in layout",
    "plan_section": "Task C.5: Implement plan grouping logic in layout",
    "blocking": [],
    "blockedBy": [16, 5],
    "atomic_commit": "feat(task-graph): add plan grouping to layout",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task C.5'",
      "Update useTaskGraphLayout.ts to calculate bounding boxes for plan groups",
      "Add padding around grouped tasks",
      "Create 'Ungrouped' region for standalone tasks",
      "Position group nodes in layout",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(task-graph): add plan grouping to layout"
    ],
    "passes": true
  },
  {
    "id": 18,
    "category": "frontend",
    "description": "Add collapse/expand for plan groups",
    "plan_section": "Task C.6: Add collapse/expand for plan groups",
    "blocking": [28],
    "blockedBy": [16],
    "atomic_commit": "feat(task-graph): add plan group collapse/expand",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task C.6'",
      "Add collapsed state to PlanGroup component",
      "When collapsed, show only header",
      "Wire collapse toggle in PlanGroupHeader",
      "Persist collapse state in component (or zustand if needed)",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(task-graph): add plan group collapse/expand"
    ],
    "passes": true
  },
  {
    "id": 19,
    "category": "backend",
    "description": "Add backend endpoint for timeline events",
    "plan_section": "Task D.1: Add backend endpoint for timeline events",
    "blocking": [20],
    "blockedBy": [1],
    "atomic_commit": "feat(task-graph): add get_task_timeline_events command",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task D.1'",
      "Create TimelineEvent struct with timestamp, task_id, event_type, description",
      "Add get_task_timeline_events command",
      "Query task_state_history table for state transitions",
      "Include plan-level events (plan accepted, all tasks complete)",
      "Order by timestamp descending",
      "Run cargo clippy && cargo test",
      "Commit: feat(task-graph): add get_task_timeline_events command"
    ],
    "passes": true
  },
  {
    "id": 20,
    "category": "frontend",
    "description": "Create frontend API for timeline events",
    "plan_section": "Task D.2: Create frontend API for timeline events",
    "blocking": [21],
    "blockedBy": [19],
    "atomic_commit": "feat(api): add timeline events API",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task D.2'",
      "Add TimelineEvent schema to task-graph.schemas.ts",
      "Add getTaskTimelineEvents function to task-graph.ts",
      "Add TypeScript types",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(api): add timeline events API"
    ],
    "passes": true
  },
  {
    "id": 21,
    "category": "frontend",
    "description": "Create TimelineEntry component",
    "plan_section": "Task D.3: Create TimelineEntry component",
    "blocking": [22],
    "blockedBy": [20],
    "atomic_commit": "feat(task-graph): create TimelineEntry component",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task D.3'",
      "Create src/components/TaskGraph/timeline/TimelineEntry.tsx",
      "Display timestamp, task reference, event description",
      "Use status colors from nodeStyles.ts for visual consistency",
      "Add clickable area for node interaction",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(task-graph): create TimelineEntry component"
    ],
    "passes": true
  },
  {
    "id": 22,
    "category": "frontend",
    "description": "Create ExecutionTimeline panel component",
    "plan_section": "Task D.4: Create ExecutionTimeline panel component",
    "blocking": [23, 24, 25],
    "blockedBy": [21],
    "atomic_commit": "feat(task-graph): create ExecutionTimeline panel",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task D.4' and 'Execution Timeline Panel'",
      "Create src/components/TaskGraph/hooks/useExecutionTimeline.ts",
      "Create src/components/TaskGraph/timeline/ExecutionTimeline.tsx",
      "Make panel collapsible",
      "Display chronological list of TimelineEntry components",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(task-graph): create ExecutionTimeline panel"
    ],
    "passes": true
  },
  {
    "id": 23,
    "category": "frontend",
    "description": "Implement timeline-to-node interaction",
    "plan_section": "Task D.5: Implement timeline-to-node interaction",
    "blocking": [],
    "blockedBy": [22, 4],
    "atomic_commit": "feat(task-graph): wire timeline entry to node highlight",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task D.5'",
      "Add onEntryClick handler to ExecutionTimeline",
      "Use React Flow's fitView or setCenter to scroll to node",
      "Add highlighted state to TaskNode for visual feedback",
      "Clear highlight after timeout or next interaction",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(task-graph): wire timeline entry to node highlight"
    ],
    "passes": true
  },
  {
    "id": 24,
    "category": "frontend",
    "description": "Add timeline event filters",
    "plan_section": "Task D.6: Add timeline event filters",
    "blocking": [],
    "blockedBy": [22],
    "atomic_commit": "feat(task-graph): add timeline event filters",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task D.6'",
      "Create src/components/TaskGraph/timeline/timelineFilters.ts",
      "Add filter UI to ExecutionTimeline (status changes, reviews, escalations)",
      "Filter timeline entries based on selection",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(task-graph): add timeline event filters"
    ],
    "passes": true
  },
  {
    "id": 25,
    "category": "frontend",
    "description": "Wire real-time updates for timeline",
    "plan_section": "Task D.7: Wire real-time updates for timeline",
    "blocking": [],
    "blockedBy": [22],
    "atomic_commit": "feat(task-graph): add real-time timeline updates",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task D.7'",
      "Update useExecutionTimeline.ts to subscribe to task:updated events",
      "Invalidate query on event",
      "Optionally auto-scroll to newest entry",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(task-graph): add real-time timeline updates"
    ],
    "passes": true
  },
  {
    "id": 26,
    "category": "frontend",
    "description": "Create TaskNodeContextMenu component",
    "plan_section": "Task E.1: Create TaskNodeContextMenu component",
    "blocking": [27],
    "blockedBy": [9],
    "atomic_commit": "feat(task-graph): create TaskNodeContextMenu",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task E.1' and 'Quick Actions Context Menu'",
      "Create src/components/TaskGraph/nodes/TaskNodeContextMenu.tsx",
      "Implement status-appropriate actions per the spec table",
      "Use existing action handlers from task detail views",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(task-graph): create TaskNodeContextMenu"
    ],
    "passes": true
  },
  {
    "id": 27,
    "category": "frontend",
    "description": "Wire context menu to task nodes",
    "plan_section": "Task E.2: Wire context menu to task nodes",
    "blocking": [],
    "blockedBy": [26],
    "atomic_commit": "feat(task-graph): wire context menu to nodes",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task E.2'",
      "Add onContextMenu handler to TaskNode.tsx",
      "Show TaskNodeContextMenu on right-click",
      "Pass task data and appropriate actions",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(task-graph): wire context menu to nodes"
    ],
    "passes": true
  },
  {
    "id": 28,
    "category": "frontend",
    "description": "Create GraphControls component",
    "plan_section": "Task E.3: Create GraphControls component",
    "blocking": [29],
    "blockedBy": [4],
    "atomic_commit": "feat(task-graph): create GraphControls",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task E.3' and 'Filtering & Grouping'",
      "Create src/components/TaskGraph/controls/GraphControls.tsx",
      "Add status filter (multi-select)",
      "Add plan filter",
      "Add layout direction toggle (TB ↔ LR)",
      "Add grouping options dropdown",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(task-graph): create GraphControls"
    ],
    "passes": true
  },
  {
    "id": 29,
    "category": "frontend",
    "description": "Create filter/grouping hooks",
    "plan_section": "Task E.4: Create filter/grouping hooks",
    "blocking": [],
    "blockedBy": [28],
    "atomic_commit": "feat(task-graph): add filter and grouping hooks",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task E.4'",
      "Create src/components/TaskGraph/hooks/useTaskGraphFilters.ts",
      "Manage filter state (status, plan, show completed)",
      "Manage grouping state (by plan, by tier, by status, none)",
      "Filter nodes/edges based on selections",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(task-graph): add filter and grouping hooks"
    ],
    "passes": true
  },
  {
    "id": 30,
    "category": "frontend",
    "description": "Create custom GraphMiniMap component",
    "plan_section": "Task E.5: Create custom GraphMiniMap component",
    "blocking": [],
    "blockedBy": [8],
    "atomic_commit": "feat(task-graph): create custom MiniMap with status colors",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task E.5'",
      "Create src/components/TaskGraph/controls/GraphMiniMap.tsx",
      "Use React Flow's MiniMap with custom nodeColor function",
      "Color nodes by status using nodeStyles.ts",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(task-graph): create custom MiniMap with status colors"
    ],
    "passes": false
  },
  {
    "id": 31,
    "category": "frontend",
    "description": "Handle cross-plan edge rendering",
    "plan_section": "Task E.6: Handle cross-plan edge rendering",
    "blocking": [],
    "blockedBy": [16],
    "atomic_commit": "feat(task-graph): handle cross-plan edges",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task E.6'",
      "Ensure edges crossing plan group boundaries render on top",
      "Adjust z-index or layer ordering in TaskGraphView",
      "Test with tasks that have cross-plan dependencies",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(task-graph): handle cross-plan edges"
    ],
    "passes": false
  },
  {
    "id": 32,
    "category": "frontend",
    "description": "Create TaskNodeCompact component",
    "plan_section": "Task F.1: Create TaskNodeCompact component",
    "blocking": [33],
    "blockedBy": [9],
    "atomic_commit": "feat(task-graph): create TaskNodeCompact for large graphs",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task F.1'",
      "Create src/components/TaskGraph/nodes/TaskNodeCompact.tsx",
      "Smaller dimensions than TaskNode",
      "Show only status color and abbreviated title",
      "Maintain handles for connections",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(task-graph): create TaskNodeCompact for large graphs"
    ],
    "passes": false
  },
  {
    "id": 33,
    "category": "frontend",
    "description": "Implement auto-switch to compact mode",
    "plan_section": "Task F.2: Implement auto-switch to compact mode",
    "blocking": [],
    "blockedBy": [32],
    "atomic_commit": "feat(task-graph): auto-switch to compact nodes at 50+ tasks",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task F.2'",
      "Add node count check in TaskGraphView",
      "Switch nodeTypes to TaskNodeCompact when count > 50",
      "Allow manual override toggle in GraphControls",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(task-graph): auto-switch to compact nodes at 50+ tasks"
    ],
    "passes": false
  },
  {
    "id": 34,
    "category": "frontend",
    "description": "Implement layout caching",
    "plan_section": "Task F.3: Implement layout caching",
    "blocking": [],
    "blockedBy": [5],
    "atomic_commit": "perf(task-graph): cache layout by graph hash",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task F.3'",
      "Add hash computation for graph structure (nodes, edges)",
      "Cache computed positions in useTaskGraphLayout",
      "Return cached layout if hash matches",
      "Invalidate cache on structural changes only",
      "Run npm run lint && npm run typecheck",
      "Commit: perf(task-graph): cache layout by graph hash"
    ],
    "passes": false
  },
  {
    "id": 35,
    "category": "frontend",
    "description": "Add micro-interactions and animations",
    "plan_section": "Task F.4: Add micro-interactions and animations",
    "blocking": [],
    "blockedBy": [4],
    "atomic_commit": "feat(task-graph): add animations and micro-interactions",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task F.4'",
      "Add smooth transitions for node position changes",
      "Add hover effects on nodes (scale, shadow)",
      "Add status change animations (color fade)",
      "Use CSS transitions or framer-motion",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(task-graph): add animations and micro-interactions"
    ],
    "passes": false
  },
  {
    "id": 36,
    "category": "frontend",
    "description": "Add keyboard navigation",
    "plan_section": "Task F.5: Add keyboard navigation",
    "blocking": [],
    "blockedBy": [4],
    "atomic_commit": "feat(task-graph): add keyboard navigation",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task F.5'",
      "Add focus state to nodes",
      "Arrow keys navigate through dependencies",
      "Enter opens TaskDetailOverlay for focused node",
      "Escape clears selection",
      "Run npm run lint && npm run typecheck",
      "Commit: feat(task-graph): add keyboard navigation"
    ],
    "passes": false
  },
  {
    "id": 37,
    "category": "frontend",
    "description": "Implement lazy loading for collapsed groups",
    "plan_section": "Task F.6: Implement lazy loading for collapsed groups",
    "blocking": [],
    "blockedBy": [18],
    "atomic_commit": "perf(task-graph): lazy load collapsed plan groups",
    "steps": [
      "Read specs/plans/task_graph_view.md section 'Task F.6'",
      "Don't render child nodes when group is collapsed",
      "Render nodes only when group expands",
      "Maintain correct layout calculations",
      "Run npm run lint && npm run typecheck",
      "Commit: perf(task-graph): lazy load collapsed plan groups"
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
| **React Flow for graph rendering** | Battle-tested library with good performance, custom nodes/edges, and built-in controls |
| **Dagre for layout** | Standard hierarchical layout algorithm, integrates well with React Flow |
| **Graph as primary view** | Addresses core pain points: dependency visibility, plan tracking, lifecycle overview |
| **Reuse TaskDetailOverlay** | Consistent UX with Kanban, no duplicate code for task details |
| **Plan groups as React Flow group nodes** | Native support for grouping, handles z-index and interactions |
| **Status colors from shared nodeStyles** | Single source of truth for 21-status color mapping |

---

## Verification Checklist

**Automated verification after completing all tasks:**

### Backend - Run `cargo test`
- [ ] get_task_dependency_graph returns correct node/edge data
- [ ] Critical path computation is correct
- [ ] Cycle detection works
- [ ] Plan group info includes session titles

### Frontend - Run `npm run test`
- [ ] useTaskGraph fetches and transforms data correctly
- [ ] useTaskGraphLayout produces valid positions
- [ ] Filter hooks filter nodes/edges correctly

### Build Verification (run only for modified code)
- [ ] Backend: `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] Backend: `cargo test` passes
- [ ] Frontend: `npm run lint && npm run typecheck` passes
- [ ] Build succeeds (`cargo build --release` / `npm run build`)

### Manual Testing
- [ ] Graph renders with tasks grouped by plan
- [ ] Dependencies show as directed edges
- [ ] Critical path is highlighted in accent color
- [ ] Clicking node opens TaskDetailOverlay
- [ ] Right-click shows context menu with appropriate actions
- [ ] Timeline updates in real-time as tasks change
- [ ] Filters reduce visible nodes
- [ ] Layout toggles between TB and LR
- [ ] Compact mode activates at 50+ tasks
- [ ] Plan groups collapse/expand

### Wiring Verification

**For each new component/feature, verify the full path from user action to code:**

- [ ] Entry point identified (click handler, route, event listener)
- [ ] New component is imported AND rendered (not behind disabled flag)
- [ ] API wrappers call backend commands
- [ ] State changes reflect in UI

**Common failure modes to check:**
- [ ] No optional props defaulting to `false` or disabled
- [ ] No components imported but never rendered
- [ ] No functions exported but never called
- [ ] No hooks defined but not used in components

See `.claude/rules/gap-verification.md` for full verification workflow.
