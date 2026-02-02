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
import {
  Clock,
  Loader2,
  Play,
  CheckCircle,
  AlertTriangle,
  Ban,
  RotateCcw,
  GitMerge,
  AlertCircle,
  XCircle,
} from "lucide-react";
import { getNodeStyle, getStatusCategory, CATEGORY_LABELS, GLASS_SURFACE, getPriorityStripeColor, NODE_WIDTH, NODE_HEIGHT } from "./nodeStyles";
import { TaskNodeContextMenu } from "./TaskNodeContextMenu";
import { useStepProgress } from "@/hooks/useTaskSteps";
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
// Constants
// ============================================================================

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

/**
 * Check if status should show activity dots
 */
function isActiveStatus(status: string): boolean {
  return status === "executing" || status === "re_executing" || status === "reviewing";
}

/**
 * Check if status should show step progress bar (matches Kanban logic)
 */
function shouldShowProgressBar(status: string): boolean {
  return (
    status === "executing" ||
    status === "re_executing" ||
    status.startsWith("qa_") ||
    status === "pending_review" ||
    status === "reviewing" ||
    status === "review_passed" ||
    status === "escalated" ||
    status === "revision_needed" ||
    status === "approved"
  );
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

/**
 * Get background color class for step dot based on status
 */
function getStepDotColor(
  index: number,
  completed: number,
  skipped: number,
  failed: number,
  inProgress: number
): string {
  const completedAndSkipped = completed + skipped;
  const failedStart = completedAndSkipped;
  const failedEnd = failedStart + failed;
  const inProgressStart = failedEnd;
  const inProgressEnd = inProgressStart + inProgress;

  if (index < completed) return "bg-status-success";
  if (index < completedAndSkipped) return "bg-text-muted";
  if (index < failedEnd) return "bg-status-error";
  if (index < inProgressEnd) return "bg-accent-primary animate-pulse";
  return "bg-border-default";
}

/**
 * Status badge configuration - icon, color, and label for each status
 * Matches Kanban card styling: translucent backgrounds, small icons
 */
interface StatusBadgeConfig {
  icon: React.ComponentType<{ className?: string }>;
  color: string;
  bgOpacity: string;
  label: string;
}

const STATUS_BADGE_CONFIG: Record<string, StatusBadgeConfig> = {
  // Idle
  backlog: { icon: Clock, color: "hsl(220 10% 55%)", bgOpacity: "0.15", label: "Backlog" },
  ready: { icon: Play, color: "hsl(220 10% 55%)", bgOpacity: "0.15", label: "Ready" },
  // Blocked
  blocked: { icon: Ban, color: "hsl(45 90% 55%)", bgOpacity: "0.2", label: "Blocked" },
  // Executing
  executing: { icon: Loader2, color: "hsl(14 100% 55%)", bgOpacity: "0.2", label: "Executing" },
  re_executing: { icon: RotateCcw, color: "hsl(14 100% 55%)", bgOpacity: "0.2", label: "Revising" },
  // QA
  qa_refining: { icon: Loader2, color: "hsl(280 60% 55%)", bgOpacity: "0.2", label: "QA" },
  qa_testing: { icon: Loader2, color: "hsl(280 60% 55%)", bgOpacity: "0.2", label: "Testing" },
  qa_passed: { icon: CheckCircle, color: "hsl(280 60% 55%)", bgOpacity: "0.2", label: "QA ✓" },
  qa_failed: { icon: XCircle, color: "hsl(280 60% 55%)", bgOpacity: "0.2", label: "QA ✗" },
  // Review
  pending_review: { icon: Clock, color: "hsl(220 80% 60%)", bgOpacity: "0.2", label: "Pending" },
  reviewing: { icon: Loader2, color: "hsl(220 80% 60%)", bgOpacity: "0.2", label: "Reviewing" },
  review_passed: { icon: CheckCircle, color: "hsl(145 60% 45%)", bgOpacity: "0.2", label: "Approved" },
  escalated: { icon: AlertTriangle, color: "hsl(45 90% 55%)", bgOpacity: "0.2", label: "Escalated" },
  revision_needed: { icon: RotateCcw, color: "hsl(45 90% 55%)", bgOpacity: "0.2", label: "Revision" },
  // Merge
  pending_merge: { icon: GitMerge, color: "hsl(180 60% 50%)", bgOpacity: "0.2", label: "Merge" },
  merging: { icon: Loader2, color: "hsl(180 60% 50%)", bgOpacity: "0.2", label: "Merging" },
  merge_conflict: { icon: AlertCircle, color: "hsl(45 90% 55%)", bgOpacity: "0.2", label: "Conflict" },
  // Complete
  approved: { icon: CheckCircle, color: "hsl(145 60% 45%)", bgOpacity: "0.2", label: "Done" },
  merged: { icon: GitMerge, color: "hsl(145 60% 45%)", bgOpacity: "0.2", label: "Merged" },
  // Terminal
  failed: { icon: XCircle, color: "hsl(0 70% 55%)", bgOpacity: "0.2", label: "Failed" },
  cancelled: { icon: Ban, color: "hsl(0 70% 55%)", bgOpacity: "0.2", label: "Cancelled" },
};

/**
 * Get status badge config with fallback
 */
function getStatusBadgeConfig(status: string): StatusBadgeConfig {
  return STATUS_BADGE_CONFIG[status] ?? {
    icon: Clock,
    color: "hsl(220 10% 55%)",
    bgOpacity: "0.15",
    label: status,
  };
}

function TaskNodeComponent({ data, selected }: NodeProps<TaskNodeType>) {
  const { label, taskId, internalStatus, priority, isCriticalPath, description, category, isHighlighted, isFocused, handlers } = data;
  const style = getNodeStyle(internalStatus);
  const statusCategory = getStatusCategory(internalStatus as InternalStatus);
  const categoryLabel = CATEGORY_LABELS[statusCategory];
  const statusLabel = getStatusDisplayLabel(internalStatus);
  const showActivityDots = isActiveStatus(internalStatus);
  const activityDotColor = getActivityDotColor(internalStatus);
  const showProgressBar = shouldShowProgressBar(internalStatus);
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
          ${isCriticalPath && !selected ? "ring-1 ring-[hsl(14_100%_55%_/_0.3)]" : ""}
          ${isHighlighted ? "ring-2 ring-[hsl(var(--accent-primary))] ring-offset-1 ring-offset-[hsl(220_10%_10%)] animate-pulse" : ""}
          ${isFocused && !isHighlighted && !selected ? "ring-2 ring-sky-400/70 ring-offset-1 ring-offset-[hsl(220_10%_10%)]" : ""}
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
          // Finder-like blue selection border, or default border
          border: selected
            ? "1px solid hsla(220 60% 60% / 0.3)"
            : GLASS_SURFACE.border,
          // Left priority stripe (matches Kanban card styling)
          borderLeft: `3px solid ${getPriorityStripeColor(priority)}`,
          // Status-specific glow for active states
          boxShadow: isHighlighted
            ? `${GLASS_SURFACE.boxShadow}, 0 0 12px 2px hsl(var(--accent-primary) / 0.4)`
            : isFocused && !selected
            ? `${GLASS_SURFACE.boxShadow}, 0 0 8px 1px rgba(56, 189, 248, 0.3)`
            : style.boxShadow
            ? `${GLASS_SURFACE.boxShadow}, ${style.boxShadow}`
            : GLASS_SURFACE.boxShadow,
          // Pulsing border animation for active states (executing, reviewing)
          animation: style.animation,
          transition: "background 150ms ease, transform 150ms ease, box-shadow 150ms ease",
        }}
      >
        {/* Status badge + activity dots - top-right corner (Kanban parity) */}
        {(() => {
          const badgeConfig = getStatusBadgeConfig(internalStatus);
          const IconComponent = badgeConfig.icon;
          const isSpinning = internalStatus === "executing" || internalStatus === "reviewing" ||
            internalStatus === "merging" || internalStatus === "qa_refining" || internalStatus === "qa_testing";

          return (
            <div
              className="absolute top-1.5 right-1.5 flex items-center gap-1"
              data-testid="status-badge-container"
            >
              {/* Activity dots for active states */}
              {showActivityDots && (
                <div className="flex gap-0.5" data-testid="activity-dots">
                  <span
                    className="w-1 h-1 rounded-full"
                    style={{
                      backgroundColor: activityDotColor,
                      animation: "bounce 1.4s ease-in-out 0s infinite",
                    }}
                  />
                  <span
                    className="w-1 h-1 rounded-full"
                    style={{
                      backgroundColor: activityDotColor,
                      animation: "bounce 1.4s ease-in-out 0.2s infinite",
                    }}
                  />
                  <span
                    className="w-1 h-1 rounded-full"
                    style={{
                      backgroundColor: activityDotColor,
                      animation: "bounce 1.4s ease-in-out 0.4s infinite",
                    }}
                  />
                </div>
              )}
              {/* Status badge with icon - translucent background */}
              <div
                className="inline-flex items-center gap-0.5 px-1.5 py-px rounded text-[9px] font-medium"
                style={{
                  backgroundColor: `color-mix(in srgb, ${badgeConfig.color} ${parseFloat(badgeConfig.bgOpacity) * 100}%, transparent)`,
                  color: badgeConfig.color,
                }}
                title={`${categoryLabel}: ${statusLabel}`}
                data-testid="status-badge"
              >
                <IconComponent className={`w-2.5 h-2.5 ${isSpinning ? "animate-spin" : ""}`} />
              </div>
            </div>
          );
        })()}

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
              {category}
            </span>
          )}
          {showProgressBar && stepProgress && stepProgress.total > 0 && (
            <div className="flex items-center gap-0.5">
              {Array.from({ length: stepProgress.total }).map((_, index) => (
                <div
                  key={index}
                  className={`h-1.5 w-1.5 rounded-full ${getStepDotColor(
                    index,
                    stepProgress.completed,
                    stepProgress.skipped,
                    stepProgress.failed,
                    stepProgress.inProgress
                  )}`}
                />
              ))}
            </div>
          )}
        </div>

        {/* Progress bar */}
        {showProgressBar && stepProgress && stepProgress.total > 0 && (
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
