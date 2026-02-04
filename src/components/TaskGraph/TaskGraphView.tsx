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

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  ReactFlow,
  ReactFlowProvider,
  Background,
  Controls,
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
import { useTaskGraphViewport } from "./hooks/useTaskGraphViewport";
import { useGraphSelectionController } from "./hooks/useGraphSelectionController";
import { TaskNode, type TaskNodeHandlers } from "./nodes/TaskNode";
import { TaskNodeCompact } from "./nodes/TaskNodeCompact";
import { DependencyEdge } from "./edges/DependencyEdge";
import { MARKER_IDS, NORMAL_STROKE, CRITICAL_STROKE, EDGE_FADE_COLOR } from "./edges/edgeStyles";
import { PlanGroup, PLAN_GROUP_NODE_TYPE, type PlanGroupData } from "./groups/PlanGroup";
import { TierGroup, TIER_GROUP_NODE_TYPE, type TierGroupData } from "./groups/TierGroup";
import { getPlanGroupNodeId, getTierGroupNodeId } from "./groups/groupTypes";
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
  type GroupingState,
} from "./controls/GraphControls";
import { GraphSplitLayout } from "@/components/layout/GraphSplitLayout";
import type { TaskGraphNode, TaskGraphEdge, PlanGroupInfo } from "@/api/task-graph.types";
import type { InternalStatus } from "@/types/status";
import { useUiStore } from "@/stores/uiStore";
import { useTaskMutation } from "@/hooks/useTaskMutation";
import { useNavCompactBreakpoint } from "@/hooks";
import { api } from "@/lib/tauri";
import { toast } from "sonner";
import { Filter, Loader2, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { buildTierGroups, UNGROUPED_PLAN_ID } from "./groups/tierGroupUtils";

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

/** Empty layout config - defined as constant to prevent re-renders */
// Layout config is now computed dynamically based on node mode

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

// ============================================================================
// Custom Node and Edge Types
// ============================================================================

// ============================================================================
// Edge Marker Definitions
// ============================================================================

/**
 * SVG definitions for edge arrows and gradients
 * Arrows are slightly larger for better visibility
 * Gradients fade from stroke color to near-background at the end
 */
function EdgeMarkerDefinitions() {
  return (
    <svg style={{ position: "absolute", width: 0, height: 0 }}>
      <defs>
        {/* Gradient definitions for edge fade effect */}
        <linearGradient id="edge-gradient-normal" x1="0%" y1="0%" x2="100%" y2="0%">
          <stop offset="0%" stopColor={NORMAL_STROKE} />
          <stop offset="70%" stopColor={NORMAL_STROKE} />
          <stop offset="100%" stopColor={EDGE_FADE_COLOR} stopOpacity="0.3" />
        </linearGradient>
        <linearGradient id="edge-gradient-critical" x1="0%" y1="0%" x2="100%" y2="0%">
          <stop offset="0%" stopColor={CRITICAL_STROKE} />
          <stop offset="70%" stopColor={CRITICAL_STROKE} />
          <stop offset="100%" stopColor={EDGE_FADE_COLOR} stopOpacity="0.3" />
        </linearGradient>
        <linearGradient id="edge-gradient-active" x1="0%" y1="0%" x2="100%" y2="0%">
          <stop offset="0%" stopColor={CRITICAL_STROKE} />
          <stop offset="70%" stopColor={CRITICAL_STROKE} />
          <stop offset="100%" stopColor={EDGE_FADE_COLOR} stopOpacity="0.3" />
        </linearGradient>

        {/* Normal edge arrow - muted gray, larger for visibility */}
        <marker
          id={MARKER_IDS.normal}
          viewBox="0 0 10 10"
          refX={9}
          refY={5}
          markerWidth={10}
          markerHeight={8}
          markerUnits="userSpaceOnUse"
          orient="auto-start-reverse"
        >
          <path
            d="M 0 0 L 10 5 L 0 10 z"
            fill={NORMAL_STROKE}
            fillOpacity="0.8"
          />
        </marker>

        {/* Critical path edge arrow - orange, larger for visibility */}
        <marker
          id={MARKER_IDS.critical}
          viewBox="0 0 10 10"
          refX={9}
          refY={5}
          markerWidth={10}
          markerHeight={8}
          markerUnits="userSpaceOnUse"
          orient="auto-start-reverse"
        >
          <path
            d="M 0 0 L 10 5 L 0 10 z"
            fill={CRITICAL_STROKE}
          />
        </marker>

        {/* Active edge arrow - orange, larger for visibility */}
        <marker
          id={MARKER_IDS.active}
          viewBox="0 0 10 10"
          refX={9}
          refY={5}
          markerWidth={10}
          markerHeight={8}
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
  [TIER_GROUP_NODE_TYPE]: TierGroup,
};

/**
 * Node types for compact mode (smaller nodes for 50+ task graphs)
 * IMPORTANT: Defined outside component to prevent unnecessary re-renders
 */
const compactNodeTypes: NodeTypes = {
  task: TaskNodeCompact,
  [PLAN_GROUP_NODE_TYPE]: PlanGroup,
  [TIER_GROUP_NODE_TYPE]: TierGroup,
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
  const {
    fitNodeInView,
    fitNode,
    centerOnNode,
    centerOnNodeObject,
    fitViewDefault,
    zoomBy,
  } = useTaskGraphViewport();
  const setSelectedTaskId = useUiStore((s) => s.setSelectedTaskId);
  const graphRightPanelUserOpen = useUiStore((s) => s.graphRightPanelUserOpen);
  const graphRightPanelCompactOpen = useUiStore((s) => s.graphRightPanelCompactOpen);
  const { isNavCompact } = useNavCompactBreakpoint();
  const [overlayClosing, setOverlayClosing] = useState(false);
  const overlayCloseTimeoutRef = useRef<number | null>(null);

  const graphReady = Boolean(graphData && graphData.nodes.length > 0);

  // Collapse state for plan groups
  const [collapsedPlanIds, setCollapsedPlanIds] = useState<Set<string>>(
    new Set()
  );
  const [collapsedTierIds, setCollapsedTierIds] = useState<Set<string>>(
    new Set()
  );

  const [grouping, setGrouping] = useState<GroupingState>(DEFAULT_GROUPING);

  // Compute and apply collapsed state for plan groups on initial data load
  // (avoid resetting on grouping toggles)
  useEffect(() => {
    if (!graphData) return;

    const planGroups = graphData.planGroups ?? [];
    const allNodes = graphData.nodes ?? [];

    // Find uncategorized tasks (not in any plan group)
    const groupedTaskIds = new Set<string>();
    for (const pg of planGroups) {
      for (const taskId of pg.taskIds) {
        groupedTaskIds.add(taskId);
      }
    }
    const ungroupedTasks = allNodes.filter((n) => !groupedTaskIds.has(n.taskId));

    // Build combined list of all groups with their completion status
    interface GroupInfo {
      id: string;
      total: number;
      completed: number;
    }
    const allGroups: GroupInfo[] = [];

    // Add plan groups
    for (const pg of planGroups) {
      allGroups.push({
        id: pg.planArtifactId,
        total: pg.taskIds.length,
        completed: pg.statusSummary.completed,
      });
    }

    // Add uncategorized if it has tasks
    if (ungroupedTasks.length > 0) {
      const completedCount = ungroupedTasks.filter((t) =>
        ["approved", "merged", "completed"].includes(t.internalStatus)
      ).length;
      allGroups.push({
        id: UNGROUPED_PLAN_ID,
        total: ungroupedTasks.length,
        completed: completedCount,
      });
    }

    if (allGroups.length > 0) {
      // Find first group that is NOT 100% complete - that one stays expanded
      let expandedGroupId: string | null = null;
      for (const g of allGroups) {
        if (g.total === 0 || g.completed < g.total) {
          expandedGroupId = g.id;
          break;
        }
      }
      // If all are 100%, keep the last one expanded
      if (!expandedGroupId) {
        expandedGroupId = allGroups[allGroups.length - 1]?.id ?? null;
      }

      // Collapse all groups except the expanded one
      const toCollapse = new Set(
        allGroups
          .filter((g) => g.id !== expandedGroupId)
          .map((g) => g.id)
      );

      setCollapsedPlanIds(toCollapse);
    } else {
      setCollapsedPlanIds(new Set());
    }

  }, [graphData]);

  // Compute tier collapse defaults when tier grouping is active
  useEffect(() => {
    if (!graphData) return;
    if (!grouping.byTier) {
      setCollapsedTierIds(new Set());
      return;
    }

    const planGroups = graphData.planGroups ?? [];
    const allNodes = graphData.nodes ?? [];
    const tierGroups = buildTierGroups(allNodes, planGroups, {
      enabled: grouping.byTier,
      includeUngrouped: grouping.showUncategorized || !grouping.byPlan,
    });
    if (tierGroups.length === 0) {
      setCollapsedTierIds(new Set());
      return;
    }

    const completedStatuses = new Set(["approved", "merged", "completed"]);
    const nodeMap = new Map(allNodes.map((node) => [node.taskId, node]));
    const tiersByPlan = new Map<string, typeof tierGroups>();

    for (const tg of tierGroups) {
      const existing = tiersByPlan.get(tg.planArtifactId);
      if (existing) {
        existing.push(tg);
      } else {
        tiersByPlan.set(tg.planArtifactId, [tg]);
      }
    }

    const toCollapseTiers = new Set<string>();

    for (const [, tiers] of tiersByPlan) {
      const tierInfos = tiers.map((tg) => {
        let completed = 0;
        for (const taskId of tg.taskIds) {
          const node = nodeMap.get(taskId);
          if (node && completedStatuses.has(node.internalStatus)) {
            completed += 1;
          }
        }
        return {
          id: tg.id,
          total: tg.taskIds.length,
          completed,
        };
      });

      let expandedTierId: string | null = null;
      for (const tier of tierInfos) {
        if (tier.total === 0 || tier.completed < tier.total) {
          expandedTierId = tier.id;
          break;
        }
      }
      if (!expandedTierId) {
        expandedTierId = tierInfos[tierInfos.length - 1]?.id ?? null;
      }

      for (const tier of tierInfos) {
        if (tier.id !== expandedTierId) {
          toCollapseTiers.add(tier.id);
        }
      }
    }

    setCollapsedTierIds(toCollapseTiers);
  }, [graphData, grouping.byPlan, grouping.byTier, grouping.showUncategorized]);

  useEffect(() => {
    if (!isNavCompact) {
      if (overlayCloseTimeoutRef.current) {
        window.clearTimeout(overlayCloseTimeoutRef.current);
        overlayCloseTimeoutRef.current = null;
      }
      setOverlayClosing(false);
      return;
    }

    if (graphRightPanelCompactOpen) {
      if (overlayCloseTimeoutRef.current) {
        window.clearTimeout(overlayCloseTimeoutRef.current);
        overlayCloseTimeoutRef.current = null;
      }
      setOverlayClosing(false);
      return;
    }

    setOverlayClosing(true);
    overlayCloseTimeoutRef.current = window.setTimeout(() => {
      setOverlayClosing(false);
      overlayCloseTimeoutRef.current = null;
    }, 200);

    return () => {
      if (overlayCloseTimeoutRef.current) {
        window.clearTimeout(overlayCloseTimeoutRef.current);
        overlayCloseTimeoutRef.current = null;
      }
    };
  }, [isNavCompact, graphRightPanelCompactOpen]);

  // GraphControls state
  const [filters, setFilters] = useState<GraphFilters>(DEFAULT_GRAPH_FILTERS);
  const [layoutDirection, setLayoutDirection] = useState<LayoutDirection>(DEFAULT_LAYOUT_DIRECTION);

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

  const centerOnPlanGroup = useCallback(
    (planArtifactId: string, duration = 200, zoom = 0.9): boolean =>
      centerOnNode(getPlanGroupNodeId(planArtifactId), {
        duration,
        zoom,
        fallbackWidth: 320,
        fallbackHeight: 120,
      }),
    [centerOnNode]
  );

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

  const tierGroups = useMemo(
    () => buildTierGroups(filteredGraphData.nodes, filteredGraphData.planGroups, {
      enabled: grouping.byTier,
      includeUngrouped: grouping.showUncategorized || !grouping.byPlan,
    }),
    [
      filteredGraphData.nodes,
      filteredGraphData.planGroups,
      grouping.byPlan,
      grouping.byTier,
      grouping.showUncategorized,
    ]
  );

  const tierGroupsById = useMemo(() => {
    const map = new Map<string, (typeof tierGroups)[number]>();
    for (const tierGroup of tierGroups) {
      map.set(tierGroup.id, tierGroup);
    }
    return map;
  }, [tierGroups]);

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
    // Auto-fit view after layout updates - focus on this group only
    setTimeout(() => {
      if (fitNodeInView(getPlanGroupNodeId(planArtifactId), { duration: 220, padding: 0.18, maxZoom: 0.95 })) return;
      if (centerOnPlanGroup(planArtifactId, 200, 0.9)) return;
      fitViewDefault({ padding: 0.2, duration: 200 });
    }, 50);
  }, [centerOnPlanGroup, fitNodeInView, fitViewDefault]);

  const handleToggleTierCollapse = useCallback((tierGroupId: string) => {
    let shouldCenterTier = false;
    let shouldCenterPlan = false;
    const planArtifactId = tierGroupsById.get(tierGroupId)?.planArtifactId ?? null;
    setCollapsedTierIds((prev) => {
      const next = new Set(prev);
      if (next.has(tierGroupId)) {
        next.delete(tierGroupId);
        shouldCenterTier = true;
      } else {
        next.add(tierGroupId);
        shouldCenterPlan = true;
      }
      return next;
    });
    setTimeout(() => {
      if (shouldCenterTier) {
        if (fitNodeInView(getTierGroupNodeId(tierGroupId), { duration: 220, padding: 0.18, maxZoom: 0.95 })) return;
        if (
          centerOnNode(getTierGroupNodeId(tierGroupId), {
            duration: 200,
            zoom: 0.95,
            fallbackWidth: 320,
            fallbackHeight: 80,
          })
        ) {
          return;
        }
      }
      if (shouldCenterPlan) {
        if (planArtifactId && fitNodeInView(getPlanGroupNodeId(planArtifactId), { duration: 220, padding: 0.18, maxZoom: 0.95 })) {
          return;
        }
        if (planArtifactId && centerOnPlanGroup(planArtifactId, 200, 0.9)) {
          return;
        }
      }
      fitViewDefault({ padding: 0.2, duration: 200 });
    }, 50);
  }, [centerOnNode, centerOnPlanGroup, fitNodeInView, fitViewDefault, tierGroupsById]);

  const tierGroupsByPlan = useMemo(() => {
    const map = new Map<string, string[]>();
    for (const tierGroup of tierGroups) {
      const existing = map.get(tierGroup.planArtifactId);
      if (existing) {
        existing.push(tierGroup.id);
      } else {
        map.set(tierGroup.planArtifactId, [tierGroup.id]);
      }
    }
    return map;
  }, [tierGroups]);

  const handleToggleAllTiers = useCallback(
    (planArtifactId: string, action: "expand" | "collapse") => {
      const tierIds = tierGroupsByPlan.get(planArtifactId) ?? [];
      if (tierIds.length === 0) return;
      setCollapsedTierIds((prev) => {
        const next = new Set(prev);
        if (action === "expand") {
          for (const tierId of tierIds) {
            next.delete(tierId);
          }
        } else {
          for (const tierId of tierIds) {
            next.add(tierId);
          }
        }
        return next;
      });
      setTimeout(() => {
        if (fitNodeInView(getPlanGroupNodeId(planArtifactId), { duration: 220, padding: 0.18, maxZoom: 0.95 })) return;
        if (centerOnPlanGroup(planArtifactId, 200, 0.9)) return;
        fitViewDefault({ padding: 0.2, duration: 200 });
      }, 50);
    },
    [centerOnPlanGroup, fitNodeInView, fitViewDefault, tierGroupsByPlan]
  );

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

  // Layout config with compact mode flag
  const layoutConfig = useMemo(() => ({
    isCompact: effectiveNodeMode === "compact",
  }), [effectiveNodeMode]);

  // Compute layout using dagre (includes plan grouping)
  const { nodes: layoutNodes, edges: layoutEdges, groupNodes } = useTaskGraphLayout(
    filteredGraphData.nodes,
    filteredGraphData.edges,
    graphData?.criticalPath ?? [],
    filteredGraphData.planGroups,
    grouping,
    layoutConfig,
    collapsedPlanIds,
    collapsedTierIds,
    handleToggleCollapse,
    handleToggleTierCollapse,
    handleToggleAllTiers
  );

  const {
    focusedNodeId,
    highlightedTaskId,
    graphSelection,
    containerRef,
    onNodeClick,
    onNodeDoubleClick,
    onPaneClick,
    onTimelineTaskClick,
    onKeyDown,
  } = useGraphSelectionController({
    nodes: layoutNodes,
    edges: layoutEdges,
    layoutNodes,
    groupNodes,
    planGroups: filteredGraphData.planGroups,
    tierGroups,
    grouping,
    collapsedPlanIds,
    collapsedTierIds,
    onToggleCollapse: handleToggleCollapse,
    onToggleTierCollapse: handleToggleTierCollapse,
    onToggleAllTiers: handleToggleAllTiers,
    centerOnPlanGroup,
    fitNodeInView,
    fitNode,
    centerOnNode,
    centerOnNodeObject,
    fitViewDefault,
    zoomBy,
    graphReady,
    graphError: error ?? null,
    isLoading,
  });

  // Compute visible nodes and edges (controlled mode - no useEffect sync needed)
  // Note: Lazy loading is now handled in useTaskGraphLayout - collapsed group tasks
  // are excluded from layout computation entirely, not just filtered after.
  // Inject handlers for context menu actions
  // Combine group nodes and visible task nodes - groups first for proper z-ordering
  const nodes = useMemo<Node[]>(() => {
    const groupNodesWithSelection = groupNodes.map((node) => {
      if (node.type === PLAN_GROUP_NODE_TYPE) {
        const planArtifactId = (node.data as PlanGroupData | undefined)?.planArtifactId;
        const isSelected = graphSelection?.kind === "planGroup" && graphSelection.id === planArtifactId;
        return {
          ...node,
          data: {
            ...node.data,
            isSelected,
          },
        };
      }
      if (node.type === TIER_GROUP_NODE_TYPE) {
        const tierGroupId = (node.data as TierGroupData | undefined)?.tierGroupId;
        const isSelected = graphSelection?.kind === "tierGroup" && graphSelection.id === tierGroupId;
        return {
          ...node,
          data: {
            ...node.data,
            isSelected,
          },
        };
      }
      return node;
    });
    const taskNodesWithData = layoutNodes.map((node) => ({
      ...node,
      data: {
        ...node.data,
        isHighlighted: node.id === highlightedTaskId,
        isFocused: node.id === focusedNodeId,
        handlers: nodeHandlers,
      },
      selected: graphSelection?.kind === "task" && graphSelection.id === node.id,
    }));
    return [...groupNodesWithSelection, ...taskNodesWithData];
  }, [layoutNodes, groupNodes, graphSelection, highlightedTaskId, focusedNodeId, nodeHandlers]);

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

  const graphRightPanelVisible = isNavCompact
    ? graphRightPanelCompactOpen
    : graphRightPanelUserOpen;
  const rightPanelMode = !graphRightPanelVisible
    ? "hidden"
    : isNavCompact
      ? "overlay"
      : "split";


  return (
    <GraphSplitLayout
      projectId={projectId}
      footer={footer}
      timelineContent={
        <FloatingTimeline
          projectId={projectId}
          onTaskClick={onTimelineTaskClick}
          highlightedTaskId={highlightedTaskId}
          variant={rightPanelMode === "overlay" || overlayClosing ? "overlay" : "panel"}
        />
      }
      rightPanelMode={rightPanelMode}
    >
      {/* Graph canvas container */}
      <div
        className="h-full w-full relative outline-none"
        data-testid="task-graph-view"
        tabIndex={0}
        ref={containerRef}
        onKeyDown={onKeyDown}
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
          isCompact={isNavCompact}
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
            onNodeClick={onNodeClick}
            onNodeDoubleClick={onNodeDoubleClick}
            onPaneClick={onPaneClick}
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
