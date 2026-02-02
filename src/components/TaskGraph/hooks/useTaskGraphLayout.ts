/**
 * useTaskGraphLayout hook - Dagre-based hierarchical layout for task graph
 *
 * Uses dagre algorithm to compute node positions for a proper hierarchical layout
 * with configurable spacing and direction. Supports plan grouping with visual
 * region containers.
 *
 * Layout Caching: Dagre computation is expensive. We cache layouts by a structural
 * hash (node IDs + edge pairs + config). When only node data changes (e.g., status)
 * but structure is the same, we reuse cached positions and just update node data.
 */

import { useMemo, useRef } from "react";
import dagre from "@dagrejs/dagre";
import { Position, type Node, type Edge } from "@xyflow/react";
import type { TaskGraphNode, TaskGraphEdge, PlanGroupInfo } from "@/api/task-graph.types";
import {
  calculateGroupBoundingBoxes,
  expandBoundingBox,
  boundingBoxToGroupNode,
  GROUP_PADDING,
  HEADER_HEIGHT,
} from "../groups/groupUtils";
import { createPlanGroupNode, type PlanGroupNode } from "../groups/PlanGroup";
import { NODE_WIDTH, NODE_HEIGHT, COMPACT_NODE_WIDTH, COMPACT_NODE_HEIGHT } from "../nodes/nodeStyles";

// ============================================================================
// Types
// ============================================================================

export interface LayoutConfig {
  /** Layout direction: TB (top-to-bottom) or LR (left-to-right) */
  direction: "TB" | "LR";
  /** Horizontal spacing between nodes */
  nodesep: number;
  /** Vertical spacing between ranks/tiers */
  ranksep: number;
  /** Horizontal margin */
  marginx: number;
  /** Vertical margin */
  marginy: number;
  /** Whether to use compact node dimensions */
  isCompact?: boolean;
}

export interface LayoutResult {
  nodes: Node[];
  edges: Edge[];
  groupNodes: PlanGroupNode[];
}

// Use Record<string, unknown> compatible structure for React Flow
// This matches TaskNodeData from nodes/TaskNode.tsx
type TaskNodeData = Record<string, unknown> & {
  label: string;
  taskId: string;
  internalStatus: string;
  priority: number;
  isCriticalPath: boolean;
  description?: string | null;
  category?: string;
};

// Edge data for custom DependencyEdge component
type DependencyEdgeData = Record<string, unknown> & {
  isCriticalPath: boolean;
  sourceStatus?: string;
  /** Whether edge crosses plan group boundaries */
  isCrossPlan?: boolean;
  /** Source task title for tooltip */
  sourceLabel?: string;
  /** Target task title for tooltip */
  targetLabel?: string;
  /** Whether this is a synthetic edge connecting groups */
  isGroupConnector?: boolean;
};

// ============================================================================
// Default Configuration
// ============================================================================

const DEFAULT_CONFIG: LayoutConfig = {
  direction: "TB",
  nodesep: 60,
  ranksep: 80,
  marginx: 40,
  marginy: 40,
};


// ============================================================================
// Group Node Creation
// ============================================================================

/**
 * ID for the "Ungrouped" pseudo-plan that contains standalone tasks
 */
const UNGROUPED_PLAN_ID = "__ungrouped__";

/** Collapsed group dimensions */
const COLLAPSED_GROUP_WIDTH = 320; // Min width to accommodate title + progress
const COLLAPSED_GROUP_HEIGHT = HEADER_HEIGHT + 8;

/**
 * Create React Flow group nodes for plan groups
 *
 * @param taskNodes - Positioned task nodes from dagre layout
 * @param planGroups - Plan group info from the API
 * @param collapsedPlanIds - Set of plan artifact IDs that are collapsed
 * @param positions - Position map from dagre (includes placeholder positions for collapsed groups)
 * @param onToggleCollapse - Callback when collapse is toggled
 * @returns Array of PlanGroupNode for React Flow
 */
