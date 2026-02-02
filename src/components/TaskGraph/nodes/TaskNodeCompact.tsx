/**
 * TaskNodeCompact - Compact React Flow node variant for large graphs
 *
 * Smaller node (100px width) for graphs with 50+ tasks:
 * - Status-based border and background colors
 * - Abbreviated task title (first 2 words + ellipsis)
 * - No status badge (status communicated via color)
 * - Handles for connections (top source, bottom target)
 * - Right-click context menu (same as TaskNode)
 *
 * Per spec: Phase F.1 of Task Graph View implementation
 */

import { memo, useCallback } from "react";
import { Handle, Position, type NodeProps, type Node } from "@xyflow/react";
import { getNodeStyle } from "./nodeStyles";
import { TaskNodeContextMenu } from "./TaskNodeContextMenu";
import type { InternalStatus } from "@/types/status";
import type { Task } from "@/types/task";
import type { TaskNodeData } from "./TaskNode";

// ============================================================================
// Types
// ============================================================================

/**
 * Compact node uses the same data type as TaskNode for consistency
 */
export type TaskNodeCompactType = Node<TaskNodeData, "taskCompact">;

// ============================================================================
// Constants
// ============================================================================

/** Compact node width - smaller than standard 180px */
const NODE_WIDTH = 100;

/** Maximum characters for abbreviated title */
const MAX_TITLE_CHARS = 12;

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * Abbreviate text to fit compact node
 * Takes first ~12 characters and adds ellipsis if truncated
 */
function abbreviateTitle(text: string): string {
  if (text.length <= MAX_TITLE_CHARS) return text;
  // Try to break at word boundary
  const truncated = text.slice(0, MAX_TITLE_CHARS);
  const lastSpace = truncated.lastIndexOf(" ");
  if (lastSpace > MAX_TITLE_CHARS / 2) {
    return truncated.slice(0, lastSpace) + "…";
  }
  return truncated.slice(0, MAX_TITLE_CHARS - 1) + "…";
}

// ============================================================================
// Component
// ============================================================================

function TaskNodeCompactComponent({ data, selected }: NodeProps<TaskNodeCompactType>) {
  const { label, taskId, internalStatus, priority, isCriticalPath, isHighlighted, handlers } = data;
  const style = getNodeStyle(internalStatus);

  // Create a minimal task-like object for the context menu
  const minimalTask: Task = {
    id: taskId,
    projectId: "",
    category: "",
    title: label,
    description: null,
    priority,
    internalStatus: internalStatus as InternalStatus,
    needsReviewPoint: false,
    createdAt: "",
    updatedAt: "",
    startedAt: null,
    completedAt: null,
    archivedAt: null,
    blockedReason: null,
  };

  // Wrap handlers to pass taskId
  const handleViewDetails = useCallback(() => {
    handlers?.onViewDetails(taskId);
  }, [handlers, taskId]);

  const handleStartExecution = useCallback(() => {
    handlers?.onStartExecution?.(taskId);
  }, [handlers, taskId]);

  const handleBlockWithReason = useCallback((reason?: string) => {
    handlers?.onBlockWithReason?.(taskId, reason);
  }, [handlers, taskId]);

  const handleUnblock = useCallback(() => {
    handlers?.onUnblock?.(taskId);
  }, [handlers, taskId]);

  const handleApprove = useCallback(() => {
    handlers?.onApprove?.(taskId);
  }, [handlers, taskId]);

  const handleReject = useCallback(() => {
    handlers?.onReject?.(taskId);
  }, [handlers, taskId]);

  const handleRequestChanges = useCallback(() => {
    handlers?.onRequestChanges?.(taskId);
  }, [handlers, taskId]);

  const handleMarkResolved = useCallback(() => {
    handlers?.onMarkResolved?.(taskId);
  }, [handlers, taskId]);

  // Node content
  const nodeContent = (
    <div
      className="relative"
      style={{ width: NODE_WIDTH }}
      data-testid="task-node-compact"
      data-status={internalStatus}
      data-critical-path={isCriticalPath}
      data-highlighted={isHighlighted}
    >
      {/* Target handle - top (incoming edges) */}
      <Handle
        type="target"
        position={Position.Top}
        className="!bg-[hsl(220_10%_40%)] !border-[hsl(220_10%_25%)] !w-1.5 !h-1.5"
        style={{ top: -3 }}
      />

      {/* Node content - compact version */}
      <div
        className={`
          rounded border-2 px-2 py-1.5
          transition-all duration-150
          ${selected ? "ring-2 ring-white/30" : ""}
          ${isCriticalPath ? "ring-1 ring-[hsl(14_100%_55%_/_0.3)]" : ""}
          ${isHighlighted ? "ring-2 ring-[hsl(var(--accent-primary))] ring-offset-1 ring-offset-[hsl(220_10%_10%)] animate-pulse" : ""}
        `}
        style={{
          borderColor: style.borderColor,
          backgroundColor: style.backgroundColor,
          boxShadow: isHighlighted
            ? `${style.boxShadow ?? ""}, 0 0 8px 1px hsl(var(--accent-primary) / 0.4)`
            : style.boxShadow,
        }}
      >
        {/* Abbreviated title - no status badge */}
        <div
          className="text-xs font-medium text-[hsl(220_10%_90%)] leading-tight text-center"
          title={label}
        >
          {abbreviateTitle(label)}
        </div>
      </div>

      {/* Source handle - bottom (outgoing edges) */}
      <Handle
        type="source"
        position={Position.Bottom}
        className="!bg-[hsl(220_10%_40%)] !border-[hsl(220_10%_25%)] !w-1.5 !h-1.5"
        style={{ bottom: -3 }}
      />
    </div>
  );

  // If no handlers provided, render without context menu
  if (!handlers) {
    return nodeContent;
  }

  // Wrap with context menu
  return (
    <TaskNodeContextMenu
      task={minimalTask}
      onViewDetails={handleViewDetails}
      onStartExecution={handleStartExecution}
      onBlockWithReason={handleBlockWithReason}
      onUnblock={handleUnblock}
      onViewAgentChat={handleViewDetails}
      onApprove={handleApprove}
      onReject={handleReject}
      onRequestChanges={handleRequestChanges}
      onMarkResolved={handleMarkResolved}
    >
      {nodeContent}
    </TaskNodeContextMenu>
  );
}

/**
 * Memoized TaskNodeCompact for React Flow performance
 */
export const TaskNodeCompact = memo(TaskNodeCompactComponent);
