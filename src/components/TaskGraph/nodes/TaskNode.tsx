/**
 * TaskNode - Custom React Flow node for task visualization
 *
 * Primary task node (180px width) with:
 * - Status-based border and background colors
 * - Truncated task title
 * - Status badge
 * - Handles for connections (top source, bottom target)
 * - Right-click context menu with status-appropriate actions
 *
 * Per spec: Phase B.2 + E.2 of Task Graph View implementation
 */

import { memo, useCallback } from "react";
import { Handle, Position, type NodeProps, type Node } from "@xyflow/react";
import { getNodeStyle, getStatusCategory, CATEGORY_LABELS, GLASS_SURFACE } from "./nodeStyles";
import { TaskNodeContextMenu } from "./TaskNodeContextMenu";
import type { InternalStatus } from "@/types/status";
import type { Task } from "@/types/task";

// ============================================================================
// Types
// ============================================================================

/**
 * Handler functions passed to TaskNode for context menu actions
 */
export interface TaskNodeHandlers {
  /** Open the task detail overlay */
  onViewDetails: (taskId: string) => void;
  /** Start task execution (ready status) */
  onStartExecution?: (taskId: string) => void;
  /** Block a task with optional reason */
  onBlockWithReason?: (taskId: string, reason?: string) => void;
  /** Unblock a task */
  onUnblock?: (taskId: string) => void;
  /** Approve a task */
  onApprove?: (taskId: string) => void;
  /** Reject a task */
  onReject?: (taskId: string) => void;
  /** Request changes */
  onRequestChanges?: (taskId: string) => void;
  /** Mark merge conflict as resolved */
  onMarkResolved?: (taskId: string) => void;
}

/**
 * Data passed to the TaskNode component
 * Uses Record<string, unknown> intersection to satisfy React Flow constraints
 */
export type TaskNodeData = Record<string, unknown> & {
  label: string;
  taskId: string;
  internalStatus: string;
  priority: number;
  isCriticalPath: boolean;
  /** Whether this node is highlighted (e.g., from timeline click) */
  isHighlighted?: boolean;
  /** Whether this node is keyboard-focused (for keyboard navigation) */
  isFocused?: boolean;
  /** Handler functions for context menu actions */
  handlers?: TaskNodeHandlers;
};

export type TaskNodeType = Node<TaskNodeData, "task">;

// ============================================================================
// Constants
// ============================================================================

/** Node width per design spec */
const NODE_WIDTH = 180;

/** Status label mapping for display */
const STATUS_LABELS: Record<string, string> = {
  backlog: "Backlog",
  ready: "Ready",
  blocked: "Blocked",
  executing: "Executing",
  re_executing: "Re-executing",
  qa_refining: "QA Refining",
  qa_testing: "QA Testing",
  qa_passed: "QA Passed",
  qa_failed: "QA Failed",
  pending_review: "Pending Review",
  reviewing: "Reviewing",
  review_passed: "Review Passed",
  escalated: "Escalated",
  revision_needed: "Revision Needed",
  pending_merge: "Pending Merge",
  merging: "Merging",
  merge_conflict: "Merge Conflict",
  approved: "Approved",
  merged: "Merged",
  failed: "Failed",
  cancelled: "Cancelled",
};

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * Get human-readable label for a status
 */
function getStatusDisplayLabel(status: string): string {
  return STATUS_LABELS[status] ?? status;
}

/**
 * Truncate text to max length with ellipsis
 */
function truncateText(text: string, maxLength: number): string {
  if (text.length <= maxLength) return text;
  return text.slice(0, maxLength - 1) + "…";
}

// ============================================================================
// Component
// ============================================================================

function TaskNodeComponent({ data, selected }: NodeProps<TaskNodeType>) {
  const { label, taskId, internalStatus, priority, isCriticalPath, isHighlighted, isFocused, handlers } = data;
  const style = getNodeStyle(internalStatus);
  const category = getStatusCategory(internalStatus as InternalStatus);
  const categoryLabel = CATEGORY_LABELS[category];
  const statusLabel = getStatusDisplayLabel(internalStatus);

  // Create a minimal task-like object for the context menu
  // The context menu only uses title and internalStatus
  const minimalTask: Task = {
    id: taskId,
    projectId: "", // Not needed for context menu
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

  // Node content that will be wrapped by context menu
  const nodeContent = (
    <div
      className="relative"
      style={{ width: NODE_WIDTH }}
      data-testid="task-node"
      data-status={internalStatus}
      data-critical-path={isCriticalPath}
      data-highlighted={isHighlighted}
      data-focused={isFocused}
    >
      {/* Target handle - top (incoming edges) */}
      <Handle
        type="target"
        position={Position.Top}
        className="!bg-[hsl(220_10%_40%)] !border-[hsl(220_10%_25%)] !w-2 !h-2"
        style={{ top: -4 }}
      />

      {/* Node content - Glass morphism surface */}
      <div
        className={`
          rounded-lg px-3 py-2
          transition-all duration-150 ease-out
          hover:shadow-lg
          ${selected ? "ring-2 ring-white/30" : ""}
          ${isCriticalPath ? "ring-1 ring-[hsl(14_100%_55%_/_0.3)]" : ""}
          ${isHighlighted ? "ring-2 ring-[hsl(var(--accent-primary))] ring-offset-1 ring-offset-[hsl(220_10%_10%)] animate-pulse" : ""}
          ${isFocused && !isHighlighted && !selected ? "ring-2 ring-sky-400/70 ring-offset-1 ring-offset-[hsl(220_10%_10%)]" : ""}
        `}
        style={{
          // Glass morphism surface
          background: GLASS_SURFACE.background,
          backdropFilter: GLASS_SURFACE.backdropFilter,
          WebkitBackdropFilter: GLASS_SURFACE.WebkitBackdropFilter,
          border: GLASS_SURFACE.border,
          // Status-specific border color applied as left stripe (will be added in Task 2)
          // For now, keep status color influence in box-shadow for active states
          boxShadow: isHighlighted
            ? `${GLASS_SURFACE.boxShadow}, 0 0 12px 2px hsl(var(--accent-primary) / 0.4)`
            : isFocused && !selected
            ? `${GLASS_SURFACE.boxShadow}, 0 0 8px 1px rgba(56, 189, 248, 0.3)`
            : style.boxShadow
            ? `${GLASS_SURFACE.boxShadow}, ${style.boxShadow}`
            : GLASS_SURFACE.boxShadow,
          transition: "background 150ms ease, transform 150ms ease, box-shadow 150ms ease",
        }}
      >
        {/* Title */}
        <div
          className="text-sm font-medium text-[hsl(220_10%_90%)] mb-1.5 leading-tight"
          title={label}
        >
          {truncateText(label, 22)}
        </div>

        {/* Status badge */}
        <div
          className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium"
          style={{
            backgroundColor: style.borderColor,
            color: "hsl(220 10% 10%)",
          }}
          title={`${categoryLabel}: ${statusLabel}`}
        >
          {statusLabel}
        </div>
      </div>

      {/* Source handle - bottom (outgoing edges) */}
      <Handle
        type="source"
        position={Position.Bottom}
        className="!bg-[hsl(220_10%_40%)] !border-[hsl(220_10%_25%)] !w-2 !h-2"
        style={{ bottom: -4 }}
      />
    </div>
  );

  // If no handlers provided, render without context menu (for preview/compact modes)
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
      onViewAgentChat={handleViewDetails} // Falls back to view details
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
 * Memoized TaskNode for React Flow performance
 */
export const TaskNode = memo(TaskNodeComponent);