function createGroupNodes(
  taskNodes: Node[],
  planGroups: PlanGroupInfo[],
  collapsedPlanIds: Set<string>,
  positions: Map<string, { x: number; y: number }>,
  graphNodes: TaskGraphNode[],
  nodeWidth: number,
  nodeHeight: number,
  onToggleCollapse?: (planArtifactId: string) => void
): PlanGroupNode[] {
  if (planGroups.length === 0) {
    return [];
  }

  // Build map of planArtifactId -> taskIds for bounding box calculation
  const planGroupMap = new Map<string, string[]>();
  for (const pg of planGroups) {
    planGroupMap.set(pg.planArtifactId, pg.taskIds);
  }

  // Find ungrouped tasks (tasks not in any plan group)
  const groupedTaskIds = new Set<string>();
  for (const pg of planGroups) {
    for (const taskId of pg.taskIds) {
      groupedTaskIds.add(taskId);
    }
  }

  const ungroupedTaskIds: string[] = [];
  for (const node of taskNodes) {
    if (!groupedTaskIds.has(node.id)) {
      ungroupedTaskIds.push(node.id);
    }
  }

  // Create group nodes for each plan group
  const groupNodes: PlanGroupNode[] = [];

  // Create a map for quick lookup of plan group info
  const planGroupInfoMap = new Map<string, PlanGroupInfo>();
  for (const pg of planGroups) {
    planGroupInfoMap.set(pg.planArtifactId, pg);
  }

  for (const pg of planGroups) {
    const isCollapsed = collapsedPlanIds.has(pg.planArtifactId);

    let position: { x: number; y: number };
    let width: number;
    let height: number;

    if (isCollapsed) {
      // For collapsed groups, use the placeholder position
      const placeholderPos = positions.get(`__group_position_${pg.planArtifactId}__`);
      position = placeholderPos ?? { x: 0, y: 0 };
      width = COLLAPSED_GROUP_WIDTH;
      height = COLLAPSED_GROUP_HEIGHT;
    } else {
      // For expanded groups, calculate bounding box from task positions
      const groupTaskNodes = taskNodes.filter((n) => pg.taskIds.includes(n.id));
      if (groupTaskNodes.length === 0) continue;

      const singleGroupMap = new Map<string, string[]>();
      singleGroupMap.set(pg.planArtifactId, pg.taskIds);
      const boundingBoxes = calculateGroupBoundingBoxes(
        groupTaskNodes,
        singleGroupMap,
        nodeWidth,
        nodeHeight
      );
      const bbox = boundingBoxes[0];
      if (!bbox) continue;

      const expanded = expandBoundingBox(bbox, GROUP_PADDING, HEADER_HEIGHT);
      const groupDims = boundingBoxToGroupNode(expanded);
      position = groupDims.position;
      width = groupDims.width;
      height = groupDims.height;
    }

    const groupNode = createPlanGroupNode(
      pg.planArtifactId,
      pg.sessionId,
      pg.sessionTitle,
      pg.taskIds,
      pg.statusSummary,
      position,
      width,
      height,
      isCollapsed,
      onToggleCollapse
    );

    groupNodes.push(groupNode);
  }

  // Create "Ungrouped" region for standalone tasks (if any)
  if (ungroupedTaskIds.length > 0) {
    const isUngroupedCollapsed = collapsedPlanIds.has(UNGROUPED_PLAN_ID);

    let position: { x: number; y: number };
    let width: number;
    let height: number;

    if (isUngroupedCollapsed) {
      const placeholderPos = positions.get(`__group_position_${UNGROUPED_PLAN_ID}__`);
      position = placeholderPos ?? { x: 0, y: 0 };
      width = COLLAPSED_GROUP_WIDTH;
      height = COLLAPSED_GROUP_HEIGHT;
    } else {
      const ungroupedTaskNodes = taskNodes.filter((n) => ungroupedTaskIds.includes(n.id));
      if (ungroupedTaskNodes.length === 0) {
        // No ungrouped tasks visible
      } else {
        const ungroupedMap = new Map<string, string[]>();
        ungroupedMap.set(UNGROUPED_PLAN_ID, ungroupedTaskIds);
        const ungroupedBoxes = calculateGroupBoundingBoxes(
          ungroupedTaskNodes,
          ungroupedMap,
          nodeWidth,
          nodeHeight
        );
        const ungroupedBbox = ungroupedBoxes[0];
        if (ungroupedBbox) {
          const expanded = expandBoundingBox(ungroupedBbox, GROUP_PADDING, HEADER_HEIGHT);
          const groupDims = boundingBoxToGroupNode(expanded);
          position = groupDims.position;
          width = groupDims.width;
          height = groupDims.height;
        } else {
          return groupNodes;
        }
      }
    }

    // Calculate StatusSummary for ungrouped tasks from their actual statuses
    const ungroupedSummary = {
      backlog: 0,
      ready: 0,
      blocked: 0,
      executing: 0,
      qa: 0,
      review: 0,
      merge: 0,
      completed: 0,
      terminal: 0,
    };

    // Build status counts from actual task statuses
    for (const taskId of ungroupedTaskIds) {
      const node = graphNodes.find((n) => n.taskId === taskId);
      if (!node) continue;

      const status = node.internalStatus;
      if (status === "backlog") ungroupedSummary.backlog++;
      else if (status === "ready") ungroupedSummary.ready++;
      else if (status === "blocked") ungroupedSummary.blocked++;
      else if (status === "executing" || status === "re_executing") ungroupedSummary.executing++;
      else if (status.startsWith("qa_")) ungroupedSummary.qa++;
      else if (status === "pending_review" || status === "reviewing" || status === "review_passed" || status === "escalated" || status === "revision_needed") ungroupedSummary.review++;
      else if (status === "approved") ungroupedSummary.merge++;
      else if (status === "merged") ungroupedSummary.completed++;
      else if (status === "failed" || status === "cancelled") ungroupedSummary.terminal++;
    }

    const groupNode = createPlanGroupNode(
      UNGROUPED_PLAN_ID,
      "", // No session ID
      "Ungrouped", // Display title
      ungroupedTaskIds,
      ungroupedSummary,
      position!,
      width!,
      height!,
      isUngroupedCollapsed,
      onToggleCollapse
    );

    groupNodes.push(groupNode);
  }

  return groupNodes;
}

