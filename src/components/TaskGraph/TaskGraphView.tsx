/**
 * TaskGraphView - React Flow-based task dependency graph visualization
 *
 * Displays tasks as nodes with dependencies as edges.
 * Uses default nodes initially (custom nodes will be added in Phase B).
 */

import { useMemo, useCallback, useEffect } from "react";
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  useNodesState,
  useEdgesState,
  Position,
  type Node,
  type Edge,
} from "@xyflow/react";

import "@xyflow/react/dist/style.css";

import { useTaskGraph } from "./hooks/useTaskGraph";
import { Loader2 } from "lucide-react";

// ============================================================================
// Types
// ============================================================================

export interface TaskGraphViewProps {
  projectId: string;
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
// Transform graph data to React Flow format
// ============================================================================

type GraphNode = { taskId: string; title: string; internalStatus: string; priority: number; tier: number };

function transformToReactFlowNodes(nodes: GraphNode[]): Node[] {
  // Position nodes vertically by tier, horizontally spread
  const nodesByTier = new Map<number, GraphNode[]>();

  nodes.forEach((node) => {
    const tierNodes = nodesByTier.get(node.tier) ?? [];
    tierNodes.push(node);
    nodesByTier.set(node.tier, tierNodes);
  });

  return nodes.map((node) => {
    const tierNodes = nodesByTier.get(node.tier) ?? [];
    const indexInTier = tierNodes.indexOf(node);
    const tierCount = tierNodes.length;

    // Simple grid layout: x based on position in tier, y based on tier
    const xSpacing = 200;
    const ySpacing = 120;
    const xOffset = (tierCount - 1) * xSpacing / 2;

    return {
      id: node.taskId,
      position: {
        x: indexInTier * xSpacing - xOffset + 400,
        y: node.tier * ySpacing + 50,
      },
      data: {
        label: node.title.length > 25 ? node.title.slice(0, 25) + "..." : node.title,
        taskId: node.taskId,
        internalStatus: node.internalStatus,
        priority: node.priority,
        isCriticalPath: false, // Will be set after checking edges
      } satisfies TaskNodeData,
      sourcePosition: Position.Bottom,
      targetPosition: Position.Top,
      style: {
        background: getStatusBackground(node.internalStatus),
        border: `1px solid ${getStatusBorderColor(node.internalStatus)}`,
        borderRadius: 8,
        padding: "8px 12px",
        fontSize: 12,
        width: 180,
      },
    } as Node;
  });
}

function transformToReactFlowEdges(
  edges: { source: string; target: string; isCriticalPath: boolean }[]
): Edge[] {
  return edges.map((edge) => ({
    id: `${edge.source}-${edge.target}`,
    source: edge.source,
    target: edge.target,
    animated: edge.isCriticalPath,
    style: {
      stroke: edge.isCriticalPath ? "hsl(14 100% 55%)" : "hsl(220 10% 40%)",
      strokeWidth: edge.isCriticalPath ? 2 : 1,
      strokeDasharray: edge.isCriticalPath ? undefined : "5 5",
    },
  }));
}

// ============================================================================
// Status Colors (temporary - will be moved to nodeStyles.ts in Phase B)
// ============================================================================

function getStatusBackground(status: string): string {
  switch (status) {
    // Executing
    case "executing":
    case "re_executing":
      return "hsla(14 100% 55% / 0.15)";
    // Blocked
    case "blocked":
      return "hsla(45 90% 55% / 0.1)";
    // Review
    case "pending_review":
    case "reviewing":
    case "review_passed":
    case "escalated":
    case "revision_needed":
      return "hsla(220 80% 60% / 0.12)";
    // QA
    case "qa_refining":
    case "qa_testing":
    case "qa_passed":
    case "qa_failed":
      return "hsla(280 60% 55% / 0.12)";
    // Merge
    case "pending_merge":
    case "merging":
    case "merge_conflict":
      return "hsla(180 60% 50% / 0.12)";
    // Complete
    case "approved":
    case "merged":
      return "hsla(145 60% 45% / 0.12)";
    // Terminal
    case "failed":
    case "cancelled":
      return "hsla(0 70% 55% / 0.12)";
    // Idle (backlog, ready)
    default:
      return "hsla(220 10% 15% / 0.8)";
  }
}

function getStatusBorderColor(status: string): string {
  switch (status) {
    case "executing":
    case "re_executing":
      return "hsl(14 100% 55%)";
    case "blocked":
      return "hsl(45 90% 55%)";
    case "pending_review":
    case "reviewing":
    case "review_passed":
    case "escalated":
    case "revision_needed":
      return "hsl(220 80% 60%)";
    case "qa_refining":
    case "qa_testing":
    case "qa_passed":
    case "qa_failed":
      return "hsl(280 60% 55%)";
    case "pending_merge":
    case "merging":
    case "merge_conflict":
      return "hsl(180 60% 50%)";
    case "approved":
    case "merged":
      return "hsl(145 60% 45%)";
    case "failed":
    case "cancelled":
      return "hsl(0 70% 55%)";
    default:
      return "hsl(220 10% 40%)";
  }
}

// ============================================================================
// Component
// ============================================================================

export function TaskGraphView({ projectId }: TaskGraphViewProps) {
  const { data: graphData, isLoading, error } = useTaskGraph(projectId);

  // Transform graph data to React Flow format
  const { transformedNodes, transformedEdges } = useMemo(() => {
    if (!graphData) {
      return { transformedNodes: [] as Node[], transformedEdges: [] as Edge[] };
    }

    return {
      transformedNodes: transformToReactFlowNodes(graphData.nodes),
      transformedEdges: transformToReactFlowEdges(graphData.edges),
    };
  }, [graphData]);

  // React Flow state
  const [nodes, setNodes, onNodesChange] = useNodesState<Node>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([]);

  // Update React Flow state when graph data changes
  useEffect(() => {
    setNodes(transformedNodes);
    setEdges(transformedEdges);
  }, [transformedNodes, transformedEdges, setNodes, setEdges]);

  // Handle node click - will open TaskDetailOverlay in Task A.7
  const onNodeClick = useCallback((_: React.MouseEvent, node: Node) => {
    console.log("Node clicked:", node.id);
    // TODO: Task A.7 will wire this to openModal('task-detail', { task })
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

  // Empty state
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

  return (
    <div className="h-full w-full" data-testid="task-graph-view">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onNodeClick={onNodeClick}
        fitView
        fitViewOptions={{ padding: 0.2 }}
        minZoom={0.1}
        maxZoom={2}
        proOptions={{ hideAttribution: true }}
      >
        <Background color="hsl(220 10% 25%)" gap={20} />
        <Controls
          showInteractive={false}
          style={{
            background: "hsla(220 10% 12% / 0.9)",
            border: "1px solid hsla(220 20% 100% / 0.08)",
            borderRadius: 8,
          }}
        />
        <MiniMap
          nodeColor={(node) => {
            const data = node.data as TaskNodeData | undefined;
            return getStatusBorderColor(data?.internalStatus ?? "backlog");
          }}
          maskColor="hsla(220 10% 5% / 0.8)"
          style={{
            background: "hsla(220 10% 12% / 0.9)",
            border: "1px solid hsla(220 20% 100% / 0.08)",
            borderRadius: 8,
          }}
        />
      </ReactFlow>
    </div>
  );
}
