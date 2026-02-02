/**
 * TaskGraphView - React Flow-based task dependency graph visualization
 *
 * Displays tasks as nodes with dependencies as edges.
 * Uses dagre for hierarchical layout computation.
 * Custom TaskNode and DependencyEdge components provide rich visualization.
 *
 * Layout: Uses GraphSplitLayout for split-screen view with:
 * - Left: Graph canvas with FloatingGraphFilters overlay
 * - Right: FloatingTimeline (no task selected) or IntegratedChatPanel (task selected)
 * - Footer: Optional ExecutionControlBar
 */

import { useCallback, useEffect, useMemo, useState, useRef, type KeyboardEvent as ReactKeyboardEvent } from "react";
import {
  ReactFlow,
  ReactFlowProvider,
  Background,
  Controls,
  useReactFlow,
  type Node,
  type Edge,
  type NodeTypes,
  type EdgeTypes,
  type OnNodesChange,
  type OnEdgesChange,
} from "@xyflow/react";

import "@xyflow/react/dist/style.css";

import { useTaskGraph } from "./hooks/useTaskGraph";
import { useTaskGraphLayout } from "./hooks/useTaskGraphLayout";
import { TaskNode, type TaskNodeHandlers } from "./nodes/TaskNode";
import { TaskNodeCompact } from "./nodes/TaskNodeCompact";
import { DependencyEdge } from "./edges/DependencyEdge";
import { MARKER_IDS, NORMAL_STROKE, CRITICAL_STROKE } from "./edges/edgeStyles";
import { PlanGroup, PLAN_GROUP_NODE_TYPE } from "./groups/PlanGroup";
import { FloatingTimeline } from "./timeline/FloatingTimeline";
import { GraphLegend } from "./controls/GraphLegend";
import { FloatingGraphFilters } from "./controls/FloatingGraphFilters";
import {
  COMPACT_MODE_THRESHOLD,
  DEFAULT_GRAPH_FILTERS,
  DEFAULT_LAYOUT_DIRECTION,
  DEFAULT_GROUPING,
  type NodeMode,
  type GraphFilters,
  type LayoutDirection,
  type GroupingOption,
} from "./controls/GraphControls";
import { GraphSplitLayout } from "@/components/layout/GraphSplitLayout";
import type { TaskGraphNode, TaskGraphEdge, PlanGroupInfo } from "@/api/task-graph.types";
import type { InternalStatus } from "@/types/status";
import { useUiStore } from "@/stores/uiStore";
import { useTaskMutation } from "@/hooks/useTaskMutation";
import { api } from "@/lib/tauri";
import { toast } from "sonner";
import { Filter, Loader2, X } from "lucide-react";
import { Button } from "@/components/ui/button";

// ============================================================================
// Types
// ============================================================================

export interface TaskGraphViewProps {
  projectId: string;
  /** Optional footer to render at the bottom of the left section (e.g., ExecutionControlBar) */
  footer?: React.ReactNode;
}

// ============================================================================
// Constants
// ============================================================================

/** Duration in ms before clearing the highlighted node */
const HIGHLIGHT_TIMEOUT_MS = 3000;

/** Empty layout config - defined as constant to prevent re-renders */
const EMPTY_LAYOUT_CONFIG = {};

/** Status categories considered "completed" for filtering */
const COMPLETED_STATUSES: InternalStatus[] = ["approved", "merged"];

/**
 * Apply filters to graph data
 * Filters nodes based on status, plan, and showCompleted settings.
 * Filters edges to only include those where both source and target are visible.
 */