// ============================================================================
// Inter-Group Edge Generation
// ============================================================================

/**
 * Inter-group edge info for layout and rendering
 */
interface InterGroupEdgeInfo {
  /** Task node to use for dagre layout (leaf node from source group) */
  layoutSource: string;
  /** Task node to use for dagre layout (root node from target group) */
  layoutTarget: string;
  /** Group ID for rendering (source group) */
  sourceGroupId: string;
  /** Group ID for rendering (target group) */
  targetGroupId: string;
}

/**
 * Generate synthetic edges between consecutive plan groups to stack them vertically.
 * Returns both task-level edges (for dagre layout) and group-level edges (for rendering).
 * Also handles ungrouped tasks as a pseudo-group.
 */
function generateInterGroupEdges(
  graphNodes: TaskGraphNode[],
  planGroups: PlanGroupInfo[]
): InterGroupEdgeInfo[] {
  // Build node lookup maps
  const nodeMap = new Map<string, TaskGraphNode>();
  for (const node of graphNodes) {
    nodeMap.set(node.taskId, node);
  }

  // Find ungrouped tasks (tasks not in any plan group)
  const groupedTaskIds = new Set<string>();
  for (const pg of planGroups) {
    for (const taskId of pg.taskIds) {
      groupedTaskIds.add(taskId);
    }
  }
  const ungroupedTaskIds = graphNodes
    .filter((n) => !groupedTaskIds.has(n.taskId))
    .map((n) => n.taskId);

  // Build all groups including ungrouped pseudo-group
  interface GroupWithTasks {
    id: string;
    taskIds: string[];
  }
  const allGroups: GroupWithTasks[] = planGroups.map((pg) => ({
    id: pg.planArtifactId,
    taskIds: pg.taskIds,
  }));

  // Add ungrouped as a pseudo-group if there are ungrouped tasks
  if (ungroupedTaskIds.length > 0) {
    allGroups.push({
      id: UNGROUPED_PLAN_ID,
      taskIds: ungroupedTaskIds,
    });
  }

  // Need at least 2 groups to create connector edges
  if (allGroups.length < 2) return [];

  // Sort groups by the minimum tier of their tasks (earlier groups first)
  const groupsWithTier = allGroups.map((group) => {
    const tasks = group.taskIds.map((id) => nodeMap.get(id)).filter(Boolean) as TaskGraphNode[];
    const minTier = tasks.length > 0 ? Math.min(...tasks.map((t) => t.tier)) : Infinity;
    return { group, minTier };
  });
  groupsWithTier.sort((a, b) => a.minTier - b.minTier);

  const interGroupEdges: InterGroupEdgeInfo[] = [];

  // For each consecutive pair of groups, create a connector edge
  for (let i = 0; i < groupsWithTier.length - 1; i++) {
    const currentGroupInfo = groupsWithTier[i];
    const nextGroupInfo = groupsWithTier[i + 1];
    if (!currentGroupInfo || !nextGroupInfo) continue;

    const currentGroup = currentGroupInfo.group;
    const nextGroup = nextGroupInfo.group;

    // Find tasks in each group for layout edge
    const currentTasks = currentGroup.taskIds
      .map((id) => nodeMap.get(id))
      .filter(Boolean) as TaskGraphNode[];
    const nextTasks = nextGroup.taskIds
      .map((id) => nodeMap.get(id))
      .filter(Boolean) as TaskGraphNode[];

    if (currentTasks.length === 0 || nextTasks.length === 0) continue;

    // Pick leaf from current group (prefer outDegree === 0, else highest tier)
    const leafCandidates = currentTasks.filter((t) => t.outDegree === 0);
    const leafNode = leafCandidates.length > 0
      ? leafCandidates.reduce((a, b) => (a.tier > b.tier ? a : b))
      : currentTasks.reduce((a, b) => (a.tier > b.tier ? a : b));

    // Pick root from next group (prefer inDegree === 0, else lowest tier)
    const rootCandidates = nextTasks.filter((t) => t.inDegree === 0);
    const rootNode = rootCandidates.length > 0
      ? rootCandidates.reduce((a, b) => (a.tier < b.tier ? a : b))
      : nextTasks.reduce((a, b) => (a.tier < b.tier ? a : b));

    interGroupEdges.push({
      layoutSource: leafNode.taskId,
      layoutTarget: rootNode.taskId,
      sourceGroupId: currentGroup.id,
      targetGroupId: nextGroup.id,
    });
  }

  return interGroupEdges;
}

