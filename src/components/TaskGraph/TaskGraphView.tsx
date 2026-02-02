/**
 * TaskGraphView - React Flow-based task dependency graph visualization
 *
 * Displays tasks as nodes with dependencies as edges.
 * Uses dagre for hierarchical layout computation.
 * Custom TaskNode and DependencyEdge components provide rich visualization.
 * Includes ExecutionTimeline side panel for timeline-to-node interaction.
 */

import { useCallback, useEffect, useMemo, useState, useRef } from "react";
import {
  ReactFlow,
  ReactFlowProvider,
  Background,
  Controls,
  MiniMap,
  useNodesState,
  useEdgesState,
  useReactFlow,
  type Node,
  type Edge,
  type NodeTypes,
  type EdgeTypes,
} from "@xyflow/react";

import "@xyflow/react/dist/style.css";

import { useTaskGraph } from "./hooks/useTaskGraph";
import { useTaskGraphLayout } from "./hooks/useTaskGraphLayout";
import { TaskNode, type TaskNodeHandlers } from "./nodes/TaskNode";
import { DependencyEdge } from "./edges/DependencyEdge";
import { PlanGroup, PLAN_GROUP_NODE_TYPE } from "./groups/PlanGroup";
import { getStatusBorderColor } from "./nodes/nodeStyles";
import { ExecutionTimeline } from "./timeline/ExecutionTimeline";
import { useUiStore } from "@/stores/uiStore";
import { TaskDetailOverlay } from "@/components/tasks/TaskDetailOverlay";
import { useTaskMutation } from "@/hooks/useTaskMutation";
import { api } from "@/lib/tauri";
import { toast } from "sonner";
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
  isHighlighted?: boolean;
  handlers?: TaskNodeHandlers;
};

// ============================================================================
// Constants
// ============================================================================

/** Duration in ms before clearing the highlighted node */
const HIGHLIGHT_TIMEOUT_MS = 3000;

/** Empty layout config - defined as constant to prevent re-renders */
const EMPTY_LAYOUT_CONFIG = {};

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
// Inner Component (has access to useReactFlow)
// ============================================================================

interface TaskGraphViewInnerProps {
  projectId: string;
}

function TaskGraphViewInner({ projectId }: TaskGraphViewInnerProps) {
  const { data: graphData, isLoading, error } = useTaskGraph(projectId);
  const { setCenter, getNodes } = useReactFlow();

  // UI Store for task selection
  const selectedTaskId = useUiStore((s) => s.selectedTaskId);
  const setSelectedTaskId = useUiStore((s) => s.setSelectedTaskId);

  // Highlighted task state (for timeline-to-node interaction)
  const [highlightedTaskId, setHighlightedTaskId] = useState<string | null>(null);
  const highlightTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Collapse state for plan groups
  const [collapsedPlanIds, setCollapsedPlanIds] = useState<Set<string>>(
    new Set()
  );

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
  }, []);

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

  // Compute layout using dagre (includes plan grouping)
  const { nodes: layoutNodes, edges: layoutEdges, groupNodes } = useTaskGraphLayout(
    graphData?.nodes ?? [],
    graphData?.edges ?? [],
    graphData?.criticalPath ?? [],
    graphData?.planGroups ?? [],
    EMPTY_LAYOUT_CONFIG,
    collapsedPlanIds,
    handleToggleCollapse
  );

  // React Flow state
  const [nodes, setNodes, onNodesChange] = useNodesState<Node>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([]);

  // Build set of task IDs that belong to collapsed groups
  const collapsedTaskIds = useMemo(() => {
    const hiddenIds = new Set<string>();
    for (const pg of graphData?.planGroups ?? []) {
      if (collapsedPlanIds.has(pg.planArtifactId)) {
        for (const taskId of pg.taskIds) {
          hiddenIds.add(taskId);
        }
      }
    }
    // Also check for ungrouped tasks if the ungrouped group is collapsed
    if (collapsedPlanIds.has("__ungrouped__")) {
      // Find tasks not in any plan group
      const groupedTaskIds = new Set<string>();
      for (const pg of graphData?.planGroups ?? []) {
        for (const taskId of pg.taskIds) {
          groupedTaskIds.add(taskId);
        }
      }
      for (const node of graphData?.nodes ?? []) {
        if (!groupedTaskIds.has(node.taskId)) {
          hiddenIds.add(node.taskId);
        }
      }
    }
    return hiddenIds;
  }, [graphData?.planGroups, graphData?.nodes, collapsedPlanIds]);

  // Update React Flow state when layout changes or highlight changes
  // Group nodes are rendered first (lower z-index) so they appear behind task nodes
  // Filter out task nodes that belong to collapsed groups
  useEffect(() => {
    // Filter task nodes - hide those in collapsed groups
    // Also inject handlers for context menu actions
    const visibleTaskNodes = layoutNodes
      .filter((node) => !collapsedTaskIds.has(node.id))
      .map((node) => ({
        ...node,
        data: {
          ...node.data,
          isHighlighted: node.id === highlightedTaskId,
          handlers: nodeHandlers,
        },
      }));
    // Filter edges - hide those connected to hidden nodes
    const visibleEdges = layoutEdges.filter(
      (edge) => !collapsedTaskIds.has(edge.source) && !collapsedTaskIds.has(edge.target)
    );
    // Combine group nodes and visible task nodes - groups first for proper z-ordering
    const allNodes = [...groupNodes, ...visibleTaskNodes];
    setNodes(allNodes);
    setEdges(visibleEdges);
  }, [layoutNodes, layoutEdges, groupNodes, collapsedTaskIds, highlightedTaskId, nodeHandlers, setNodes, setEdges]);

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
      // Clear highlight
      setHighlightedTaskId(null);

      // Skip group nodes (their IDs start with "group-")
      if (node.id.startsWith("group-")) {
        return;
      }
      // node.id is the task ID - open detail overlay
      setSelectedTaskId(node.id);
    },
    [setSelectedTaskId]
  );

  // Clear highlight on pane click
  const handlePaneClick = useCallback(() => {
    if (highlightTimeoutRef.current) {
      clearTimeout(highlightTimeoutRef.current);
      highlightTimeoutRef.current = null;
    }
    setHighlightedTaskId(null);
  }, []);

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
    <div className="h-full w-full relative flex" data-testid="task-graph-view">
      {/* Main graph area */}
      <div className="flex-1 h-full relative">
        <ReactFlow
          nodes={nodes}
          edges={edges}
          nodeTypes={nodeTypes}
          edgeTypes={edgeTypes}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          onNodeClick={handleNodeClick}
          onPaneClick={handlePaneClick}
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

      {/* Execution Timeline side panel */}
      <ExecutionTimeline
        projectId={projectId}
        onTaskClick={handleTimelineTaskClick}
        highlightedTaskId={highlightedTaskId}
        defaultCollapsed={false}
      />

      {/* Task Detail Overlay - renders when a node is selected */}
      {selectedTaskId && <TaskDetailOverlay projectId={projectId} />}
    </div>
  );
}

// ============================================================================
// Main Component (provides ReactFlowProvider)
// ============================================================================

export function TaskGraphView({ projectId }: TaskGraphViewProps) {
  return (
    <ReactFlowProvider>
      <TaskGraphViewInner projectId={projectId} />
    </ReactFlowProvider>
  );
}