function applyGraphFilters(
  nodes: TaskGraphNode[],
  edges: TaskGraphEdge[],
  planGroups: PlanGroupInfo[],
  filters: GraphFilters
): { nodes: TaskGraphNode[]; edges: TaskGraphEdge[]; planGroups: PlanGroupInfo[] } {
  // Filter nodes
  const filteredNodes = nodes.filter((node) => {
    const status = node.internalStatus as InternalStatus;

    // Check showCompleted filter
    if (!filters.showCompleted && COMPLETED_STATUSES.includes(status)) {
      return false;
    }

    // Check status filter (empty = show all)
    if (filters.statuses.length > 0 && !filters.statuses.includes(status)) {
      return false;
    }

    // Check plan filter (empty = show all)
    // Use plan group's authoritative taskIds list instead of node.planArtifactId
    // This handles cases where session.plan_artifact_id differs from proposal.plan_artifact_id
    if (filters.planIds.length > 0) {
      const selectedPlanTaskIds = new Set(
        planGroups
          .filter((g) => filters.planIds.includes(g.planArtifactId))
          .flatMap((g) => g.taskIds)
      );

      if (!selectedPlanTaskIds.has(node.taskId)) {
        return false;
      }
    }

    return true;
  });

  // Build set of visible node IDs for edge filtering
  const visibleNodeIds = new Set(filteredNodes.map((n) => n.taskId));

  // Filter edges - only keep edges where both source and target are visible
  const filteredEdges = edges.filter(
    (edge) => visibleNodeIds.has(edge.source) && visibleNodeIds.has(edge.target)
  );

  // Update plan groups with filtered task lists
  const filteredPlanGroups = planGroups.map((group) => ({
    ...group,
    taskIds: group.taskIds.filter((taskId) => visibleNodeIds.has(taskId)),
  }));

  return {
    nodes: filteredNodes,
    edges: filteredEdges,
    planGroups: filteredPlanGroups,
  };
}

/**
 * Find the next node to navigate to based on arrow key direction
 * For Up/Down: follows dependency edges (Up = blockedBy/sources, Down = blocking/targets)
 * For Left/Right: moves to sibling nodes at the same tier level
 */
function findNextNode(
  direction: "up" | "down" | "left" | "right",
  currentNodeId: string,
  nodes: Node[],
  edges: Edge[]
): string | null {
  // Filter out group nodes
  const taskNodes = nodes.filter((n) => n.type === "task" || n.type === undefined);
  const currentNode = taskNodes.find((n) => n.id === currentNodeId);
  if (!currentNode) return null;

  if (direction === "up") {
    // Navigate to source nodes (tasks that block this one)
    const sourceIds = edges
      .filter((e) => e.target === currentNodeId)
      .map((e) => e.source);
    if (sourceIds.length === 0) return null;
    // If multiple sources, pick the one closest horizontally
    const sourceNodes = taskNodes.filter((n) => sourceIds.includes(n.id));
    if (sourceNodes.length === 0) return null;
    return sourceNodes.reduce((closest, node) =>
      Math.abs(node.position.x - currentNode.position.x) <
      Math.abs(closest.position.x - currentNode.position.x)
        ? node
        : closest
    ).id;
  }

  if (direction === "down") {
    // Navigate to target nodes (tasks blocked by this one)
    const targetIds = edges
      .filter((e) => e.source === currentNodeId)
      .map((e) => e.target);
    if (targetIds.length === 0) return null;
    // If multiple targets, pick the one closest horizontally
    const targetNodes = taskNodes.filter((n) => targetIds.includes(n.id));
    if (targetNodes.length === 0) return null;
    return targetNodes.reduce((closest, node) =>
      Math.abs(node.position.x - currentNode.position.x) <
      Math.abs(closest.position.x - currentNode.position.x)
        ? node
        : closest
    ).id;
  }

  // For left/right: find sibling nodes at similar Y positions (same tier)
  const tolerance = 40; // Y-position tolerance for "same tier"
  const siblingNodes = taskNodes.filter(
    (n) =>
      n.id !== currentNodeId &&
      Math.abs(n.position.y - currentNode.position.y) < tolerance
  );

  if (siblingNodes.length === 0) return null;

  if (direction === "left") {
    // Find the nearest node to the left
    const leftNodes = siblingNodes.filter(
      (n) => n.position.x < currentNode.position.x
    );
    if (leftNodes.length === 0) return null;
    return leftNodes.reduce((nearest, node) =>
      node.position.x > nearest.position.x ? node : nearest
    ).id;
  }

  if (direction === "right") {
    // Find the nearest node to the right
    const rightNodes = siblingNodes.filter(
      (n) => n.position.x > currentNode.position.x
    );
    if (rightNodes.length === 0) return null;
    return rightNodes.reduce((nearest, node) =>
      node.position.x < nearest.position.x ? node : nearest
    ).id;
  }

  return null;
}

// ============================================================================
// Custom Node and Edge Types
// ============================================================================

// ============================================================================
// Edge Marker Definitions
// ============================================================================