// ============================================================================
// Layout Computation (with caching)
// ============================================================================

/**
 * Build set of task IDs that belong to collapsed groups.
 * Used for lazy loading - we skip these tasks in layout computation.
 */
function buildCollapsedTaskIds(
  graphNodes: TaskGraphNode[],
  planGroups: PlanGroupInfo[],
  collapsedPlanIds: Set<string>
): Set<string> {
  const hiddenIds = new Set<string>();

  // Add tasks from collapsed plan groups
  for (const pg of planGroups) {
    if (collapsedPlanIds.has(pg.planArtifactId)) {
      for (const taskId of pg.taskIds) {
        hiddenIds.add(taskId);
      }
    }
  }

  // Check for ungrouped tasks if the ungrouped group is collapsed
  if (collapsedPlanIds.has(UNGROUPED_PLAN_ID)) {
    const groupedTaskIds = new Set<string>();
    for (const pg of planGroups) {
      for (const taskId of pg.taskIds) {
        groupedTaskIds.add(taskId);
      }
    }
    for (const node of graphNodes) {
      if (!groupedTaskIds.has(node.taskId)) {
        hiddenIds.add(node.taskId);
      }
    }
  }

  return hiddenIds;
}

/**
 * Compute layout using cached positions if structure unchanged, otherwise recompute.
 *
 * LAZY LOADING: Tasks in collapsed groups are excluded from layout computation entirely.
 * This improves performance by avoiding dagre calculations for hidden nodes.
 * When a group expands, the cache is invalidated (hash changes) and layout is recomputed.
 */
