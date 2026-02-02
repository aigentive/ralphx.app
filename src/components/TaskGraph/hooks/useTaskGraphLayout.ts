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
};

// Edge data for custom DependencyEdge component
type DependencyEdgeData = Record<string, unknown> & {
  isCriticalPath: boolean;
  sourceStatus?: string;
  /** Whether edge crosses plan group boundaries */
  isCrossPlan?: boolean;
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

// Node dimensions (must match what's rendered in TaskGraphView)
const NODE_WIDTH = 180;
const NODE_HEIGHT = 50;

// ============================================================================
// Group Node Creation
// ============================================================================

/**
 * ID for the "Ungrouped" pseudo-plan that contains standalone tasks
 */
const UNGROUPED_PLAN_ID = "__ungrouped__";

/**
 * Create React Flow group nodes for plan groups
 *
 * @param taskNodes - Positioned task nodes from dagre layout
 * @param planGroups - Plan group info from the API
 * @param collapsedPlanIds - Set of plan artifact IDs that are collapsed
 * @param onToggleCollapse - Callback when collapse is toggled
 * @returns Array of PlanGroupNode for React Flow
 */
function createGroupNodes(
  taskNodes: Node[],
  planGroups: PlanGroupInfo[],
  collapsedPlanIds: Set<string>,
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

  // Calculate bounding boxes for all plan groups
  const boundingBoxes = calculateGroupBoundingBoxes(
    taskNodes,
    planGroupMap,
    NODE_WIDTH,
    NODE_HEIGHT
  );

  // Create group nodes for each plan group
  const groupNodes: PlanGroupNode[] = [];

  // Create a map for quick lookup of plan group info
  const planGroupInfoMap = new Map<string, PlanGroupInfo>();
  for (const pg of planGroups) {
    planGroupInfoMap.set(pg.planArtifactId, pg);
  }

  for (const bbox of boundingBoxes) {
    const planInfo = planGroupInfoMap.get(bbox.planArtifactId);
    if (!planInfo) continue;

    // Expand the bounding box with padding and header space
    const expanded = expandBoundingBox(bbox, GROUP_PADDING, HEADER_HEIGHT);
    const { position, width, height } = boundingBoxToGroupNode(expanded);

    const isCollapsed = collapsedPlanIds.has(planInfo.planArtifactId);

    const groupNode = createPlanGroupNode(
      planInfo.planArtifactId,
      planInfo.sessionId,
      planInfo.sessionTitle,
      planInfo.taskIds,
      planInfo.statusSummary,
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
    const ungroupedMap = new Map<string, string[]>();
    ungroupedMap.set(UNGROUPED_PLAN_ID, ungroupedTaskIds);

    const ungroupedBoxes = calculateGroupBoundingBoxes(
      taskNodes,
      ungroupedMap,
      NODE_WIDTH,
      NODE_HEIGHT
    );

    const ungroupedBbox = ungroupedBoxes[0];
    if (ungroupedBbox) {
      const expanded = expandBoundingBox(ungroupedBbox, GROUP_PADDING, HEADER_HEIGHT);
      const { position, width, height } = boundingBoxToGroupNode(expanded);

      // Create a pseudo-StatusSummary for ungrouped tasks
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

      const isUngroupedCollapsed = collapsedPlanIds.has(UNGROUPED_PLAN_ID);

      const groupNode = createPlanGroupNode(
        UNGROUPED_PLAN_ID,
        "", // No session ID
        "Ungrouped", // Display title
        ungroupedTaskIds,
        ungroupedSummary,
        position,
        width,
        height,
        isUngroupedCollapsed,
        onToggleCollapse
      );

      groupNodes.push(groupNode);
    }
  }

  return groupNodes;
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
  // Build set of collapsed task IDs for lazy loading
  const collapsedTaskIds = buildCollapsedTaskIds(graphNodes, planGroups, collapsedPlanIds);

  // Filter nodes and edges to exclude collapsed groups (lazy loading)
  // This saves dagre computation for tasks that won't be rendered
  const visibleNodes = graphNodes.filter((n) => !collapsedTaskIds.has(n.taskId));
  const visibleEdges = graphEdges.filter(
    (e) => !collapsedTaskIds.has(e.source) && !collapsedTaskIds.has(e.target)
  );

  // Compute structural hash to check cache
  // Hash includes collapsed state so expanding a group invalidates cache
  const nodeIds = visibleNodes.map((n) => n.taskId);
  const edgePairs = visibleEdges.map((e) => ({ source: e.source, target: e.target }));
  const hash = computeGraphHash(nodeIds, edgePairs, config.direction);

  // Check if we can use cached positions
  let positions: Map<string, { x: number; y: number }>;
  if (cache.current && cache.current.hash === hash) {
    // Cache hit: reuse positions
    positions = cache.current.positions;
  } else {
    // Cache miss: compute new layout and cache it
    positions = computePositions(nodeIds, edgePairs, config);
    cache.current = { hash, positions };
  }

  // Create a set of critical path nodes for quick lookup
  const criticalPathSet = new Set(criticalPath);

  // Determine handle positions based on layout direction
  const sourcePosition = config.direction === "TB" ? Position.Bottom : Position.Right;
  const targetPosition = config.direction === "TB" ? Position.Top : Position.Left;

  // Build a map of task ID -> status for edge source status lookup
  const nodeStatusMap = new Map<string, string>();
  graphNodes.forEach((node) => {
    nodeStatusMap.set(node.taskId, node.internalStatus);
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

    const edgeData: DependencyEdgeData = {
      isCriticalPath: graphEdge.isCriticalPath,
      isCrossPlan,
    };
    // Only add sourceStatus if it exists
    if (sourceStatus !== undefined) {
      edgeData.sourceStatus = sourceStatus;
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

  // Create group nodes for plan groups
  const groupNodes = createGroupNodes(nodes, planGroups, collapsedPlanIds, onToggleCollapse);

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
 * Compute a structural hash of the graph for cache key.
 * Hash includes: node IDs (sorted), edge pairs (sorted), and config direction.
 * Does NOT include node data (status, title, priority) since those don't affect layout.
 */
function computeGraphHash(
  nodeIds: string[],
  edges: { source: string; target: string }[],
  direction: "TB" | "LR"
): string {
  // Sort for consistent ordering
  const sortedNodes = [...nodeIds].sort().join(",");
  const sortedEdges = [...edges]
    .map((e) => `${e.source}>${e.target}`)
    .sort()
    .join(",");
  return `${direction}:${sortedNodes}|${sortedEdges}`;
}

/**
 * Compute layout positions using dagre.
 * Returns only the position map - caller applies node data.
 */
function computePositions(
  nodeIds: string[],
  edges: { source: string; target: string }[],
  config: LayoutConfig
): Map<string, { x: number; y: number }> {
  const g = new dagre.graphlib.Graph();
  g.setDefaultEdgeLabel(() => ({}));
  g.setGraph({
    rankdir: config.direction,
    nodesep: config.nodesep,
    ranksep: config.ranksep,
    marginx: config.marginx,
    marginy: config.marginy,
  });

  // Add nodes
  for (const id of nodeIds) {
    g.setNode(id, { width: NODE_WIDTH, height: NODE_HEIGHT });
  }

  // Add edges
  for (const edge of edges) {
    g.setEdge(edge.source, edge.target);
  }

  // Run dagre layout
  dagre.layout(g);

  // Extract positions
  const positions = new Map<string, { x: number; y: number }>();
  for (const id of nodeIds) {
    const dagreNode = g.node(id);
    if (dagreNode) {
      // Dagre gives center position, React Flow needs top-left
      positions.set(id, {
        x: dagreNode.x - NODE_WIDTH / 2,
        y: dagreNode.y - NODE_HEIGHT / 2,
      });
    }
  }

  return positions;
}

// Export default config for use in controls
export { DEFAULT_CONFIG };