/**
 * SVG marker definitions for edge arrows
 * All arrows use the same size (8x6) - only color differs
 */
function EdgeMarkerDefinitions() {
  return (
    <svg style={{ position: "absolute", width: 0, height: 0 }}>
      <defs>
        {/* Normal edge arrow - muted gray */}
        <marker
          id={MARKER_IDS.normal}
          viewBox="0 0 10 10"
          refX={8}
          refY={5}
          markerWidth={8}
          markerHeight={6}
          markerUnits="userSpaceOnUse"
          orient="auto-start-reverse"
        >
          <path
            d="M 0 0 L 10 5 L 0 10 z"
            fill={NORMAL_STROKE}
          />
        </marker>

        {/* Critical path edge arrow - orange, same size as normal */}
        <marker
          id={MARKER_IDS.critical}
          viewBox="0 0 10 10"
          refX={8}
          refY={5}
          markerWidth={8}
          markerHeight={6}
          markerUnits="userSpaceOnUse"
          orient="auto-start-reverse"
        >
          <path
            d="M 0 0 L 10 5 L 0 10 z"
            fill={CRITICAL_STROKE}
          />
        </marker>

        {/* Active edge arrow - orange, same size as normal */}
        <marker
          id={MARKER_IDS.active}
          viewBox="0 0 10 10"
          refX={8}
          refY={5}
          markerWidth={8}
          markerHeight={6}
          markerUnits="userSpaceOnUse"
          orient="auto-start-reverse"
        >
          <path
            d="M 0 0 L 10 5 L 0 10 z"
            fill={CRITICAL_STROKE}
          />
        </marker>
      </defs>
    </svg>
  );
}

/**
 * Node types for standard mode (full-size nodes with status badges)
 * IMPORTANT: Defined outside component to prevent unnecessary re-renders
 */
const standardNodeTypes: NodeTypes = {
  task: TaskNode,
  [PLAN_GROUP_NODE_TYPE]: PlanGroup,
};

/**
 * Node types for compact mode (smaller nodes for 50+ task graphs)
 * IMPORTANT: Defined outside component to prevent unnecessary re-renders
 */
const compactNodeTypes: NodeTypes = {
  task: TaskNodeCompact,
  [PLAN_GROUP_NODE_TYPE]: PlanGroup,
};

/**
 * Register custom edge types for React Flow
 * IMPORTANT: Defined outside component to prevent unnecessary re-renders
 */
const edgeTypes: EdgeTypes = {
  dependency: DependencyEdge,
};

// ============================================================================
// Inner Component (has access to useReactFlow)
// ============================================================================

interface TaskGraphViewInnerProps {
  projectId: string;
  /** Optional footer to render at the bottom of the left section (e.g., ExecutionControlBar) */
  footer?: React.ReactNode;
}

