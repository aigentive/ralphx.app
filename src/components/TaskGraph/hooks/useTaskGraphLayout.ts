/**
 * useTaskGraphLayout hook - Dagre-based hierarchical layout for task graph
 *
 * Uses dagre algorithm to compute node positions for a proper hierarchical layout
 * with configurable spacing and direction.
 */

import { useMemo } from "react";
import dagre from "@dagrejs/dagre";
import { Position, type Node, type Edge } from "@xyflow/react";
import type { TaskGraphNode, TaskGraphEdge } from "@/api/task-graph.types";
import { getStatusBorderColor, getStatusBackground } from "../nodes/nodeStyles";

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
}

// Use Record<string, unknown> compatible structure for React Flow
type TaskNodeData = Record<string, unknown> & {
  label: string;
  taskId: string;
  internalStatus: string;
  priority: number;
  isCriticalPath: boolean;
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
// Layout Computation
// ============================================================================

function computeLayout(
  graphNodes: TaskGraphNode[],
  graphEdges: TaskGraphEdge[],
  criticalPath: string[],
  config: LayoutConfig
): LayoutResult {
  // Create dagre graph
  const g = new dagre.graphlib.Graph();
  g.setDefaultEdgeLabel(() => ({}));

  // Set graph options
  g.setGraph({
    rankdir: config.direction,
    nodesep: config.nodesep,
    ranksep: config.ranksep,
    marginx: config.marginx,
    marginy: config.marginy,
  });

  // Create a set of critical path nodes for quick lookup
  const criticalPathSet = new Set(criticalPath);

  // Add nodes to dagre graph
  graphNodes.forEach((node) => {
    g.setNode(node.taskId, {
      width: NODE_WIDTH,
      height: NODE_HEIGHT,
    });
  });

  // Add edges to dagre graph
  graphEdges.forEach((edge) => {
    g.setEdge(edge.source, edge.target);
  });

  // Run dagre layout
  dagre.layout(g);

  // Determine handle positions based on layout direction
  const sourcePosition = config.direction === "TB" ? Position.Bottom : Position.Right;
  const targetPosition = config.direction === "TB" ? Position.Top : Position.Left;

  // Transform dagre nodes to React Flow nodes
  const nodes: Node[] = graphNodes.map((graphNode) => {
    const dagreNode = g.node(graphNode.taskId);

    // Dagre gives center position, React Flow needs top-left
    const x = dagreNode.x - NODE_WIDTH / 2;
    const y = dagreNode.y - NODE_HEIGHT / 2;

    return {
      id: graphNode.taskId,
      position: { x, y },
      data: {
        label:
          graphNode.title.length > 25
            ? graphNode.title.slice(0, 25) + "..."
            : graphNode.title,
        taskId: graphNode.taskId,
        internalStatus: graphNode.internalStatus,
        priority: graphNode.priority,
        isCriticalPath: criticalPathSet.has(graphNode.taskId),
      } satisfies TaskNodeData,
      sourcePosition,
      targetPosition,
      style: {
        background: getStatusBackground(graphNode.internalStatus),
        border: `1px solid ${getStatusBorderColor(graphNode.internalStatus)}`,
        borderRadius: 8,
        padding: "8px 12px",
        fontSize: 12,
        width: NODE_WIDTH,
      },
    } as Node;
  });

  // Transform graph edges to React Flow edges
  const edges: Edge[] = graphEdges.map((graphEdge) => ({
    id: `${graphEdge.source}-${graphEdge.target}`,
    source: graphEdge.source,
    target: graphEdge.target,
    animated: graphEdge.isCriticalPath,
    style: {
      stroke: graphEdge.isCriticalPath ? "hsl(14 100% 55%)" : "hsl(220 10% 40%)",
      strokeWidth: graphEdge.isCriticalPath ? 2 : 1,
      strokeDasharray: graphEdge.isCriticalPath ? undefined : "5 5",
    },
  }));

  return { nodes, edges };
}


// ============================================================================
// Hook
// ============================================================================

/**
 * Hook to compute dagre-based hierarchical layout for task graph
 *
 * @param nodes - Task graph nodes from API
 * @param edges - Task graph edges from API
 * @param criticalPath - Array of task IDs on the critical path
 * @param config - Optional layout configuration overrides
 * @returns React Flow nodes and edges with computed positions
 *
 * @example
 * ```tsx
 * const { nodes, edges } = useTaskGraphLayout(
 *   graphData.nodes,
 *   graphData.edges,
 *   graphData.criticalPath,
 *   { direction: "LR" }
 * );
 *
 * return <ReactFlow nodes={nodes} edges={edges} />;
 * ```
 */
export function useTaskGraphLayout(
  graphNodes: TaskGraphNode[],
  graphEdges: TaskGraphEdge[],
  criticalPath: string[],
  config: Partial<LayoutConfig> = {}
): LayoutResult {
  // Merge with default config
  const fullConfig = useMemo(
    () => ({ ...DEFAULT_CONFIG, ...config }),
    [config]
  );

  // Compute layout whenever inputs change
  const layout = useMemo(() => {
    if (graphNodes.length === 0) {
      return { nodes: [], edges: [] };
    }
    return computeLayout(graphNodes, graphEdges, criticalPath, fullConfig);
  }, [graphNodes, graphEdges, criticalPath, fullConfig]);

  return layout;
}

// Export default config for use in controls
export { DEFAULT_CONFIG };
