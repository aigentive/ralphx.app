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
  type Node,
  type Edge,
  type NodeTypes,
  type EdgeTypes,
  type OnNodesChange,
  type OnEdgesChange,
  useStore,
} from "@xyflow/react";

import "@xyflow/react/dist/style.css";

import { useTaskGraph } from "./hooks/useTaskGraph";
import { useTaskGraphLayout } from "./hooks/useTaskGraphLayout";
import { useTaskGraphViewport } from "./hooks/useTaskGraphViewport";
import { TaskNode, type TaskNodeHandlers } from "./nodes/TaskNode";
import { TaskNodeCompact } from "./nodes/TaskNodeCompact";
import { DependencyEdge } from "./edges/DependencyEdge";
import { MARKER_IDS, NORMAL_STROKE, CRITICAL_STROKE, EDGE_FADE_COLOR } from "./edges/edgeStyles";
import { PlanGroup, PLAN_GROUP_NODE_TYPE, type PlanGroupData } from "./groups/PlanGroup";
import { TierGroup, TIER_GROUP_NODE_TYPE, type TierGroupData } from "./groups/TierGroup";
import { getPlanGroupNodeId, getTierGroupNodeId, parseGroupNodeId } from "./groups/groupTypes";
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
import type { GraphSelection } from "@/stores/uiStore";
import { useTaskMutation } from "@/hooks/useTaskMutation";
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

/** Duration in ms before clearing the highlighted node */
const HIGHLIGHT_TIMEOUT_MS = 3000;
const DOUBLE_CLICK_DELAY_MS = 140;

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

/**
 * Find the next group node to navigate to based on arrow direction.
 * Uses positional proximity to select the closest node in the given direction.
 */
function findNextGroupNode(
  direction: "up" | "down" | "left" | "right",
  currentNodeId: string,
  groupNodes: Node[]
): string | null {
  const currentNode = groupNodes.find((n) => n.id === currentNodeId);
  if (!currentNode) return null;

  const candidates = groupNodes.filter((n) => {
    if (n.id === currentNodeId) return false;
    if (direction === "up") return n.position.y < currentNode.position.y;
    if (direction === "down") return n.position.y > currentNode.position.y;
    if (direction === "left") return n.position.x < currentNode.position.x;
    return n.position.x > currentNode.position.x;
  });

  if (candidates.length === 0) return null;

  return candidates.reduce((closest, node) => {
    const primaryDelta =
      direction === "up" || direction === "down"
        ? Math.abs(node.position.y - currentNode.position.y)
        : Math.abs(node.position.x - currentNode.position.x);
    const closestPrimary =
      direction === "up" || direction === "down"
        ? Math.abs(closest.position.y - currentNode.position.y)
        : Math.abs(closest.position.x - currentNode.position.x);

    if (primaryDelta !== closestPrimary) {
      return primaryDelta < closestPrimary ? node : closest;
    }

    const secondaryDelta =
      direction === "up" || direction === "down"
        ? Math.abs(node.position.x - currentNode.position.x)
        : Math.abs(node.position.y - currentNode.position.y);
    const closestSecondary =
      direction === "up" || direction === "down"
        ? Math.abs(closest.position.x - currentNode.position.x)
        : Math.abs(closest.position.y - currentNode.position.y);

    return secondaryDelta < closestSecondary ? node : closest;
  }).id;
}

function sortNodesByPosition(a: Node, b: Node): number {
  if (a.position.y === b.position.y) {
    return a.position.x - b.position.x;
  }
  return a.position.y - b.position.y;
}

function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tagName = target.tagName.toLowerCase();
  return tagName === "input" || tagName === "textarea" || target.isContentEditable;
}

function hasHandledGraphKey(event: { nativeEvent?: KeyboardEvent } | KeyboardEvent): boolean {
  const nativeEvent = "nativeEvent" in event ? event.nativeEvent : event;
  if (!nativeEvent) return false;
  const marker = nativeEvent as KeyboardEvent & { __graphHandled?: boolean };
  if (marker.__graphHandled) return true;
  marker.__graphHandled = true;
  return false;
}