function computeLayoutWithCache(
  graphNodes: TaskGraphNode[],
  graphEdges: TaskGraphEdge[],
  criticalPath: string[],
  planGroups: PlanGroupInfo[],
  config: LayoutConfig,
  collapsedPlanIds: Set<string>,
  onToggleCollapse: ((planArtifactId: string) => void) | undefined,
  cache: React.MutableRefObject<CachedLayout | null>
): LayoutResult {
  // Use correct node dimensions based on compact mode
  const nodeWidth = config.isCompact ? COMPACT_NODE_WIDTH : NODE_WIDTH;
  const nodeHeight = config.isCompact ? COMPACT_NODE_HEIGHT : NODE_HEIGHT;

  // Build set of collapsed task IDs for lazy loading
  const collapsedTaskIds = buildCollapsedTaskIds(graphNodes, planGroups, collapsedPlanIds);

  // Generate inter-group connector edges to stack groups vertically
  const interGroupEdges = generateInterGroupEdges(graphNodes, planGroups);

  // Filter nodes and edges to exclude collapsed groups (lazy loading)
  // This saves rendering for tasks that won't be shown
  const visibleNodes = graphNodes.filter((n) => !collapsedTaskIds.has(n.taskId));
  const visibleEdges = graphEdges.filter(
    (e) => !collapsedTaskIds.has(e.source) && !collapsedTaskIds.has(e.target)
  );

  // Inter-group edges connect group nodes directly, so they're always visible
  // (they don't depend on individual task visibility)

  // Compute layout for ALL nodes (needed for group bounding boxes)
  // We need positions for collapsed group tasks too, so groups remain visible
  const allNodeIds = graphNodes.map((n) => n.taskId);
  // Include inter-group edges for layout computation (so dagre stacks groups)
  // Use task-level edges (layoutSource/layoutTarget) for dagre, not group IDs
  const allEdgePairs = [
    ...graphEdges.map((e) => ({ source: e.source, target: e.target })),
    ...interGroupEdges.map((e) => ({ source: e.layoutSource, target: e.layoutTarget })),
  ];

  // Convert PlanGroupInfo to LayoutPlanGroup for layout computation
  const layoutPlanGroups: LayoutPlanGroup[] = planGroups.map((pg) => ({
    planArtifactId: pg.planArtifactId,
    taskIds: pg.taskIds,
  }));

  const hash = computeGraphHash(allNodeIds, allEdgePairs, config.direction, layoutPlanGroups, collapsedPlanIds);

  // Check if we can use cached positions
  let positions: Map<string, { x: number; y: number }>;
  if (cache.current && cache.current.hash === hash) {
    // Cache hit: reuse positions
    positions = cache.current.positions;
  } else {
    // Cache miss: compute new layout using compound graph and cache it
    // Collapsed groups use minimal placeholder nodes
    positions = computePositions(allNodeIds, allEdgePairs, config, layoutPlanGroups, collapsedPlanIds);
    cache.current = { hash, positions };
  }

  // Create a set of critical path nodes for quick lookup
  const criticalPathSet = new Set(criticalPath);

  // Determine handle positions based on layout direction
  const sourcePosition = config.direction === "TB" ? Position.Bottom : Position.Right;
  const targetPosition = config.direction === "TB" ? Position.Top : Position.Left;

  // Build maps of task ID -> status and task ID -> title for edge data
  const nodeStatusMap = new Map<string, string>();
  const nodeTitleMap = new Map<string, string>();
  graphNodes.forEach((node) => {
    nodeStatusMap.set(node.taskId, node.internalStatus);
    nodeTitleMap.set(node.taskId, node.title);
  });

  // Build a map of task ID -> plan artifact ID for cross-plan edge detection
  const taskToPlanMap = new Map<string, string>();
  for (const pg of planGroups) {
    for (const taskId of pg.taskIds) {
      taskToPlanMap.set(taskId, pg.planArtifactId);
    }
  }

  // Transform to React Flow nodes using cached/computed positions
  // Only process visible nodes (lazy loading - collapsed group tasks excluded)
  const nodes: Node[] = visibleNodes.map((graphNode) => {
    const pos = positions.get(graphNode.taskId) ?? { x: 0, y: 0 };

    return {
      id: graphNode.taskId,
      type: "task", // Use custom TaskNode component
      position: pos,
      data: {
        label: graphNode.title, // Full title - TaskNode handles truncation
        taskId: graphNode.taskId,
        description: graphNode.description,
        category: graphNode.category,
        internalStatus: graphNode.internalStatus,
        priority: graphNode.priority,
        isCriticalPath: criticalPathSet.has(graphNode.taskId),
      } satisfies TaskNodeData,
      sourcePosition,
      targetPosition,
    } as Node;
  });

  // Transform graph edges to React Flow edges
  // Only process visible edges (lazy loading - edges to/from collapsed tasks excluded)
  // Cross-plan edges (source and target in different plan groups) get higher z-index
  // to render on top of plan group regions
  const CROSS_PLAN_EDGE_ZINDEX = 10;

  const edges: Edge[] = visibleEdges.map((graphEdge) => {
    const sourceStatus = nodeStatusMap.get(graphEdge.source);
    const sourcePlan = taskToPlanMap.get(graphEdge.source);
    const targetPlan = taskToPlanMap.get(graphEdge.target);

    // Edge is cross-plan if source and target are in different plan groups
    // (or one is grouped and the other is ungrouped)
    const isCrossPlan = sourcePlan !== targetPlan;

    // Get titles for tooltip
    const sourceLabel = nodeTitleMap.get(graphEdge.source);
    const targetLabel = nodeTitleMap.get(graphEdge.target);

    const edgeData: DependencyEdgeData = {
      isCriticalPath: graphEdge.isCriticalPath,
      isCrossPlan,
    };
    // Only add optional fields if they exist
    if (sourceStatus !== undefined) {
      edgeData.sourceStatus = sourceStatus;
    }
    if (sourceLabel !== undefined) {
      edgeData.sourceLabel = sourceLabel;
    }
    if (targetLabel !== undefined) {
      edgeData.targetLabel = targetLabel;
    }

    const edge: Edge = {
      id: `${graphEdge.source}-${graphEdge.target}`,
      type: "dependency", // Use custom DependencyEdge component
      source: graphEdge.source,
      target: graphEdge.target,
      data: edgeData,
    };

    // Set higher z-index for cross-plan edges to ensure they render on top of groups
    if (isCrossPlan) {
      edge.zIndex = CROSS_PLAN_EDGE_ZINDEX;
    }

    return edge;
  });

  // Build a map of group ID -> session title for edge labels
  const groupTitleMap = new Map<string, string>();
  for (const pg of planGroups) {
    groupTitleMap.set(pg.planArtifactId, pg.sessionTitle ?? "Plan");
  }
  groupTitleMap.set(UNGROUPED_PLAN_ID, "Ungrouped");

  // Add inter-group connector edges (connect group nodes directly for rendering)
  // Group node IDs have "group-" prefix (see createPlanGroupNode)
  const GROUP_CONNECTOR_ZINDEX = 15; // Higher than regular cross-plan edges
  for (const interEdge of interGroupEdges) {
    const sourceLabel = groupTitleMap.get(interEdge.sourceGroupId) ?? interEdge.sourceGroupId;
    const targetLabel = groupTitleMap.get(interEdge.targetGroupId) ?? interEdge.targetGroupId;

    const edgeData: DependencyEdgeData = {
      isCriticalPath: false,
      isCrossPlan: true,
      isGroupConnector: true,
      sourceLabel,
      targetLabel,
    };

    // Use "group-" prefix to match group node IDs from createPlanGroupNode
    const sourceNodeId = `group-${interEdge.sourceGroupId}`;
    const targetNodeId = `group-${interEdge.targetGroupId}`;

    edges.push({
      id: `group-connector-${interEdge.sourceGroupId}-${interEdge.targetGroupId}`,
      type: "dependency",
      source: sourceNodeId,
      target: targetNodeId,
      data: edgeData,
      zIndex: GROUP_CONNECTOR_ZINDEX,
    });
  }

  // Create ALL positioned nodes for group bounding box calculation
  // (includes collapsed group tasks that won't be rendered)
  const allPositionedNodes: Node[] = graphNodes.map((graphNode) => {
    const pos = positions.get(graphNode.taskId) ?? { x: 0, y: 0 };
    return {
      id: graphNode.taskId,
      type: "task",
      position: pos,
      data: {},
    } as Node;
  });

  // Create group nodes for plan groups using ALL positioned nodes
  const groupNodes = createGroupNodes(allPositionedNodes, planGroups, collapsedPlanIds, positions, graphNodes, nodeWidth, nodeHeight, onToggleCollapse);

  return { nodes, edges, groupNodes };
}