function TaskGraphViewInner({ projectId, footer }: TaskGraphViewInnerProps) {
  const { data: graphData, isLoading, error } = useTaskGraph(projectId);
  const { setCenter, getNodes, fitView } = useReactFlow();

  // UI Store for task selection
  const selectedTaskId = useUiStore((s) => s.selectedTaskId);
  const setSelectedTaskId = useUiStore((s) => s.setSelectedTaskId);

  // Highlighted task state (for timeline-to-node interaction)
  const [highlightedTaskId, setHighlightedTaskId] = useState<string | null>(null);
  const highlightTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Keyboard-focused node state (for keyboard navigation)
  const [focusedNodeId, setFocusedNodeId] = useState<string | null>(null);

  // Collapse state for plan groups
  const [collapsedPlanIds, setCollapsedPlanIds] = useState<Set<string>>(
    new Set()
  );

  // Track which groups we've auto-collapsed (don't re-collapse after user expands)
  const autoCollapsedRef = useRef<Set<string>>(new Set());

  // Auto-collapse 100% completed groups on initial load
  useEffect(() => {
    if (!graphData?.planGroups) return;

    const toCollapse: string[] = [];
    for (const pg of graphData.planGroups) {
      // Skip if already processed
      if (autoCollapsedRef.current.has(pg.planArtifactId)) continue;

      // Check if 100% completed
      const total = pg.taskIds.length;
      const completed = pg.statusSummary.completed;
      if (total > 0 && completed === total) {
        toCollapse.push(pg.planArtifactId);
        autoCollapsedRef.current.add(pg.planArtifactId);
      }
    }

    if (toCollapse.length > 0) {
      setCollapsedPlanIds((prev) => {
        const next = new Set(prev);
        for (const id of toCollapse) {
          next.add(id);
        }
        return next;
      });
    }
  }, [graphData?.planGroups]);

  // GraphControls state
  const [filters, setFilters] = useState<GraphFilters>(DEFAULT_GRAPH_FILTERS);
  const [layoutDirection, setLayoutDirection] = useState<LayoutDirection>(DEFAULT_LAYOUT_DIRECTION);
  const [grouping, setGrouping] = useState<GroupingOption>(DEFAULT_GROUPING);

  // Node mode state (standard or compact)
  // null means "auto" - will be determined by task count
  const [nodeModeOverride, setNodeModeOverride] = useState<NodeMode | null>(null);

  // Calculate task count and determine effective node mode
  const taskCount = graphData?.nodes.length ?? 0;
  const isAutoCompact = taskCount >= COMPACT_MODE_THRESHOLD;
  const effectiveNodeMode: NodeMode = nodeModeOverride ?? (isAutoCompact ? "compact" : "standard");

  // Select the appropriate node types based on mode
  const activeNodeTypes = effectiveNodeMode === "compact" ? compactNodeTypes : standardNodeTypes;

  // Handler for manual node mode toggle
  const handleNodeModeChange = useCallback((mode: NodeMode) => {
    // If user sets to what auto would be, clear override (return to auto)
    const autoMode: NodeMode = isAutoCompact ? "compact" : "standard";
    if (mode === autoMode) {
      setNodeModeOverride(null);
    } else {
      setNodeModeOverride(mode);
    }
  }, [isAutoCompact]);

  // Toggle collapse state for a plan group
  const handleToggleCollapse = useCallback((planArtifactId: string) => {
    setCollapsedPlanIds((prev) => {
      const next = new Set(prev);
      if (next.has(planArtifactId)) {
        next.delete(planArtifactId);
      } else {
        next.add(planArtifactId);
      }
      return next;
    });
    // Auto-fit view after layout updates
    setTimeout(() => {
      fitView({ padding: 0.2, duration: 200 });
    }, 50);
  }, [fitView]);

  // Task mutations for context menu actions
  const {
    moveMutation,
    blockMutation,
    unblockMutation,
  } = useTaskMutation(projectId);

  // ============================================================================
  // Context Menu Handlers
  // ============================================================================

  // Handler for viewing task details (opens TaskDetailOverlay)
  const handleViewDetails = useCallback((taskId: string) => {
    setSelectedTaskId(taskId);
  }, [setSelectedTaskId]);

  // Handler for starting task execution (move to executing via state machine)
  const handleStartExecution = useCallback(async (taskId: string) => {
    try {
      // Moving to "executing" triggers the scheduler to pick up the task
      await api.tasks.move(taskId, "executing");
      toast.success("Task scheduled for execution");
    } catch (err) {
      toast.error(`Failed to start task: ${err instanceof Error ? err.message : String(err)}`);
    }
  }, []);

  // Handler for blocking a task with reason
  const handleBlockWithReason = useCallback((taskId: string, reason?: string) => {
    // Only pass reason if it's defined (satisfies exactOptionalPropertyTypes)
    if (reason !== undefined) {
      blockMutation.mutate({ taskId, reason });
    } else {
      blockMutation.mutate({ taskId });
    }
  }, [blockMutation]);

  // Handler for unblocking a task
  const handleUnblock = useCallback((taskId: string) => {
    unblockMutation.mutate(taskId);
  }, [unblockMutation]);

  // Handler for approving a task
  const handleApprove = useCallback(async (taskId: string) => {
    try {
      await api.reviews.approveTask({ task_id: taskId });
      toast.success("Task approved");
    } catch (err) {
      toast.error(`Failed to approve task: ${err instanceof Error ? err.message : String(err)}`);
    }
  }, []);

  // Handler for rejecting a task (move to failed)
  const handleReject = useCallback((taskId: string) => {
    moveMutation.mutate({ taskId, toStatus: "failed" });
  }, [moveMutation]);

  // Handler for requesting changes (move to revision_needed)
  const handleRequestChanges = useCallback((taskId: string) => {
    moveMutation.mutate({ taskId, toStatus: "revision_needed" });
  }, [moveMutation]);

  // Handler for marking merge conflict as resolved
  const handleMarkResolved = useCallback(async (taskId: string) => {
    try {
      await api.tasks.move(taskId, "pending_merge");
      toast.success("Conflict marked as resolved");
    } catch (err) {
      toast.error(`Failed to mark resolved: ${err instanceof Error ? err.message : String(err)}`);
    }
  }, []);

  // Memoized handlers object for nodes
  const nodeHandlers: TaskNodeHandlers = useMemo(() => ({
    onViewDetails: handleViewDetails,
    onStartExecution: handleStartExecution,
    onBlockWithReason: handleBlockWithReason,
    onUnblock: handleUnblock,
    onApprove: handleApprove,
    onReject: handleReject,
    onRequestChanges: handleRequestChanges,
    onMarkResolved: handleMarkResolved,
  }), [
    handleViewDetails,
    handleStartExecution,
    handleBlockWithReason,
    handleUnblock,
    handleApprove,
    handleReject,
    handleRequestChanges,
    handleMarkResolved,
  ]);

  // Apply filters to graph data before layout computation
  const filteredGraphData = useMemo(() => {
    if (!graphData) {
      return { nodes: [], edges: [], planGroups: [] };
    }
    return applyGraphFilters(
      graphData.nodes,
      graphData.edges,
      graphData.planGroups,
      filters
    );
  }, [graphData, filters]);

  // Compute layout using dagre (includes plan grouping)
  const { nodes: layoutNodes, edges: layoutEdges, groupNodes } = useTaskGraphLayout(
    filteredGraphData.nodes,
    filteredGraphData.edges,
    graphData?.criticalPath ?? [],
    filteredGraphData.planGroups,
    EMPTY_LAYOUT_CONFIG,
    collapsedPlanIds,
    handleToggleCollapse
  );

  // Compute visible nodes and edges (controlled mode - no useEffect sync needed)
  // Note: Lazy loading is now handled in useTaskGraphLayout - collapsed group tasks
  // are excluded from layout computation entirely, not just filtered after.
  // Inject handlers for context menu actions
  // Combine group nodes and visible task nodes - groups first for proper z-ordering
  const nodes = useMemo<Node[]>(() => {
    const taskNodesWithData = layoutNodes.map((node) => ({
      ...node,
      data: {
        ...node.data,
        isHighlighted: node.id === highlightedTaskId,
        isFocused: node.id === focusedNodeId,
        handlers: nodeHandlers,
      },
    }));
    return [...groupNodes, ...taskNodesWithData];
  }, [layoutNodes, groupNodes, highlightedTaskId, focusedNodeId, nodeHandlers]);

  // Edges are already filtered in useTaskGraphLayout (lazy loading)
  const edges = useMemo<Edge[]>(() => layoutEdges, [layoutEdges]);

  // Handle node changes (for selection, dragging etc.) in controlled mode
  const onNodesChange: OnNodesChange = useCallback(() => {
    // We don't allow user-driven node changes (positions come from dagre layout)
    // Selection is handled via onNodeClick
  }, []);

  // Handle edge changes in controlled mode
  const onEdgesChange: OnEdgesChange = useCallback(() => {
    // We don't allow user-driven edge changes (edges are computed from dependencies)
  }, []);

  // Handle timeline entry click - highlight node and scroll to it
  const handleTimelineTaskClick = useCallback(
    (taskId: string) => {
      // Clear any existing timeout
      if (highlightTimeoutRef.current) {
        clearTimeout(highlightTimeoutRef.current);
        highlightTimeoutRef.current = null;
      }

      // Set the highlighted task
      setHighlightedTaskId(taskId);

      // Find the node's position and center the view on it
      const allNodes = getNodes();
      const targetNode = allNodes.find((n) => n.id === taskId);
      if (targetNode && targetNode.position) {
        // Get node dimensions (default to standard task node size)
        const nodeWidth = targetNode.measured?.width ?? 180;
        const nodeHeight = targetNode.measured?.height ?? 60;
        // Center on the node's center point
        const x = targetNode.position.x + nodeWidth / 2;
        const y = targetNode.position.y + nodeHeight / 2;
        // Use setCenter with zoom level 1.2 for good visibility
        setCenter(x, y, { duration: 500, zoom: 1.2 });
      }

      // Set timeout to clear highlight
      highlightTimeoutRef.current = setTimeout(() => {
        setHighlightedTaskId(null);
        highlightTimeoutRef.current = null;
      }, HIGHLIGHT_TIMEOUT_MS);
    },
    [getNodes, setCenter]
  );

  // Clear highlight on any node click (new interaction)
  const handleNodeClick = useCallback(
    (_: React.MouseEvent, node: Node) => {
      // Clear any existing highlight timeout
      if (highlightTimeoutRef.current) {
        clearTimeout(highlightTimeoutRef.current);
        highlightTimeoutRef.current = null;
      }
      // Clear highlight and focus (mouse click takes over from keyboard)
      setHighlightedTaskId(null);
      setFocusedNodeId(null);

      // Skip group nodes (their IDs start with "group-")
      if (node.id.startsWith("group-")) {
        return;
      }
      // node.id is the task ID - open detail overlay
      setSelectedTaskId(node.id);
    },
    [setSelectedTaskId]
  );

  // Handle double-click on nodes (collapse/expand for groups)
  const handleNodeDoubleClick = useCallback(
    (_: React.MouseEvent, node: Node) => {
      // Only handle group nodes
      if (node.id.startsWith("group-")) {
        // Extract plan artifact ID from "group-{planArtifactId}"
        const planArtifactId = node.id.slice(6);
        handleToggleCollapse(planArtifactId);
      }
    },
    [handleToggleCollapse]
  );

  // Clear highlight on pane click
  const handlePaneClick = useCallback(() => {
    if (highlightTimeoutRef.current) {
      clearTimeout(highlightTimeoutRef.current);
      highlightTimeoutRef.current = null;
    }
    setHighlightedTaskId(null);
    // Also clear focus on pane click
    setFocusedNodeId(null);
  }, []);

  // Handle keyboard navigation
  const handleKeyDown = useCallback(
    (event: ReactKeyboardEvent<HTMLDivElement>) => {
      // Only handle navigation keys
      const key = event.key;
      if (!["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "Enter", "Escape"].includes(key)) {
        return;
      }

      event.preventDefault();

      // Escape clears selection and focus
      if (key === "Escape") {
        setSelectedTaskId(null);
        setFocusedNodeId(null);
        setHighlightedTaskId(null);
        if (highlightTimeoutRef.current) {
          clearTimeout(highlightTimeoutRef.current);
          highlightTimeoutRef.current = null;
        }
        return;
      }

      // If no node is focused, start with the first visible task node
      const taskNodes = nodes.filter((n) => n.type === "task" || !n.type);
      if (taskNodes.length === 0) return;

      const currentFocusId = focusedNodeId ?? selectedTaskId;
      if (!currentFocusId) {
        // Start with first task node
        const firstNode = taskNodes[0];
        if (firstNode) {
          setFocusedNodeId(firstNode.id);
          // Center on the focused node
          const nodeWidth = firstNode.measured?.width ?? 180;
          const nodeHeight = firstNode.measured?.height ?? 60;
          const x = firstNode.position.x + nodeWidth / 2;
          const y = firstNode.position.y + nodeHeight / 2;
          setCenter(x, y, { duration: 300, zoom: 1.2 });
        }
        return;
      }

      // Enter opens the detail overlay for the focused node
      if (key === "Enter") {
        setSelectedTaskId(currentFocusId);
        return;
      }

      // Arrow key navigation
      const direction = key === "ArrowUp" ? "up" :
                        key === "ArrowDown" ? "down" :
                        key === "ArrowLeft" ? "left" : "right";

      const nextNodeId = findNextNode(direction, currentFocusId, nodes, edges);
      if (nextNodeId) {
        setFocusedNodeId(nextNodeId);
        // Center on the newly focused node
        const targetNode = nodes.find((n) => n.id === nextNodeId);
        if (targetNode && targetNode.position) {
          const nodeWidth = targetNode.measured?.width ?? 180;
          const nodeHeight = targetNode.measured?.height ?? 60;
          const x = targetNode.position.x + nodeWidth / 2;
          const y = targetNode.position.y + nodeHeight / 2;
          setCenter(x, y, { duration: 300, zoom: 1.2 });
        }
      }
    },
    [nodes, edges, focusedNodeId, selectedTaskId, setSelectedTaskId, setCenter]
  );

  // Cleanup timeout on unmount
  useEffect(() => {
    return () => {
      if (highlightTimeoutRef.current) {
        clearTimeout(highlightTimeoutRef.current);
      }
    };
  }, []);

  // Loading state
  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-full">
        <Loader2 className="w-8 h-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  // Error state
  if (error) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center">
          <p className="text-destructive mb-2">Failed to load task graph</p>
          <p className="text-sm text-muted-foreground">{error.message}</p>
        </div>
      </div>
    );
  }

  // Empty state (no tasks at all)
  if (!graphData || graphData.nodes.length === 0) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center">
          <p className="text-muted-foreground mb-2">No tasks to display</p>
          <p className="text-sm text-muted-foreground">
            Create tasks from the Ideation view to see them here
          </p>
        </div>
      </div>
    );
  }

  // Check if filters hide all tasks
  const hasActiveFilters =
    filters.statuses.length > 0 ||
    filters.planIds.length > 0 ||
    !filters.showCompleted;

  return (
    <GraphSplitLayout
      projectId={projectId}
      footer={footer}
      timelineContent={
        <FloatingTimeline
          projectId={projectId}
          onTaskClick={handleTimelineTaskClick}
          highlightedTaskId={highlightedTaskId}
        />
      }
    >
      {/* Graph canvas container */}
      <div
        className="h-full w-full relative outline-none"
        data-testid="task-graph-view"
        tabIndex={0}
        onKeyDown={handleKeyDown}
      >
        {/* Floating filter controls - positioned over canvas */}
        <FloatingGraphFilters
          filters={filters}
          onFiltersChange={setFilters}
          layoutDirection={layoutDirection}
          onLayoutDirectionChange={setLayoutDirection}
          nodeMode={effectiveNodeMode}
          onNodeModeChange={handleNodeModeChange}
          isAutoCompact={isAutoCompact && nodeModeOverride === null}
          grouping={grouping}
          onGroupingChange={setGrouping}
          planGroups={graphData?.planGroups ?? []}
        />

        {/* Show empty state when filters hide all tasks */}
        {filteredGraphData.nodes.length === 0 && hasActiveFilters ? (
          <div className="flex items-center justify-center h-full">
            <div className="text-center">
              <Filter className="w-8 h-8 text-muted-foreground mx-auto mb-2" />
              <p className="text-muted-foreground mb-2">No tasks match current filters</p>
              <Button
                variant="outline"
                size="sm"
                onClick={() => setFilters(DEFAULT_GRAPH_FILTERS)}
              >
                <X className="w-3 h-3 mr-1" />
                Clear filters
              </Button>
            </div>
          </div>
        ) : (
          <ReactFlow
            nodes={nodes}
            edges={edges}
            nodeTypes={activeNodeTypes}
            edgeTypes={edgeTypes}
            onNodesChange={onNodesChange}
            onEdgesChange={onEdgesChange}
            onNodeClick={handleNodeClick}
            onNodeDoubleClick={handleNodeDoubleClick}
            onPaneClick={handlePaneClick}
            fitView
            fitViewOptions={{ padding: 0.2 }}
            minZoom={0.6}
            maxZoom={1}
            zoomOnDoubleClick={false}
            proOptions={{ hideAttribution: true }}
          >
            {/* SVG marker definitions for edge arrows */}
            <EdgeMarkerDefinitions />
            <Background color="hsl(220 10% 25%)" gap={20} />
            <Controls
              showInteractive={false}
              style={{
                background: "hsla(220 10% 12% / 0.9)",
                border: "1px solid hsla(220 20% 100% / 0.08)",
                borderRadius: 8,
              }}
            />
            {/* Status Legend - positioned to right of Controls */}
            <div className="absolute bottom-4 left-14 z-10">
              <GraphLegend defaultCollapsed={true} />
            </div>
          </ReactFlow>
        )}
      </div>
    </GraphSplitLayout>
  );
}

// ============================================================================
// Main Component (provides ReactFlowProvider)
// ============================================================================

export function TaskGraphView({ projectId, footer }: TaskGraphViewProps) {
  return (
    <ReactFlowProvider>
      <TaskGraphViewInner projectId={projectId} footer={footer} />
    </ReactFlowProvider>
  );
}
