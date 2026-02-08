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

import { useQueryClient } from "@tanstack/react-query";
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
import { useDeleteIdeationSession } from "@/hooks/useIdeation";
import { useConfirmation } from "@/hooks/useConfirmation";
import { useNavCompactBreakpoint } from "@/hooks";
import { useIdeationStore } from "@/stores/ideationStore";
import { useChatStore } from "@/stores/chatStore";
import { taskGraphKeys } from "./hooks/useTaskGraph";
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

function areSetsEqual(a: Set<string>, b: Set<string>): boolean {
  if (a.size !== b.size) return false;
  for (const value of a) {
    if (!b.has(value)) return false;
  }
  return true;
}

/**
 * Apply filters to graph data
 * Filters nodes based on status and plan selections.
 * Archived filtering is handled by the backend via `include_archived` param.
 * Filters edges to only include those where both source and target are visible.
 */
function applyGraphFilters(
  nodes: TaskGraphNode[],
  edges: TaskGraphEdge[],
  planGroups: PlanGroupInfo[],
  filters: GraphFilters
): { nodes: TaskGraphNode[]; edges: TaskGraphEdge[]; planGroups: PlanGroupInfo[] } {
  // Short-circuit: if no client-side filters are active, return original references
  const noFilters = filters.statuses.length === 0
    && filters.planIds.length === 0;
  if (noFilters) {
    return { nodes, edges, planGroups };
  }

  // Filter nodes
  const filteredNodes = nodes.filter((node) => {
    const status = node.internalStatus as InternalStatus;

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
  // GraphControls state (declared early so showArchived is available for useTaskGraph)
  const [filters, setFilters] = useState<GraphFilters>(DEFAULT_GRAPH_FILTERS);

  const { data: graphData, isLoading, error } = useTaskGraph(projectId, filters.showArchived);
  const {
    fitNodeInView,
    centerOnNode,
    centerOnNodeObject,
    fitViewDefault,
    zoomBy,
  } = useTaskGraphViewport();
  const setSelectedTaskId = useUiStore((s) => s.setSelectedTaskId);
  const graphRightPanelUserOpen = useUiStore((s) => s.graphRightPanelUserOpen);
  const graphRightPanelCompactOpen = useUiStore((s) => s.graphRightPanelCompactOpen);
  const { isNavCompact } = useNavCompactBreakpoint();
  const queryClient = useQueryClient();
  const deleteSessionMutation = useDeleteIdeationSession();
  const { confirm, confirmationDialogProps, ConfirmationDialog } = useConfirmation();
  const removeSession = useIdeationStore((s) => s.removeSession);
  const clearMessages = useChatStore((s) => s.clearMessages);
  const clearGraphSelection = useUiStore((s) => s.clearGraphSelection);
  const [overlayClosing, setOverlayClosing] = useState(false);
  const overlayCloseTimeoutRef = useRef<number | null>(null);
  const pendingTierAutoCenterRef = useRef<string | null>(null);
  const lastAutoCenteredPlanRef = useRef<string | null>(null);
  const expandedPlanIdRef = useRef<string | null>(null);
  const initialFitDoneRef = useRef(false);

  const graphReady = Boolean(graphData && graphData.nodes.length > 0);

  // Collapse state for plan groups
  const [collapsedPlanIds, setCollapsedPlanIds] = useState<Set<string>>(
    new Set()
  );
  const [collapsedTierIds, setCollapsedTierIds] = useState<Set<string>>(
    new Set()
  );

  const [grouping, setGrouping] = useState<GroupingState>(DEFAULT_GROUPING);

  // ---- Synchronous collapsed-state derivation --------------------------------
  // React "adjust state during render" pattern: compute which plan/tier groups
  // should be collapsed BEFORE children render.  This ensures dagre receives
  // the correct collapsed set on the very first committed render, eliminating
  // the visible "nodes flying into place" flash on initial load.
  // See: react.dev/reference/react/useState#storing-information-from-previous-renders

  // Shared: determine which plan group should be expanded by default
  const { expandedPlanId, planGroupInfos } = useMemo(() => {
    if (!graphData) {
      return {
        expandedPlanId: null as string | null,
        planGroupInfos: [] as Array<{ id: string; total: number; completed: number }>,
      };
    }

    const pgs = graphData.planGroups ?? [];
    const allNodes = graphData.nodes ?? [];

    const groupedTaskIds = new Set<string>();
    for (const pg of pgs) {
      for (const taskId of pg.taskIds) {
        groupedTaskIds.add(taskId);
      }
    }
    const ungroupedTasks = allNodes.filter((n) => !groupedTaskIds.has(n.taskId));

    const allGroups: Array<{ id: string; total: number; completed: number }> = [];
    for (const pg of pgs) {
      allGroups.push({
        id: pg.planArtifactId,
        total: pg.taskIds.length,
        completed: pg.statusSummary.completed,
      });
    }
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

    if (allGroups.length === 0) {
      return { expandedPlanId: null as string | null, planGroupInfos: allGroups };
    }

    let expandedGroupId: string | null = null;
    for (const g of allGroups) {
      if (g.total === 0 || g.completed < g.total) {
        expandedGroupId = g.id;
        break;
      }
    }
    if (!expandedGroupId) {
      expandedGroupId = allGroups[allGroups.length - 1]?.id ?? null;
    }

    return { expandedPlanId: expandedGroupId, planGroupInfos: allGroups };
  }, [graphData]);

  // Keep ref in sync (used by initial-fit effect)
  useEffect(() => {
    expandedPlanIdRef.current = expandedPlanId;
  }, [expandedPlanId]);

  // Plan group collapse: recompute defaults synchronously when graphData changes
  const [prevPlanCollapseData, setPrevPlanCollapseData] = useState<typeof graphData>(undefined);
  if (graphData !== prevPlanCollapseData) {
    setPrevPlanCollapseData(graphData);
    if (planGroupInfos.length > 0 && expandedPlanId !== null) {
      const toCollapse = new Set(
        planGroupInfos.filter((g) => g.id !== expandedPlanId).map((g) => g.id)
      );
      setCollapsedPlanIds(toCollapse);
    } else {
      setCollapsedPlanIds(new Set());
    }
  }

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

  // Tier group collapse: recompute defaults synchronously when inputs change
  let tierAutoCenterCandidate: string | null = null;
  const [prevTierCollapseInputs, setPrevTierCollapseInputs] = useState<{
    graphData: typeof graphData;
    byPlan: boolean;
    byTier: boolean;
    showUncategorized: boolean;
  } | null>(null);

  const tierInputsChanged = prevTierCollapseInputs === null
    || graphData !== prevTierCollapseInputs.graphData
    || grouping.byPlan !== prevTierCollapseInputs.byPlan
    || grouping.byTier !== prevTierCollapseInputs.byTier
    || grouping.showUncategorized !== prevTierCollapseInputs.showUncategorized;

  if (tierInputsChanged) {
    setPrevTierCollapseInputs({
      graphData,
      byPlan: grouping.byPlan,
      byTier: grouping.byTier,
      showUncategorized: grouping.showUncategorized,
    });

    if (!graphData || !grouping.byTier) {
      setCollapsedTierIds((prev) => prev.size > 0 ? new Set<string>() : prev);
    } else {
      const pgs = graphData.planGroups ?? [];
      const allNodes = graphData.nodes ?? [];
      const computedTierGroups = buildTierGroups(allNodes, pgs, {
        enabled: grouping.byTier,
        includeUngrouped: grouping.showUncategorized || !grouping.byPlan,
      });

      if (computedTierGroups.length === 0) {
        setCollapsedTierIds((prev) => prev.size > 0 ? new Set<string>() : prev);
      } else {
        const completedStatuses = new Set(["approved", "merged", "completed"]);
        const nodeMap = new Map(allNodes.map((node) => [node.taskId, node]));
        const tiersByPlanLocal = new Map<string, typeof computedTierGroups>();

        for (const tg of computedTierGroups) {
          const existing = tiersByPlanLocal.get(tg.planArtifactId);
          if (existing) {
            existing.push(tg);
          } else {
            tiersByPlanLocal.set(tg.planArtifactId, [tg]);
          }
        }

        const toCollapseTiers = new Set<string>();
        for (const [planArtifactId, tiers] of tiersByPlanLocal) {
          // Match "Expand tiers" behavior for the default active plan group.
          if (planArtifactId === expandedPlanId) continue;

          const tierInfos = tiers.map((tg) => {
            let completed = 0;
            for (const taskId of tg.taskIds) {
              const node = nodeMap.get(taskId);
              if (node && completedStatuses.has(node.internalStatus)) {
                completed += 1;
              }
            }
            return { id: tg.id, total: tg.taskIds.length, completed };
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

        setCollapsedTierIds((prev) => areSetsEqual(prev, toCollapseTiers) ? prev : toCollapseTiers);

        // Track whether expandedPlanId has tiers (used by effect below for auto-center)
        if (expandedPlanId && tiersByPlanLocal.has(expandedPlanId)) {
          tierAutoCenterCandidate = expandedPlanId;
        }
      }
    }
  }

  // Schedule auto-center via effect (refs must not be accessed during render)
  useEffect(() => {
    if (
      tierAutoCenterCandidate &&
      initialFitDoneRef.current &&
      tierAutoCenterCandidate !== lastAutoCenteredPlanRef.current
    ) {
      pendingTierAutoCenterRef.current = tierAutoCenterCandidate;
    }
  });

  // ---- End synchronous collapsed-state derivation ----------------------------

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

  // Shared viewport centering: try fitNodeInView → centerOnPlanGroup → fitViewDefault
  const centerOnPlanGroupNode = useCallback(
    (planArtifactId: string, duration = 200) => {
      if (fitNodeInView(getPlanGroupNodeId(planArtifactId), { duration, padding: 0.18, maxZoom: 0.95 })) return;
      if (centerOnPlanGroup(planArtifactId, duration, 0.9)) return;
      fitViewDefault({ padding: 0.2, duration });
    },
    [centerOnPlanGroup, fitNodeInView, fitViewDefault]
  );

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

  // Toggle collapse state for a plan group
  // When expanding, also expand ALL tier groups within it for full visibility
  // When collapsing, also collapse all tier groups for a clean state
  const handleToggleCollapse = useCallback((planArtifactId: string) => {
    let expanding = false;
    setCollapsedPlanIds((prev) => {
      const next = new Set(prev);
      if (next.has(planArtifactId)) {
        next.delete(planArtifactId);
        expanding = true;
      } else {
        next.add(planArtifactId);
      }
      return next;
    });
    const tierIds = tierGroupsByPlan.get(planArtifactId) ?? [];
    if (tierIds.length > 0) {
      setCollapsedTierIds((prev) => {
        const next = new Set(prev);
        if (expanding) {
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
    }
    setTimeout(() => centerOnPlanGroupNode(planArtifactId), 50);
  }, [centerOnPlanGroupNode, tierGroupsByPlan]);

  const handleToggleTierCollapse = useCallback((tierGroupId: string) => {
    let expanding = false;
    const planArtifactId = tierGroupsById.get(tierGroupId)?.planArtifactId ?? null;
    setCollapsedTierIds((prev) => {
      const next = new Set(prev);
      if (next.has(tierGroupId)) {
        next.delete(tierGroupId);
        expanding = true;
      } else {
        next.add(tierGroupId);
      }
      return next;
    });
    setTimeout(() => {
      if (expanding) {
        // Expanding tier → center on the tier group
        const tierNodeId = getTierGroupNodeId(tierGroupId);
        if (fitNodeInView(tierNodeId, { duration: 200, padding: 0.18, maxZoom: 0.95 })) return;
        if (centerOnNode(tierNodeId, { duration: 200, zoom: 0.95, fallbackWidth: 320, fallbackHeight: 80 })) return;
      }
      // Collapsing tier → center on parent plan group
      if (planArtifactId) {
        centerOnPlanGroupNode(planArtifactId);
        return;
      }
      fitViewDefault({ padding: 0.2, duration: 200 });
    }, 50);
  }, [centerOnNode, centerOnPlanGroupNode, fitNodeInView, fitViewDefault, tierGroupsById]);

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
      setTimeout(() => centerOnPlanGroupNode(planArtifactId), 50);
    },
    [centerOnPlanGroupNode, tierGroupsByPlan]
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

  // Plan deletion handler (Backspace on plan group or Delete button in settings)
  const handleDeletePlan = useCallback(
    async (planArtifactId: string) => {
      const planGroup = graphData?.planGroups.find(
        (pg) => pg.planArtifactId === planArtifactId
      );
      if (!planGroup) return;

      const confirmed = await confirm({
        title: "Delete plan?",
        description: `This will permanently delete "${planGroup.sessionTitle || "Unnamed plan"}" and all ${planGroup.taskIds.length} task${planGroup.taskIds.length === 1 ? "" : "s"}. This action cannot be undone.`,
        confirmText: "Delete",
        variant: "destructive",
      });

      if (!confirmed) return;

      try {
        await deleteSessionMutation.mutateAsync(planGroup.sessionId);
        removeSession(planGroup.sessionId);
        clearMessages(`session:${planGroup.sessionId}`);
        clearGraphSelection();
        queryClient.invalidateQueries({ queryKey: taskGraphKeys.graphPrefix(projectId) });
        toast.success("Plan deleted");
      } catch {
        toast.error("Failed to delete plan");
      }
    },
    [graphData?.planGroups, confirm, deleteSessionMutation, removeSession, clearMessages, clearGraphSelection, queryClient, projectId]
  );

  // Task deletion handler (Delete key on task node)
  const handleDeleteTask = useCallback(
    async (taskId: string) => {
      const taskNode = graphData?.nodes.find((n) => n.taskId === taskId);
      const taskTitle = taskNode?.title ?? "this task";

      const confirmed = await confirm({
        title: "Delete task?",
        description: `This will permanently delete "${taskTitle}". This action cannot be undone.`,
        confirmText: "Delete",
        variant: "destructive",
      });

      if (!confirmed) return;

      try {
        await api.tasks.delete(taskId);
        clearGraphSelection();
        queryClient.invalidateQueries({ queryKey: taskGraphKeys.graphPrefix(projectId) });
        toast.success("Task deleted");
      } catch {
        toast.error("Failed to delete task");
      }
    },
    [graphData?.nodes, confirm, clearGraphSelection, queryClient, projectId]
  );

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
    handleToggleAllTiers,
    projectId,
    handleViewDetails,
    handleDeletePlan
  );

  useEffect(() => {
    const pendingPlanId = pendingTierAutoCenterRef.current;
    if (!pendingPlanId) return;
    if (isLoading || !graphReady) return;
    if (!groupNodes.some((node) => node.id === getPlanGroupNodeId(pendingPlanId))) {
      return;
    }
    pendingTierAutoCenterRef.current = null;
    lastAutoCenteredPlanRef.current = pendingPlanId;
    requestAnimationFrame(() => {
      requestAnimationFrame(() => centerOnPlanGroupNode(pendingPlanId));
    });
  }, [centerOnPlanGroupNode, graphReady, groupNodes, isLoading]);

  const {
    focusedNodeId,
    highlightedTaskId,
    graphSelection,
    select,
    focusSelectionInView,
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
    centerOnNode,
    centerOnNodeObject,
    fitViewDefault,
    zoomBy,
    graphReady,
    graphError: error ?? null,
    isLoading,
    onDeletePlanGroup: handleDeletePlan,
    onDeleteTask: handleDeleteTask,
  });

  // Compute visible nodes and edges (controlled mode).
  // React Flow v12 StoreUpdater triggers setNodes() whenever the nodes array
  // reference changes. Upstream hooks produce new array references even when
  // content is unchanged (web mode mock layer triggers this). We stabilize
  // by comparing node IDs and visual state, returning the previous array ref
  // when structurally unchanged to prevent infinite re-render loops.
  // Uses synchronous state derivation (setState-during-render) to avoid ref
  // access during render which violates react-hooks/refs.

  const nextNodesCandidate = useMemo<Node[]>(() => {
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

  // Stabilize nodes: only update state when structurally changed
  const [nodes, setNodes] = useState<Node[]>([]);
  const [prevNodesCandidate, setPrevNodesCandidate] = useState(nextNodesCandidate);
  if (nextNodesCandidate !== prevNodesCandidate) {
    setPrevNodesCandidate(nextNodesCandidate);
    // Lightweight identity check — IDs, positions, visual state, task data.
    // Must include positions so dagre re-layout after expand/collapse propagates.
    // Must include internalStatus so real-time status changes propagate to nodes.
    const prev = nodes;
    const next = nextNodesCandidate;
    let changed = next.length !== prev.length;
    if (!changed) {
      for (let i = 0; i < next.length; i++) {
        const n = next[i]!;
        const p = prev[i]!;
        const nd = n.data as Record<string, unknown>;
        const pd = p.data as Record<string, unknown>;
        if (
          n.id !== p.id
          || n.position.x !== p.position.x
          || n.position.y !== p.position.y
          || n.selected !== p.selected
          || n.type !== p.type
          || nd?.isSelected !== pd?.isSelected
          || nd?.isHighlighted !== pd?.isHighlighted
          || nd?.isFocused !== pd?.isFocused
          || nd?.internalStatus !== pd?.internalStatus
          || nd?.label !== pd?.label
        ) {
          changed = true;
          break;
        }
      }
    }
    if (changed) {
      setNodes(next);
    }
  }

  // Edges: stabilize reference with lightweight identity check
  const nextEdgesCandidate = useMemo<Edge[]>(() => layoutEdges, [layoutEdges]);
  const [edges, setEdges] = useState<Edge[]>([]);
  const [prevEdgesCandidate, setPrevEdgesCandidate] = useState(nextEdgesCandidate);
  if (nextEdgesCandidate !== prevEdgesCandidate) {
    setPrevEdgesCandidate(nextEdgesCandidate);
    const prev = edges;
    if (nextEdgesCandidate.length === prev.length && nextEdgesCandidate.every((e, i) => e.id === prev[i]!.id)) {
      // unchanged
    } else {
      setEdges(nextEdgesCandidate);
    }
  }

  // Handle node changes (for selection, dragging etc.) in controlled mode
  const onNodesChange: OnNodesChange = useCallback(() => {
    // We don't allow user-driven node changes (positions come from dagre layout)
    // Selection is handled via onNodeClick
  }, []);

  // Handle edge changes in controlled mode
  const onEdgesChange: OnEdgesChange = useCallback(() => {
    // We don't allow user-driven edge changes (edges are computed from dependencies)
  }, []);

  // One-time initial select + center on expanded group after dagre layout is ready.
  // Polls until the group node is measured by React Flow, then selects it via
  // the controller's stable `select()` API. Falls back to fitViewDefault.
  // Hide ReactFlow until initial viewport positioning is complete.
  // Prevents visible flash of default viewport before centering on expanded group.
  const [initialViewReady, setInitialViewReady] = useState(false);
  useEffect(() => {
    if (initialFitDoneRef.current) return;
    if (!graphReady || layoutNodes.length === 0) return;
    const hasPositions = layoutNodes.some((n) => n.position.x !== 0 || n.position.y !== 0);
    if (!hasPositions && groupNodes.length === 0) return;
    initialFitDoneRef.current = true;
    // Cancel any pending tier auto-center — initial fit owns viewport positioning
    pendingTierAutoCenterRef.current = null;

    const expandedId = expandedPlanIdRef.current;
    if (expandedId) {
      lastAutoCenteredPlanRef.current = expandedId;
    }

    // Extra frame after positioning to let React Flow paint at correct viewport
    const settle = () => {
      requestAnimationFrame(() => setInitialViewReady(true));
    };

    const attemptFocus = (attempt: number) => {
      if (attempt > 15) {
        fitViewDefault({ padding: 0.2, duration: 0 });
        settle();
        return;
      }
      if (expandedId) {
        const nodeId = getPlanGroupNodeId(expandedId);
        if (fitNodeInView(nodeId, { duration: 0, padding: 0.18, maxZoom: 0.95 })) {
          centerOnPlanGroup(expandedId, 0, 0.9);
          select({ kind: "planGroup", id: expandedId }, { skipFocus: true });
          settle();
          return;
        }
      } else {
        fitViewDefault({ padding: 0.2, duration: 0 });
        settle();
        return;
      }
      requestAnimationFrame(() => attemptFocus(attempt + 1));
    };
    requestAnimationFrame(() => {
      requestAnimationFrame(() => attemptFocus(0));
    });
  }, [graphReady, layoutNodes, groupNodes, fitNodeInView, centerOnPlanGroup, fitViewDefault, select]);

  const graphRightPanelVisible = isNavCompact
    ? graphRightPanelCompactOpen
    : graphRightPanelUserOpen;
  const rightPanelMode = !graphRightPanelVisible
    ? "hidden"
    : isNavCompact
      ? "overlay"
      : "split";

  // Re-center current selection when selection or panel layout changes.
  // Uses ref to avoid re-firing when focusSelectionInView ref changes.
  // Skips on first graphSelection change (initial load handles its own positioning).
  const focusInViewRef = useRef(focusSelectionInView);
  useEffect(() => {
    focusInViewRef.current = focusSelectionInView;
  }, [focusSelectionInView]);
  const prevGraphSelectionRef = useRef<typeof graphSelection>(undefined);

  useEffect(() => {
    if (!graphSelection) return;
    const isFirstSelection = prevGraphSelectionRef.current === undefined;
    prevGraphSelectionRef.current = graphSelection;
    if (isFirstSelection) return;
    focusInViewRef.current(graphSelection);
  }, [graphSelection, graphRightPanelVisible, isNavCompact]);

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

  // Check if client-side filters might hide tasks (showArchived is a backend filter, not counted here)
  const hasActiveFilters =
    filters.statuses.length > 0 ||
    filters.planIds.length > 0;

  return (
    <>
    <ConfirmationDialog {...confirmationDialogProps} />
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
      {/* Graph canvas container — hidden until initial viewport positioning completes */}
      <div
        className="h-full w-full relative outline-none"
        data-testid="task-graph-view"
        tabIndex={0}
        ref={containerRef}
        onKeyDown={onKeyDown}
        style={initialViewReady ? undefined : { visibility: "hidden" }}
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
    </>
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
