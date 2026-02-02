/**
 * TaskGraphView - React Flow-based task dependency graph visualization
 *
 * Displays tasks as nodes with dependencies as edges.
 * Uses dagre for hierarchical layout computation.
 * Custom nodes will be added in Phase B.
 */

import { useCallback, useEffect } from "react";
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  useNodesState,
  useEdgesState,
  type Node,
  type Edge,
} from "@xyflow/react";

import "@xyflow/react/dist/style.css";

import { useTaskGraph } from "./hooks/useTaskGraph";
import { useTaskGraphLayout } from "./hooks/useTaskGraphLayout";
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
// Status Colors (for MiniMap - main node colors in useTaskGraphLayout)
// ============================================================================

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

  // Compute layout using dagre
  const { nodes: layoutNodes, edges: layoutEdges } = useTaskGraphLayout(
    graphData?.nodes ?? [],
    graphData?.edges ?? [],
    graphData?.criticalPath ?? []
  );

  // React Flow state
  const [nodes, setNodes, onNodesChange] = useNodesState<Node>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([]);

  // Update React Flow state when layout changes
  useEffect(() => {
    setNodes(layoutNodes);
    setEdges(layoutEdges);
  }, [layoutNodes, layoutEdges, setNodes, setEdges]);

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