// ============================================================================
// Hook
// ============================================================================

/**
 * Hook to compute dagre-based hierarchical layout for task graph
 *
 * Uses layout caching to avoid expensive dagre recomputation when only node data
 * (status, title, priority) changes but graph structure (nodes, edges) remains the same.
 *
 * @param nodes - Task graph nodes from API
 * @param edges - Task graph edges from API
 * @param criticalPath - Array of task IDs on the critical path
 * @param planGroups - Plan group info for visual grouping
 * @param config - Optional layout configuration overrides
 * @param collapsedPlanIds - Set of plan artifact IDs that are collapsed
 * @param onToggleCollapse - Callback when collapse is toggled
 * @returns React Flow nodes, edges, and group nodes with computed positions
 *
 * @example
 * ```tsx
 * const [collapsedPlanIds, setCollapsedPlanIds] = useState<Set<string>>(new Set());
 * const handleToggleCollapse = (planArtifactId: string) => {
 *   setCollapsedPlanIds(prev => {
 *     const next = new Set(prev);
 *     if (next.has(planArtifactId)) next.delete(planArtifactId);
 *     else next.add(planArtifactId);
 *     return next;
 *   });
 * };
 *
 * const { nodes, edges, groupNodes } = useTaskGraphLayout(
 *   graphData.nodes,
 *   graphData.edges,
 *   graphData.criticalPath,
 *   graphData.planGroups,
 *   { direction: "LR" },
 *   collapsedPlanIds,
 *   handleToggleCollapse
 * );
 *
 * // Combine task nodes and group nodes for React Flow
 * const allNodes = [...groupNodes, ...nodes];
 * return <ReactFlow nodes={allNodes} edges={edges} />;
 * ```
 */
