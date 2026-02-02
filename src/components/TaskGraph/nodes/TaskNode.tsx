/**
 * TaskNode - Custom React Flow node for task visualization
 *
 * Primary task node (180px width) with:
 * - Status-based border and background colors
 * - Truncated task title
 * - Status badge
 * - Handles for connections (top source, bottom target)
 *
 * Per spec: Phase B.2 of Task Graph View implementation
 */

import { memo } from "react";
import { Handle, Position, type NodeProps, type Node } from "@xyflow/react";
import { getNodeStyle, getStatusCategory, CATEGORY_LABELS } from "./nodeStyles";
import type { InternalStatus } from "@/types/status";

// ============================================================================
// Types
// ============================================================================

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
  const { label, internalStatus, isCriticalPath } = data;
  const style = getNodeStyle(internalStatus);
  const category = getStatusCategory(internalStatus as InternalStatus);
  const categoryLabel = CATEGORY_LABELS[category];
  const statusLabel = getStatusDisplayLabel(internalStatus);

  return (
    <div
      className="relative"
      style={{ width: NODE_WIDTH }}
      data-testid="task-node"
      data-status={internalStatus}
      data-critical-path={isCriticalPath}
    >
      {/* Target handle - top (incoming edges) */}
      <Handle
        type="target"
        position={Position.Top}
        className="!bg-[hsl(220_10%_40%)] !border-[hsl(220_10%_25%)] !w-2 !h-2"
        style={{ top: -4 }}
      />

      {/* Node content */}
      <div
        className={`
          rounded-lg border-2 px-3 py-2
          transition-all duration-150
          ${selected ? "ring-2 ring-white/30" : ""}
          ${isCriticalPath ? "ring-1 ring-[hsl(14_100%_55%_/_0.3)]" : ""}
        `}
        style={{
          borderColor: style.borderColor,
          backgroundColor: style.backgroundColor,
          boxShadow: style.boxShadow,
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
}

/**
 * Memoized TaskNode for React Flow performance
 */
export const TaskNode = memo(TaskNodeComponent);
