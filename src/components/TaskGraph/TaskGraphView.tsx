/**
 * TaskGraphView - React Flow-based task dependency graph visualization
 *
 * Displays tasks as nodes with dependencies as edges.
 * Uses dagre for hierarchical layout computation.
 * Custom TaskNode and DependencyEdge components provide rich visualization.
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
  type NodeTypes,
  type EdgeTypes,
} from "@xyflow/react";

import "@xyflow/react/dist/style.css";

import { useTaskGraph } from "./hooks/useTaskGraph";
import { useTaskGraphLayout } from "./hooks/useTaskGraphLayout";
import { TaskNode } from "./nodes/TaskNode";
import { DependencyEdge } from "./edges/DependencyEdge";
import { PlanGroup, PLAN_GROUP_NODE_TYPE } from "./groups/PlanGroup";
import { getStatusBorderColor } from "./nodes/nodeStyles";
import { useUiStore } from "@/stores/uiStore";
import { TaskDetailOverlay } from "@/components/tasks/TaskDetailOverlay";
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
// Custom Node and Edge Types
// ============================================================================

/**
 * Register custom node types for React Flow
 * IMPORTANT: Defined outside component to prevent unnecessary re-renders
 */
const nodeTypes: NodeTypes = {
  task: TaskNode,
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
// Component
// ============================================================================

export function TaskGraphView({ projectId }: TaskGraphViewProps) {
  const { data: graphData, isLoading, error } = useTaskGraph(projectId);

  // UI Store for task selection
  const selectedTaskId = useUiStore((s) => s.selectedTaskId);
  const setSelectedTaskId = useUiStore((s) => s.setSelectedTaskId);

  // Compute layout using dagre (includes plan grouping)
  const { nodes: layoutNodes, edges: layoutEdges, groupNodes } = useTaskGraphLayout(
    graphData?.nodes ?? [],
    graphData?.edges ?? [],
    graphData?.criticalPath ?? [],
    graphData?.planGroups ?? []
  );

  // React Flow state
  const [nodes, setNodes, onNodesChange] = useNodesState<Node>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([]);

  // Update React Flow state when layout changes
  // Group nodes are rendered first (lower z-index) so they appear behind task nodes
  useEffect(() => {
    // Combine group nodes and task nodes - groups first for proper z-ordering
    const allNodes = [...groupNodes, ...layoutNodes];
    setNodes(allNodes);
    setEdges(layoutEdges);
  }, [layoutNodes, layoutEdges, groupNodes, setNodes, setEdges]);

  // Handle node click - opens TaskDetailOverlay via selectedTaskId
  // Only for task nodes, not group nodes
  const onNodeClick = useCallback(
    (_: React.MouseEvent, node: Node) => {
      // Skip group nodes (their IDs start with "group-")
      if (node.id.startsWith("group-")) {
        return;
      }
      // node.id is the task ID
      setSelectedTaskId(node.id);
    },
    [setSelectedTaskId]
  );

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
    <div className="h-full w-full relative" data-testid="task-graph-view">
      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
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

      {/* Task Detail Overlay - renders when a node is selected */}
      {selectedTaskId && <TaskDetailOverlay projectId={projectId} />}
    </div>
  );
}