function isNavigableGraphSelection(
  selection: GraphSelection | null
): selection is { kind: "task" | "planGroup" | "tierGroup"; id: string } {
  return Boolean(
    selection &&
      (selection.kind === "task" ||
        selection.kind === "planGroup" ||
        selection.kind === "tierGroup")
  );
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
  const reactFlowDomNode = useStore((state) => state.domNode);

  // UI Store for task selection
  const selectedTaskId = useUiStore((s) => s.selectedTaskId);
  const setSelectedTaskId = useUiStore((s) => s.setSelectedTaskId);
  const graphSelection = useUiStore((s) => s.graphSelection);
  const setGraphSelection = useUiStore((s) => s.setGraphSelection);
  const clearGraphSelection = useUiStore((s) => s.clearGraphSelection);

  // Highlighted task state (for timeline-to-node interaction)
  const [highlightedTaskId, setHighlightedTaskId] = useState<string | null>(null);
  const highlightTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const groupClickTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const containerRef = useRef<HTMLDivElement | null>(null);

  // Keyboard-focused node state (for keyboard navigation)
  const [focusedNodeId, setFocusedNodeId] = useState<string | null>(null);

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

  const planGroupsById = useMemo(() => {
    const map = new Map<string, PlanGroupInfo>();
    for (const planGroup of filteredGraphData.planGroups) {
      map.set(planGroup.planArtifactId, planGroup);
    }
    return map;
  }, [filteredGraphData.planGroups]);

  const taskToPlanMap = useMemo(() => {
    const map = new Map<string, string>();
    for (const planGroup of filteredGraphData.planGroups) {
      for (const taskId of planGroup.taskIds) {
        map.set(taskId, planGroup.planArtifactId);
      }
    }
    return map;
  }, [filteredGraphData.planGroups]);

  const taskToTierMap = useMemo(() => {
    const map = new Map<string, string>();
    for (const tierGroup of tierGroups) {
      for (const taskId of tierGroup.taskIds) {
        map.set(taskId, tierGroup.id);
      }
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

  const taskNodesById = useMemo(() => {
    const map = new Map<string, Node>();
    for (const node of layoutNodes) {
      map.set(node.id, node);
    }
    return map;
  }, [layoutNodes]);

  const graphNodesById = useMemo(() => {
    const map = new Map<string, Node>();
    for (const node of groupNodes) {
      map.set(node.id, node);
    }
    for (const node of layoutNodes) {
      map.set(node.id, node);
    }
    return map;
  }, [groupNodes, layoutNodes]);

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

  const planGroupNodes = useMemo(
    () => groupNodes.filter((node) => node.type === PLAN_GROUP_NODE_TYPE),
    [groupNodes]
  );
  const tierGroupNodes = useMemo(
    () => groupNodes.filter((node) => node.type === TIER_GROUP_NODE_TYPE),
    [groupNodes]
  );
  const planGroupNodesSorted = useMemo(
    () => [...planGroupNodes].sort(sortNodesByPosition),
    [planGroupNodes]
  );

  // Handle node changes (for selection, dragging etc.) in controlled mode
  const onNodesChange: OnNodesChange = useCallback(() => {
    // We don't allow user-driven node changes (positions come from dagre layout)
    // Selection is handled via onNodeClick
  }, []);

  // Handle edge changes in controlled mode
  const onEdgesChange: OnEdgesChange = useCallback(() => {
    // We don't allow user-driven edge changes (edges are computed from dependencies)
  }, []);

  const focusSelectionInView = useCallback(
    (selection: { kind: "task" | "planGroup" | "tierGroup"; id: string }): void => {
      const nodeId =
        selection.kind === "planGroup"
          ? getPlanGroupNodeId(selection.id)
          : selection.kind === "tierGroup"
            ? getTierGroupNodeId(selection.id)
            : selection.id;

      const runFocus = () => {
        const node = graphNodesById.get(nodeId);
        if (node) {
          fitNode(node, { duration: 220, padding: 0.18, maxZoom: 0.95 });
          if (selection.kind === "planGroup") {
            centerOnNodeObject(node, { duration: 200, zoom: 0.9, fallbackWidth: 320, fallbackHeight: 120 });
            return;
          }
          centerOnNodeObject(node, { duration: 200, zoom: 0.95, fallbackWidth: 180, fallbackHeight: 60 });
          return;
        }
        fitNodeInView(nodeId, { duration: 220, padding: 0.18, maxZoom: 0.95 });
        if (selection.kind === "planGroup") {
          centerOnPlanGroup(selection.id, 200, 0.9);
          return;
        }
        centerOnNode(nodeId, { duration: 200, zoom: 0.95, fallbackWidth: 180, fallbackHeight: 60 });
      };

      requestAnimationFrame(() => {
        requestAnimationFrame(runFocus);
      });
    },
    [
      centerOnNode,
      centerOnNodeObject,
      centerOnPlanGroup,
      fitNode,
      fitNodeInView,
      graphNodesById,
    ]
  );

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
      setGraphSelection({ kind: "task", id: taskId });

      focusSelectionInView({ kind: "task", id: taskId });

      // Set timeout to clear highlight
      highlightTimeoutRef.current = setTimeout(() => {
        setHighlightedTaskId(null);
        highlightTimeoutRef.current = null;
      }, HIGHLIGHT_TIMEOUT_MS);
    },
    [focusSelectionInView, setGraphSelection]
  );

  // Single click on node - focus/highlight and center view (consistent with timeline behavior)
  const handleNodeClick = useCallback(
    (event: React.MouseEvent, node: Node) => {
      // Group nodes: fit into view on single click
      if (node.type === PLAN_GROUP_NODE_TYPE) {
        if (event.detail > 1) {
          return;
        }
        const parsed = parseGroupNodeId(node.id);
        const planArtifactId =
          (node.data as PlanGroupData | undefined)?.planArtifactId ??
          (parsed?.kind === "plan" ? parsed.id : undefined);
        if (planArtifactId) {
          setGraphSelection({ kind: "planGroup", id: planArtifactId });
          focusSelectionInView({ kind: "planGroup", id: planArtifactId });
        }
        if (groupClickTimeoutRef.current) {
          clearTimeout(groupClickTimeoutRef.current);
        }
        groupClickTimeoutRef.current = setTimeout(() => {
          groupClickTimeoutRef.current = null;
        }, DOUBLE_CLICK_DELAY_MS);
        return;
      }
      if (node.type === TIER_GROUP_NODE_TYPE) {
        if (event.detail > 1) {
          return;
        }
        const parsed = parseGroupNodeId(node.id);
        const tierGroupId =
          (node.data as TierGroupData | undefined)?.tierGroupId ??
          (parsed?.kind === "tier" ? parsed.id : undefined);
        if (tierGroupId) {
          setGraphSelection({ kind: "tierGroup", id: tierGroupId });
          focusSelectionInView({ kind: "tierGroup", id: tierGroupId });
        }
        if (groupClickTimeoutRef.current) {
          clearTimeout(groupClickTimeoutRef.current);
        }
        groupClickTimeoutRef.current = setTimeout(() => {
          groupClickTimeoutRef.current = null;
        }, DOUBLE_CLICK_DELAY_MS);
        return;
      }

      // Clear any existing highlight timeout
      if (highlightTimeoutRef.current) {
        clearTimeout(highlightTimeoutRef.current);
        highlightTimeoutRef.current = null;
      }

      setGraphSelection({ kind: "task", id: node.id });
      // Set the highlighted task (focus the node)
      setHighlightedTaskId(node.id);
      setFocusedNodeId(node.id);

      // Fit + center the view on the clicked node
      focusSelectionInView({ kind: "task", id: node.id });

      // Auto-clear highlight after timeout
      highlightTimeoutRef.current = setTimeout(() => {
        setHighlightedTaskId(null);
        highlightTimeoutRef.current = null;
      }, HIGHLIGHT_TIMEOUT_MS);
    },
    [fitNodeInView, focusSelectionInView, setGraphSelection]
  );

  // Double-click on nodes - open task detail (for tasks) or collapse/expand (for groups)
  const handleNodeDoubleClick = useCallback(
    (_: React.MouseEvent, node: Node) => {
      // Clear any highlight since we're opening detail
      if (highlightTimeoutRef.current) {
        clearTimeout(highlightTimeoutRef.current);
        highlightTimeoutRef.current = null;
      }
      if (groupClickTimeoutRef.current) {
        clearTimeout(groupClickTimeoutRef.current);
        groupClickTimeoutRef.current = null;
      }
      setHighlightedTaskId(null);

      // Handle group nodes - collapse/expand
      if (node.type === PLAN_GROUP_NODE_TYPE) {
        const parsed = parseGroupNodeId(node.id);
        const planArtifactId =
          (node.data as PlanGroupData | undefined)?.planArtifactId ??
          (parsed?.kind === "plan" ? parsed.id : undefined);
        if (planArtifactId) {
          handleToggleCollapse(planArtifactId);
        }
        return;
      }
      if (node.type === TIER_GROUP_NODE_TYPE) {
        const parsed = parseGroupNodeId(node.id);
        const tierGroupId =
          (node.data as TierGroupData | undefined)?.tierGroupId ??
          (parsed?.kind === "tier" ? parsed.id : node.id);
        handleToggleTierCollapse(tierGroupId);
        return;
      }

      // Handle task nodes - open detail overlay
      setSelectedTaskId(node.id);
    },
    [handleToggleCollapse, handleToggleTierCollapse, setSelectedTaskId]
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
    clearGraphSelection();
    (reactFlowDomNode ?? containerRef.current)?.focus();
  }, [clearGraphSelection, reactFlowDomNode]);

  const getFirstTaskNode = useCallback(
    (taskIds: string[]): Node | null => {
      const candidates = taskIds
        .map((taskId) => taskNodesById.get(taskId))
        .filter((node): node is Node => Boolean(node));
      if (candidates.length === 0) return null;
      return [...candidates].sort(sortNodesByPosition)[0] ?? null;
    },
    [taskNodesById]
  );

  const getFirstTaskInPlan = useCallback(
    (planArtifactId: string): Node | null => {
      const planGroup = planGroupsById.get(planArtifactId);
      if (!planGroup) return null;
      return getFirstTaskNode(planGroup.taskIds);
    },
    [getFirstTaskNode, planGroupsById]
  );

  const getFirstTaskInTier = useCallback(
    (tierGroupId: string): Node | null => {
      const tierGroup = tierGroupsById.get(tierGroupId);
      if (!tierGroup) return null;
      return getFirstTaskNode(tierGroup.taskIds);
    },
    [getFirstTaskNode, tierGroupsById]
  );

  const getTierGroupNodesForPlan = useCallback(
    (planArtifactId: string): Node[] =>
      tierGroupNodes.filter(
        (node) =>
          (node.data as TierGroupData | undefined)?.planArtifactId === planArtifactId
      ),
    [tierGroupNodes]
  );

  // Handle keyboard navigation
  const handleKeyDownEvent = useCallback(
    (event: {
      key: string;
      metaKey: boolean;
      altKey: boolean;
      shiftKey: boolean;
      preventDefault: () => void;
      nativeEvent?: KeyboardEvent;
    }) => {
      if (hasHandledGraphKey(event)) return;
      const key = event.key;
      const lowerKey = key.toLowerCase();

      if (event.metaKey && ["+", "=", "-", "0", "1"].includes(key)) {
        event.preventDefault();
        if (key === "+" || key === "=") {
          zoomBy(0.1);
          return;
        }
        if (key === "-") {
          zoomBy(-0.1);
          return;
        }
        if (key === "0") {
          fitViewDefault({ padding: 0.2, duration: 200 });
          return;
        }
        if (key === "1") {
          const activeSelection =
            (isNavigableGraphSelection(graphSelection) ? graphSelection : null) ??
            (focusedNodeId
              ? { kind: "task" as const, id: focusedNodeId }
              : selectedTaskId
                ? { kind: "task" as const, id: selectedTaskId }
                : null);
          if (activeSelection) {
            focusSelectionInView(activeSelection);
          } else {
            fitViewDefault({ padding: 0.2, duration: 200 });
          }
          return;
        }
      }

      if (event.altKey && lowerKey === "a") {
        const selection = graphSelection;
        if (!selection) return;
        event.preventDefault();
        if (selection.kind === "planGroup") {
          if (event.shiftKey) {
            const tierIds = tierGroupsByPlan.get(selection.id) ?? [];
            if (tierIds.length === 0) return;
            const shouldExpand = tierIds.some((tierId) => collapsedTierIds.has(tierId));
            handleToggleAllTiers(selection.id, shouldExpand ? "expand" : "collapse");
            return;
          }
          handleToggleCollapse(selection.id);
          return;
        }
        if (selection.kind === "tierGroup") {
          if (event.shiftKey) {
            const planArtifactId = tierGroupsById.get(selection.id)?.planArtifactId;
            if (!planArtifactId) return;
            const tierIds = tierGroupsByPlan.get(planArtifactId) ?? [];
            if (tierIds.length === 0) return;
            const shouldExpand = tierIds.some((tierId) => collapsedTierIds.has(tierId));
            handleToggleAllTiers(planArtifactId, shouldExpand ? "expand" : "collapse");
            return;
          }
          handleToggleTierCollapse(selection.id);
        }
        return;
      }

      if (!["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "Enter", "Escape", "Backspace"].includes(key)) {
        return;
      }

      event.preventDefault();

      if (key === "Escape") {
        setSelectedTaskId(null);
        setFocusedNodeId(null);
        setHighlightedTaskId(null);
        clearGraphSelection();
        if (highlightTimeoutRef.current) {
          clearTimeout(highlightTimeoutRef.current);
          highlightTimeoutRef.current = null;
        }
        return;
      }

      const activeSelection =
        (isNavigableGraphSelection(graphSelection) ? graphSelection : null) ??
        (focusedNodeId
          ? { kind: "task" as const, id: focusedNodeId }
          : selectedTaskId
            ? { kind: "task" as const, id: selectedTaskId }
            : null);

      if (key === "Backspace") {
        if (!activeSelection) return;
        if (activeSelection.kind === "task") {
          const tierGroupId = taskToTierMap.get(activeSelection.id);
          if (tierGroupId) {
            setFocusedNodeId(null);
            setGraphSelection({ kind: "tierGroup", id: tierGroupId });
            focusSelectionInView({ kind: "tierGroup", id: tierGroupId });
            return;
          }
          const planArtifactId = taskToPlanMap.get(activeSelection.id);
          if (planArtifactId) {
            setFocusedNodeId(null);
            setGraphSelection({ kind: "planGroup", id: planArtifactId });
            focusSelectionInView({ kind: "planGroup", id: planArtifactId });
            return;
          }
        }
        if (activeSelection.kind === "tierGroup") {
          const planArtifactId = tierGroupsById.get(activeSelection.id)?.planArtifactId;
          if (planArtifactId) {
            setFocusedNodeId(null);
            setGraphSelection({ kind: "planGroup", id: planArtifactId });
            focusSelectionInView({ kind: "planGroup", id: planArtifactId });
            return;
          }
        }
        if (activeSelection.kind === "planGroup") {
          setFocusedNodeId(null);
          clearGraphSelection();
        }
        return;
      }

      if (key === "Enter") {
        if (!activeSelection) return;
        if (activeSelection.kind === "planGroup") {
          if (collapsedPlanIds.has(activeSelection.id)) {
            handleToggleCollapse(activeSelection.id);
            return;
          }

          if (grouping.byTier) {
            const tierNodes = getTierGroupNodesForPlan(activeSelection.id)
              .sort(sortNodesByPosition);
            if (tierNodes.length > 0) {
              const tierIds = tierGroupsByPlan.get(activeSelection.id) ?? [];
              if (tierIds.some((tierId) => collapsedTierIds.has(tierId))) {
                handleToggleAllTiers(activeSelection.id, "expand");
              }
              return;
            }
          }

          const firstTask = getFirstTaskInPlan(activeSelection.id);
          if (firstTask) {
            setGraphSelection({ kind: "task", id: firstTask.id });
            setFocusedNodeId(firstTask.id);
            focusSelectionInView({ kind: "task", id: firstTask.id });
          }
          return;
        }

        if (activeSelection.kind === "tierGroup") {
          if (collapsedTierIds.has(activeSelection.id)) {
            handleToggleTierCollapse(activeSelection.id);
            setTimeout(() => {
              const firstTask = getFirstTaskInTier(activeSelection.id);
              if (firstTask) {
                setGraphSelection({ kind: "task", id: firstTask.id });
                setFocusedNodeId(firstTask.id);
                centerOnNode(firstTask.id, { duration: 300, zoom: 1.1, fallbackWidth: 180, fallbackHeight: 60 });
              }
            }, 80);
            return;
          }

          const firstTask = getFirstTaskInTier(activeSelection.id);
          if (firstTask) {
            setGraphSelection({ kind: "task", id: firstTask.id });
            setFocusedNodeId(firstTask.id);
            focusSelectionInView({ kind: "task", id: firstTask.id });
          }
          return;
        }

        if (activeSelection.kind === "task") {
          setSelectedTaskId(activeSelection.id);
        }
        return;
      }

      const direction = key === "ArrowUp"
        ? "up"
        : key === "ArrowDown"
          ? "down"
          : key === "ArrowLeft"
            ? "left"
            : "right";

      if (!activeSelection) {
        if (planGroupNodesSorted.length > 0) {
          const node =
            direction === "up" ? planGroupNodesSorted[planGroupNodesSorted.length - 1] : planGroupNodesSorted[0];
          const planArtifactId =
            (node?.data as PlanGroupData | undefined)?.planArtifactId ?? null;
          if (planArtifactId) {
            setGraphSelection({ kind: "planGroup", id: planArtifactId });
            focusSelectionInView({ kind: "planGroup", id: planArtifactId });
          }
          return;
        }

        const firstTask = layoutNodes[0];
        if (firstTask) {
          setGraphSelection({ kind: "task", id: firstTask.id });
          setFocusedNodeId(firstTask.id);
          focusSelectionInView({ kind: "task", id: firstTask.id });
        }
        return;
      }

      if (activeSelection.kind === "planGroup") {
        if (direction === "left") {
          if (!collapsedPlanIds.has(activeSelection.id)) {
            handleToggleCollapse(activeSelection.id);
            return;
          }
        }
        if (direction === "right") {
          if (collapsedPlanIds.has(activeSelection.id)) {
            handleToggleCollapse(activeSelection.id);
            return;
          }
          if (grouping.byTier && !collapsedPlanIds.has(activeSelection.id)) {
            const tierNodes = getTierGroupNodesForPlan(activeSelection.id)
              .sort(sortNodesByPosition);
            const firstTier = tierNodes[0];
            const tierGroupId = firstTier
              ? (firstTier.data as TierGroupData | undefined)?.tierGroupId
              : null;
            if (tierGroupId) {
              setGraphSelection({ kind: "tierGroup", id: tierGroupId });
              focusSelectionInView({ kind: "tierGroup", id: tierGroupId });
              return;
            }
          }
          const firstTask = getFirstTaskInPlan(activeSelection.id);
          if (firstTask) {
            setGraphSelection({ kind: "task", id: firstTask.id });
            setFocusedNodeId(firstTask.id);
            focusSelectionInView({ kind: "task", id: firstTask.id });
          }
          return;
        }
        const currentNodeId = getPlanGroupNodeId(activeSelection.id);
        const nextNodeId = findNextGroupNode(direction, currentNodeId, planGroupNodes);
        if (nextNodeId) {
          const nextNode = planGroupNodes.find((node) => node.id === nextNodeId);
          const planArtifactId =
            (nextNode?.data as PlanGroupData | undefined)?.planArtifactId ?? null;
          if (planArtifactId) {
            setGraphSelection({ kind: "planGroup", id: planArtifactId });
            focusSelectionInView({ kind: "planGroup", id: planArtifactId });
          }
        }
        return;
      }

      if (activeSelection.kind === "tierGroup") {
        const planArtifactId = tierGroupsById.get(activeSelection.id)?.planArtifactId;
        if (direction === "left") {
          if (!collapsedTierIds.has(activeSelection.id)) {
            handleToggleTierCollapse(activeSelection.id);
            return;
          }
          if (!planArtifactId) {
            return;
          }
          setGraphSelection({ kind: "planGroup", id: planArtifactId });
          focusSelectionInView({ kind: "planGroup", id: planArtifactId });
          return;
        }
        if (direction === "right") {
          if (collapsedTierIds.has(activeSelection.id)) {
            handleToggleTierCollapse(activeSelection.id);
            return;
          }
          const firstTask = getFirstTaskInTier(activeSelection.id);
          if (firstTask) {
            setGraphSelection({ kind: "task", id: firstTask.id });
            setFocusedNodeId(firstTask.id);
            focusSelectionInView({ kind: "task", id: firstTask.id });
          }
          return;
        }
        const tierNodes = planArtifactId
          ? getTierGroupNodesForPlan(planArtifactId)
          : tierGroupNodes;
        const currentNodeId = getTierGroupNodeId(activeSelection.id);
        const nextNodeId = findNextGroupNode(direction, currentNodeId, tierNodes);
        if (nextNodeId) {
          const nextNode = tierNodes.find((node) => node.id === nextNodeId);
          const tierGroupId =
            (nextNode?.data as TierGroupData | undefined)?.tierGroupId ?? null;
          if (tierGroupId) {
            setGraphSelection({ kind: "tierGroup", id: tierGroupId });
            focusSelectionInView({ kind: "tierGroup", id: tierGroupId });
          }
        }
        return;
      }

      const nextNodeId = findNextNode(direction, activeSelection.id, nodes, edges);
      if (nextNodeId) {
        setFocusedNodeId(nextNodeId);
        setGraphSelection({ kind: "task", id: nextNodeId });
        focusSelectionInView({ kind: "task", id: nextNodeId });
      } else if (direction === "left") {
        const tierGroupId = taskToTierMap.get(activeSelection.id);
        if (tierGroupId) {
          setFocusedNodeId(null);
          setGraphSelection({ kind: "tierGroup", id: tierGroupId });
          focusSelectionInView({ kind: "tierGroup", id: tierGroupId });
          return;
        }
        const planArtifactId = taskToPlanMap.get(activeSelection.id);
        if (planArtifactId) {
          setFocusedNodeId(null);
          setGraphSelection({ kind: "planGroup", id: planArtifactId });
          focusSelectionInView({ kind: "planGroup", id: planArtifactId });
        }
      }
    },
    [
      centerOnNode,
      clearGraphSelection,
      collapsedPlanIds,
      collapsedTierIds,
      edges,
      fitViewDefault,
      focusSelectionInView,
      focusedNodeId,
      getFirstTaskInPlan,
      getFirstTaskInTier,
      getTierGroupNodesForPlan,
      graphSelection,
      grouping.byTier,
      handleToggleAllTiers,
      handleToggleCollapse,
      handleToggleTierCollapse,
      layoutNodes,
      nodes,
      planGroupNodes,
      planGroupNodesSorted,
      selectedTaskId,
      setSelectedTaskId,
      taskToPlanMap,
      taskToTierMap,
      tierGroupNodes,
      tierGroupsById,
      tierGroupsByPlan,
      zoomBy,
    ]
  );

  const handleKeyDown = useCallback(
    (event: ReactKeyboardEvent<HTMLDivElement>) => {
      handleKeyDownEvent(event);
    },
    [handleKeyDownEvent]
  );

  useEffect(() => {
    if (!reactFlowDomNode) return;
    reactFlowDomNode.setAttribute("tabindex", "0");
    const handleDomKeyDown = (event: KeyboardEvent) => {
      if (isEditableTarget(event.target)) return;
      handleKeyDownEvent(event);
    };
    reactFlowDomNode.addEventListener("keydown", handleDomKeyDown, { capture: true });
    return () => {
      reactFlowDomNode.removeEventListener("keydown", handleDomKeyDown, { capture: true });
    };
  }, [handleKeyDownEvent, reactFlowDomNode]);

  // Cleanup timeout on unmount
  useEffect(() => {
    return () => {
      if (highlightTimeoutRef.current) {
        clearTimeout(highlightTimeoutRef.current);
      }
    };
  }, []);

  useEffect(() => {
    if (!graphData || isLoading || error) return;
    const container = containerRef.current;
    if (!container) return;
    const focusTarget = reactFlowDomNode ?? container;
    const active = document.activeElement;
    if (active && active !== document.body && active !== focusTarget) return;
    focusTarget.focus();
  }, [error, graphData, isLoading, reactFlowDomNode]);

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
        ref={containerRef}
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