export function useTaskGraphLayout(
  graphNodes: TaskGraphNode[],
  graphEdges: TaskGraphEdge[],
  criticalPath: string[],
  planGroups: PlanGroupInfo[] = [],
  config: Partial<LayoutConfig> = {},
  collapsedPlanIds: Set<string> = new Set(),
  onToggleCollapse?: (planArtifactId: string) => void
): LayoutResult {
  // Merge with default config
  const fullConfig = useMemo(
    () => ({ ...DEFAULT_CONFIG, ...config }),
    [config]
  );

  // Layout cache - persists across renders, reused when graph structure unchanged
  const layoutCache = useRef<CachedLayout | null>(null);

  // Compute layout using cache when structure is unchanged
  const layout = useMemo(() => {
    if (graphNodes.length === 0) {
      return { nodes: [], edges: [], groupNodes: [] };
    }
    return computeLayoutWithCache(
      graphNodes,
      graphEdges,
      criticalPath,
      planGroups,
      fullConfig,
      collapsedPlanIds,
      onToggleCollapse,
      layoutCache
    );
  }, [graphNodes, graphEdges, criticalPath, planGroups, fullConfig, collapsedPlanIds, onToggleCollapse]);

  return layout;
}

// ============================================================================
// Layout Cache
// ============================================================================

/**
 * Cached layout positions from dagre computation.
 * Only stores structural layout info - node data is applied fresh each time.
 */
interface CachedLayout {
  hash: string;
  positions: Map<string, { x: number; y: number }>;
}

/**
 * Plan group info for compound graph layout
 */
interface LayoutPlanGroup {
  planArtifactId: string;
  taskIds: string[];
}

/**
 * Compute a structural hash of the graph for cache key.
 * Hash includes: node IDs (sorted), edge pairs (sorted), config direction, plan groups, and collapsed state.
 * Does NOT include node data (status, title, priority) since those don't affect layout.
 */
function computeGraphHash(
  nodeIds: string[],
  edges: { source: string; target: string }[],
  direction: "TB" | "LR",
  planGroups: LayoutPlanGroup[] = [],
  collapsedPlanIds: Set<string> = new Set()
): string {
  // Sort for consistent ordering
  const sortedNodes = [...nodeIds].sort().join(",");
  const sortedEdges = [...edges]
    .map((e) => `${e.source}>${e.target}`)
    .sort()
    .join(",");
  // Include plan group assignments in hash (affects compound layout)
  const sortedGroups = [...planGroups]
    .map((g) => `${g.planArtifactId}:[${[...g.taskIds].sort().join(",")}]`)
    .sort()
    .join(";");
  // Include collapsed state so layout recalculates when groups collapse/expand
  const sortedCollapsed = [...collapsedPlanIds].sort().join(",");
  return `${direction}:${sortedNodes}|${sortedEdges}|${sortedGroups}|${sortedCollapsed}`;
}

/**
 * Compute layout positions using dagre with compound graph support.
 * When planGroups are provided, uses compound graph to keep grouped nodes together
 * and prevent group overlap.
 *
 * Collapsed groups use a single placeholder node for minimal space.
 *
 * Returns position maps for both task nodes and group parent nodes.
 */
