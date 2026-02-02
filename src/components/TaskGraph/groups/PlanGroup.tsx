/**
 * PlanGroup.tsx - Visual region component for plan groups in the Task Graph
 *
 * Renders as a React Flow group node that visually contains all task nodes
 * belonging to the same plan. Uses subtle background and rounded border
 * with PlanGroupHeader at the top.
 *
 * This component is used as a custom node type in React Flow.
 */

import { memo, useCallback } from "react";
import type { NodeProps, Node } from "@xyflow/react";
import { Handle, Position } from "@xyflow/react";
import { PlanGroupHeader } from "./PlanGroupHeader";
import type { StatusSummary } from "@/api/task-graph.types";
import { cn } from "@/lib/utils";
import { GROUP_PADDING, HEADER_HEIGHT } from "./groupUtils";

// ============================================================================
// Types
// ============================================================================

/**
 * Data shape for PlanGroup node
 */
export interface PlanGroupData extends Record<string, unknown> {
  /** Plan artifact ID for this group */
  planArtifactId: string;
  /** Session ID for navigation */
  sessionId: string;
  /** Session/plan title to display */
  sessionTitle: string | null;
  /** Task IDs contained in this group */
  taskIds: string[];
  /** Status summary with counts by category */
  statusSummary: StatusSummary;
  /** Whether the group is collapsed */
  isCollapsed: boolean;
  /** Width of the group region */
  width: number;
  /** Height of the group region */
  height: number;
  /** Callback to toggle collapse state */
  onToggleCollapse?: ((planArtifactId: string) => void) | undefined;
}

export type PlanGroupNode = Node<PlanGroupData, "planGroup">;

export interface PlanGroupProps extends NodeProps<PlanGroupNode> {
  /** Callback when collapse state should change */
  onToggleCollapse?: (planArtifactId: string) => void;
  /** Callback when context menu should open */
  onContextMenu?: (planArtifactId: string) => void;
  /** Callback to navigate to planning session */
  onNavigateToSession?: (sessionId: string) => void;
}

// ============================================================================
// Component
// ============================================================================

/**
 * PlanGroup - Visual region for tasks from the same plan
 *
 * Used as a React Flow custom node with type "planGroup".
 * Contains:
 * - Subtle background with rounded border
 * - PlanGroupHeader with title, progress, and status badges
 * - Empty content area (task nodes are positioned inside)
 *
 * @example
 * ```tsx
 * const nodeTypes = {
 *   planGroup: PlanGroup,
 *   task: TaskNode,
 * };
 *
 * <ReactFlow nodeTypes={nodeTypes} nodes={nodes} edges={edges} />
 * ```
 */
export const PlanGroup = memo(function PlanGroup({
  data,
  selected,
}: PlanGroupProps) {
  const {
    planArtifactId,
    sessionId,
    sessionTitle,
    taskIds,
    statusSummary,
    isCollapsed,
    width,
    height,
    onToggleCollapse,
  } = data;

  // When collapsed, show only the header (minimal padding)
  const displayHeight = isCollapsed ? HEADER_HEIGHT + 8 : height;

  // Handle double-click to toggle collapse (instead of React Flow zoom)
  const handleDoubleClick = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation(); // Prevent React Flow zoom
      onToggleCollapse?.(planArtifactId);
    },
    [onToggleCollapse, planArtifactId]
  );

  return (
    <div
      className={cn(
        // Base styles
        "rounded-lg overflow-hidden",
        // Background - Kanban glass at 50% opacity (no border)
        "bg-[hsla(220_10%_14%_/_0.5)]",
        // Selection state - use ring instead of border
        selected && "ring-1 ring-[hsl(var(--accent-primary)/0.5)]",
        // Transition
        "transition-all duration-200"
      )}
      style={{
        width,
        height: displayHeight,
      }}
      onDoubleClick={handleDoubleClick}
      data-testid={`plan-group-${planArtifactId}`}
    >
      {/* Header */}
      <PlanGroupHeader
        planArtifactId={planArtifactId}
        sessionId={sessionId}
        sessionTitle={sessionTitle}
        taskCount={taskIds.length}
        statusSummary={statusSummary}
        isCollapsed={isCollapsed}
        onToggleCollapse={() => onToggleCollapse?.(planArtifactId)}
      />

      {/* Content area - empty, task nodes are positioned inside by React Flow */}
      {!isCollapsed && (
        <div
          className="relative"
          style={{
            height: displayHeight - HEADER_HEIGHT,
          }}
        >
          {/* Task nodes render here via React Flow's coordinate system */}
        </div>
      )}

      {/* Invisible handles for inter-group edges */}
      <Handle
        type="target"
        position={Position.Top}
        className="!bg-transparent !border-0 !w-4 !h-1"
        style={{ top: 0, left: "50%", visibility: "hidden" }}
      />
      <Handle
        type="source"
        position={Position.Bottom}
        className="!bg-transparent !border-0 !w-4 !h-1"
        style={{ bottom: 0, left: "50%", visibility: "hidden" }}
      />
    </div>
  );
});

// ============================================================================
// Factory Functions
// ============================================================================

/**
 * Create a PlanGroup node for React Flow
 *
 * @param planArtifactId - Unique ID for this plan group
 * @param sessionId - Session ID for navigation
 * @param sessionTitle - Title to display
 * @param taskIds - Task IDs in this group
 * @param statusSummary - Status counts
 * @param position - Top-left position of the group
 * @param width - Width of the group region
 * @param height - Height of the group region
 * @param isCollapsed - Whether the group starts collapsed
 * @param onToggleCollapse - Optional callback when collapse is toggled
 * @returns React Flow node object
 */
export function createPlanGroupNode(
  planArtifactId: string,
  sessionId: string,
  sessionTitle: string | null,
  taskIds: string[],
  statusSummary: StatusSummary,
  position: { x: number; y: number },
  width: number,
  height: number,
  isCollapsed = false,
  onToggleCollapse?: (planArtifactId: string) => void
): PlanGroupNode {
  return {
    id: `group-${planArtifactId}`,
    type: "planGroup",
    position,
    data: {
      planArtifactId,
      sessionId,
      sessionTitle,
      taskIds,
      statusSummary,
      isCollapsed,
      width,
      height,
      onToggleCollapse,
    },
    // Group node properties
    style: {
      width,
      height: isCollapsed ? HEADER_HEIGHT + 8 : height,
    },
    // Make the group node non-draggable by default
    // (tasks inside can still be selected)
    draggable: false,
    selectable: true,
    // Ensure group renders behind task nodes
    zIndex: -1,
  };
}

/**
 * Node type key for registering PlanGroup with React Flow
 */
export const PLAN_GROUP_NODE_TYPE = "planGroup";
