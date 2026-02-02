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
import { getNodeStyle, GLASS_SURFACE, getPriorityStripeColor } from "./nodeStyles";
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

/**
 * Check if status should show activity dots
 */
function isActiveStatus(status: string): boolean {
  return status === "executing" || status === "re_executing" || status === "reviewing";
}

/**
 * Get activity dot color based on status
 * - Orange for executing states (matches accent)
 * - Blue for reviewing states (matches status-info)
 */
function getActivityDotColor(status: string): string {
  if (status === "reviewing") {
    return "var(--status-info)";
  }
  // executing, re_executing
  return "var(--accent-primary)";
}

function TaskNodeCompactComponent({ data, selected }: NodeProps<TaskNodeCompactType>) {
  const { label, taskId, internalStatus, priority, isCriticalPath, isHighlighted, isFocused, handlers } = data;
  const style = getNodeStyle(internalStatus);
  const showActivityDots = isActiveStatus(internalStatus);
  const activityDotColor = getActivityDotColor(internalStatus);

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
      data-focused={isFocused}
    >
      {/* Target handle - top (incoming edges) */}
      <Handle
        type="target"
        position={Position.Top}
        className="!bg-[hsl(220_10%_40%)] !border-[hsl(220_10%_25%)] !w-1.5 !h-1.5 !opacity-50 hover:!opacity-100 transition-opacity duration-150"
        style={{ top: -3 }}
      />

      {/* Node content - compact version with glass morphism */}
      <div
        className={`
          relative rounded px-2 py-1.5
          transition-all duration-150 ease-out
          hover:shadow-md
          ${isCriticalPath && !selected ? "ring-1 ring-[hsl(14_100%_55%_/_0.3)]" : ""}
          ${isHighlighted ? "ring-2 ring-[hsl(var(--accent-primary))] ring-offset-1 ring-offset-[hsl(220_10%_10%)] animate-pulse" : ""}
          ${isFocused && !isHighlighted && !selected ? "ring-2 ring-sky-400/70 ring-offset-1 ring-offset-[hsl(220_10%_10%)]" : ""}
        `}
        style={{
          // Glass morphism surface - overridden by selection state
          background: selected
            ? "hsla(220 60% 50% / 0.25)"
            : GLASS_SURFACE.background,
          backdropFilter: GLASS_SURFACE.backdropFilter,
          WebkitBackdropFilter: GLASS_SURFACE.WebkitBackdropFilter,
          // Finder-like blue selection border, or default border
          border: selected
            ? "1px solid hsla(220 60% 60% / 0.3)"
            : GLASS_SURFACE.border,
          // Left priority stripe (matches Kanban card styling)
          borderLeft: `3px solid ${getPriorityStripeColor(priority)}`,
          // Status-specific shadow for active states
          boxShadow: isHighlighted
            ? `${GLASS_SURFACE.boxShadow}, 0 0 8px 1px hsl(var(--accent-primary) / 0.4)`
            : isFocused && !selected
            ? `${GLASS_SURFACE.boxShadow}, 0 0 6px 1px rgba(56, 189, 248, 0.3)`
            : style.boxShadow
            ? `${GLASS_SURFACE.boxShadow}, ${style.boxShadow}`
            : GLASS_SURFACE.boxShadow,
          // Pulsing border animation for active states (executing, reviewing)
          animation: style.animation,
          transition: "background 150ms ease, transform 150ms ease, box-shadow 150ms ease",
        }}
      >
        {/* Activity dots - top-right corner for active states */}
        {showActivityDots && (
          <div
            className="absolute top-1 right-1 flex gap-0.5"
            data-testid="activity-dots"
          >
            <span
              className="w-0.5 h-0.5 rounded-full"
              style={{
                backgroundColor: activityDotColor,
                animation: "bounce 1.4s ease-in-out 0s infinite",
              }}
            />
            <span
              className="w-0.5 h-0.5 rounded-full"
              style={{
                backgroundColor: activityDotColor,
                animation: "bounce 1.4s ease-in-out 0.2s infinite",
              }}
            />
            <span
              className="w-0.5 h-0.5 rounded-full"
              style={{
                backgroundColor: activityDotColor,
                animation: "bounce 1.4s ease-in-out 0.4s infinite",
              }}
            />
          </div>
        )}

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
        className="!bg-[hsl(220_10%_40%)] !border-[hsl(220_10%_25%)] !w-1.5 !h-1.5 !opacity-50 hover:!opacity-100 transition-opacity duration-150"
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