function computePositions(
  nodeIds: string[],
  edges: { source: string; target: string }[],
  config: LayoutConfig,
  planGroups: LayoutPlanGroup[] = [],
  collapsedPlanIds: Set<string> = new Set()
): Map<string, { x: number; y: number }> {
  // Use correct node dimensions based on compact mode
  const nodeWidth = config.isCompact ? COMPACT_NODE_WIDTH : NODE_WIDTH;
  const nodeHeight = config.isCompact ? COMPACT_NODE_HEIGHT : NODE_HEIGHT;

  // Use compound graph when we have plan groups to prevent overlap
  const useCompound = planGroups.length > 0;
  const g = new dagre.graphlib.Graph({ compound: useCompound });
  g.setDefaultEdgeLabel(() => ({}));
  g.setGraph({
    rankdir: config.direction,
    nodesep: config.nodesep,
    ranksep: config.ranksep,
    marginx: config.marginx,
    marginy: config.marginy,
  });

  // Build task to group mapping and identify collapsed group tasks
  const taskToGroupMap = new Map<string, string>();
  const collapsedTaskIds = new Set<string>();

  if (useCompound) {
    for (const group of planGroups) {
      const isCollapsed = collapsedPlanIds.has(group.planArtifactId);
      for (const taskId of group.taskIds) {
        taskToGroupMap.set(taskId, group.planArtifactId);
        if (isCollapsed) {
          collapsedTaskIds.add(taskId);
        }
      }
    }

    // Add parent nodes for each plan group
    // These are "invisible" containers that dagre uses for layout
    for (const group of planGroups) {
      // Group parent node needs dimensions - we'll use a placeholder
      // Dagre will expand this based on child nodes
      g.setNode(group.planArtifactId, {
        label: group.planArtifactId,
        // No width/height - dagre computes from children
      });
    }

    // Find ungrouped tasks and create a pseudo-group for them
    const groupedTaskIds = new Set<string>();
    for (const group of planGroups) {
      for (const taskId of group.taskIds) {
        groupedTaskIds.add(taskId);
      }
    }
    const ungroupedTaskIds = nodeIds.filter(id => !groupedTaskIds.has(id));

    // Check if ungrouped pseudo-group is collapsed
    const isUngroupedCollapsed = collapsedPlanIds.has(UNGROUPED_PLAN_ID);
    if (ungroupedTaskIds.length > 0) {
      g.setNode(UNGROUPED_PLAN_ID, { label: UNGROUPED_PLAN_ID });
      for (const taskId of ungroupedTaskIds) {
        taskToGroupMap.set(taskId, UNGROUPED_PLAN_ID);
        if (isUngroupedCollapsed) {
          collapsedTaskIds.add(taskId);
        }
      }
    }
  }

  // Add task nodes - skip collapsed group tasks, add placeholder instead
  const collapsedGroupPlaceholders = new Set<string>();
  for (const id of nodeIds) {
    if (collapsedTaskIds.has(id)) {
      // For collapsed groups, add ONE placeholder per group (not per task)
      const groupId = taskToGroupMap.get(id);
      if (groupId && !collapsedGroupPlaceholders.has(groupId)) {
        // Add a small placeholder node for the collapsed group
        const placeholderId = `__collapsed_placeholder_${groupId}__`;
        g.setNode(placeholderId, { width: COLLAPSED_GROUP_WIDTH, height: 40 }); // Minimal height
        g.setParent(placeholderId, groupId);
        collapsedGroupPlaceholders.add(groupId);
      }
      continue; // Skip individual collapsed tasks
    }

    g.setNode(id, { width: nodeWidth, height: nodeHeight });

    // Set parent relationship for compound graph
    if (useCompound) {
      const parentGroupId = taskToGroupMap.get(id);
      if (parentGroupId) {
        g.setParent(id, parentGroupId);
      }
    }
  }

  // Add edges - for collapsed tasks, redirect to placeholder nodes
  for (const edge of edges) {
    let source = edge.source;
    let target = edge.target;

    // If source is in a collapsed group, use the placeholder
    if (collapsedTaskIds.has(source)) {
      const groupId = taskToGroupMap.get(source);
      if (groupId && collapsedGroupPlaceholders.has(groupId)) {
        source = `__collapsed_placeholder_${groupId}__`;
      } else {
        continue; // Skip if no placeholder
      }
    }

    // If target is in a collapsed group, use the placeholder
    if (collapsedTaskIds.has(target)) {
      const groupId = taskToGroupMap.get(target);
      if (groupId && collapsedGroupPlaceholders.has(groupId)) {
        target = `__collapsed_placeholder_${groupId}__`;
      } else {
        continue; // Skip if no placeholder
      }
    }

    // Avoid self-loops (both source and target collapsed to same placeholder)
    if (source === target) continue;

    g.setEdge(source, target);
  }

  // Run dagre layout
  dagre.layout(g);

  // Extract positions for task nodes
  const positions = new Map<string, { x: number; y: number }>();
  for (const id of nodeIds) {
    const dagreNode = g.node(id);
    if (dagreNode) {
      // Dagre gives center position, React Flow needs top-left
      positions.set(id, {
        x: dagreNode.x - nodeWidth / 2,
        y: dagreNode.y - nodeHeight / 2,
      });
    }
  }

  // Store placeholder positions for collapsed groups (used for group bounding boxes)
  for (const groupId of collapsedGroupPlaceholders) {
    const placeholderId = `__collapsed_placeholder_${groupId}__`;
    const placeholderNode = g.node(placeholderId);
    if (placeholderNode) {
      // Store under special key for the group
      positions.set(`__group_position_${groupId}__`, {
        x: placeholderNode.x - COLLAPSED_GROUP_WIDTH / 2,
        y: placeholderNode.y - 20, // Half of placeholder height (40/2)
      });
    }
  }

  return positions;
}

// Export default config for use in controls
export { DEFAULT_CONFIG };
