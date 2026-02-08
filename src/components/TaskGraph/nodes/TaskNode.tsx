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
import { GLASS_SURFACE, NODE_WIDTH, NODE_HEIGHT } from "./nodeStyles";
import { TaskNodeContextMenu } from "./TaskNodeContextMenu";
import { useStepProgress } from "@/hooks/useTaskSteps";
import type { InternalStatus } from "@/types/status";
import { TaskStatusBadge } from "@/components/tasks/TaskBoard/TaskStatusBadge";
import { getStatusBorderColor } from "@/types/status-icons";
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
  /** Task description for compact display (2-line clamp) */
  description?: string | null;
  /** Task category for badge display */
  category?: string;
  /** Whether this node is highlighted (e.g., from timeline click) */
  isHighlighted?: boolean;
  /** Whether this node is keyboard-focused (for keyboard navigation) */
  isFocused?: boolean;
  /** Handler functions for context menu actions */
  handlers?: TaskNodeHandlers;
};

export type TaskNodeType = Node<TaskNodeData, "task">;

// ============================================================================
// Helper Functions
// ============================================================================

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

/**
 * Get background color for step dot based on status (inline style)
 */
function getStepDotStyle(
  index: number,
  completed: number,
  skipped: number,
  failed: number,
  inProgress: number
): React.CSSProperties {
  const completedAndSkipped = completed + skipped;
  const failedStart = completedAndSkipped;
  const failedEnd = failedStart + failed;
  const inProgressStart = failedEnd;
  const inProgressEnd = inProgressStart + inProgress;

  const base: React.CSSProperties = {
    width: 6,
    height: 6,
    borderRadius: "50%",
  };

  if (index < completed) return { ...base, backgroundColor: "hsl(142 76% 45%)" }; // success green
  if (index < completedAndSkipped) return { ...base, backgroundColor: "hsl(220 10% 50%)" }; // muted
  if (index < failedEnd) return { ...base, backgroundColor: "hsl(0 84% 60%)" }; // error red
  if (index < inProgressEnd) return { ...base, backgroundColor: "hsl(14 100% 60%)" }; // accent orange
  return { ...base, backgroundColor: "hsl(220 10% 30%)" }; // pending gray
}


function TaskNodeComponent({ data, selected }: NodeProps<TaskNodeType>) {
  const { label, taskId, internalStatus, priority, isCriticalPath, description, category, isHighlighted, isFocused, handlers } = data;
  const statusColor = getStatusBorderColor(internalStatus);
  const { data: stepProgress } = useStepProgress(taskId);

  // Create a minimal task-like object for the context menu
  // The context menu only uses title and internalStatus
  const minimalTask: Task = {
    id: taskId,
    projectId: "", // Not needed for context menu
    category: category ?? "",
    title: label,
    description: description ?? null,
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
        className="!bg-[hsl(220_10%_40%)] !border-[hsl(220_10%_25%)] !w-1.5 !h-1.5 !opacity-50 hover:!opacity-100 transition-opacity duration-150"
        style={{ top: -3 }}
      />

      {/* Node content - Glass morphism surface with fixed height */}
      <div
        className={`
          relative rounded-lg px-3 py-2 overflow-hidden
          transition-all duration-150 ease-out
          hover:shadow-lg
          ${isCriticalPath && !selected && !isHighlighted && !isFocused ? "ring-1 ring-[hsl(14_100%_55%_/_0.3)]" : ""}
        `}
        style={{
          // Fixed height for consistent graph layout (minus handle space)
          height: NODE_HEIGHT - 6,
          // Glass morphism surface - overridden by selection state
          background: selected
            ? "hsla(220 60% 50% / 0.25)"
            : GLASS_SURFACE.background,
          backdropFilter: GLASS_SURFACE.backdropFilter,
          WebkitBackdropFilter: GLASS_SURFACE.WebkitBackdropFilter,
          // Border: solid orange when focused, blue when selected, default otherwise
          border: (isHighlighted || isFocused) && !selected
            ? "2px solid hsl(14 100% 55%)"
            : selected
            ? "1px solid hsla(220 60% 60% / 0.3)"
            : GLASS_SURFACE.border,
          // Left border colored by status (only when not focused)
          borderLeft: (isHighlighted || isFocused) && !selected
            ? "2px solid hsl(14 100% 55%)"
            : `3px solid ${statusColor}`,
          boxShadow: GLASS_SURFACE.boxShadow,
          transition: "background 150ms ease, transform 150ms ease, box-shadow 150ms ease, border 150ms ease",
        }}
      >
        {/* Status badge - top-right corner (shared with Kanban) */}
        <div className="absolute top-1.5 right-1.5" data-testid="status-badge-container">
          <TaskStatusBadge status={internalStatus as InternalStatus} />
        </div>

        {/* Title - Kanban parity (13px, 500 weight) - fixed height */}
        <div
          className="truncate leading-tight pr-8"
          style={{
            fontSize: "13px",
            fontWeight: 500,
            color: "hsl(220 10% 90%)",
            lineHeight: 1.4,
            height: "18px",
          }}
          title={label}
        >
          {truncateText(label, 18)}
        </div>

        {/* Description area - fixed height (1 line) */}
        <div
          className="mt-1 pr-2"
          style={{
            height: "18px", // Space for 1 line
          }}
        >
          {description && (
            <div
              className="truncate"
              style={{
                fontSize: "12px",
                color: "hsl(220 10% 55%)",
                lineHeight: 1.45,
              }}
            >
              {description}
            </div>
          )}
        </div>

        {/* Category + step dots - same line */}
        <div
          className="flex items-center gap-2 mt-2.5"
          style={{ height: "16px" }}
          data-testid="step-progress-footer"
        >
          {category && (
            <span
              style={{
                fontSize: "10px",
                fontWeight: 500,
                color: "hsl(220 10% 45%)",
                textTransform: "capitalize",
              }}
            >
              {category === "plan_merge" ? "Merge to main" : category}
            </span>
          )}
          {/* Show dots when we have step data */}
          {stepProgress && stepProgress.total > 0 && (
            <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
              {Array.from({ length: stepProgress.total }).map((_, index) => (
                <div
                  key={index}
                  style={getStepDotStyle(
                    index,
                    stepProgress.completed,
                    stepProgress.skipped,
                    stepProgress.failed,
                    stepProgress.inProgress
                  )}
                />
              ))}
            </div>
          )}
        </div>

        {/* Progress bar */}
        {stepProgress && stepProgress.total > 0 && (
          <div className="flex items-center gap-2">
            <div
              className="flex-1 h-1 rounded-full overflow-hidden"
              style={{ backgroundColor: "hsl(220 10% 14%)" }}
            >
              <div
                className="h-full rounded-full transition-all duration-300"
                style={{
                  width: `${Math.round(((stepProgress.completed + stepProgress.skipped) / stepProgress.total) * 100)}%`,
                  backgroundColor: "hsl(220 10% 35%)",
                }}
              />
            </div>
            <span
              className="text-[10px] tabular-nums shrink-0"
              style={{ color: "hsl(220 10% 40%)" }}
            >
              {Math.round(((stepProgress.completed + stepProgress.skipped) / stepProgress.total) * 100)}%
            </span>
          </div>
        )}
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
