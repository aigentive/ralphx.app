/**
 * TaskCard - Draggable task card for the kanban board
 *
 * Design spec: specs/design/pages/kanban-board.md
 * - Priority stripe on left border (3px colored)
 * - Layered shadows for depth
 * - Hover lift (translateY -2px)
 * - Drag state (scale, rotate, elevated shadow)
 * - Selected state (orange border + tinted bg)
 * - Drag handle appears on hover (Lucide GripVertical)
 */

import { useDraggable } from "@dnd-kit/core";
import { GripVertical } from "lucide-react";
import type { Task } from "@/types/task";
import { StatusBadge, type ReviewStatus } from "@/components/ui/StatusBadge";
import { TaskQABadge } from "@/components/qa/TaskQABadge";
import { Badge } from "@/components/ui/badge";
import type { QAPrepStatus } from "@/types/qa-config";
import type { QAOverallStatus } from "@/types/qa";

interface TaskCardProps {
  task: Task;
  onSelect?: (taskId: string) => void;
  isDragging?: boolean;
  isSelected?: boolean;
  reviewStatus?: ReviewStatus;
  /** Whether this task needs QA */
  needsQA?: boolean;
  /** QA prep status */
  prepStatus?: QAPrepStatus;
  /** Overall QA test status */
  testStatus?: QAOverallStatus;
  hasCheckpoint?: boolean;
}

/**
 * Get priority color for the left border stripe
 */
function getPriorityColor(priority: number): string {
  switch (priority) {
    case 1: // Critical
      return "var(--status-error)"; // #ef4444
    case 2: // High
      return "var(--status-warning)"; // #f59e0b
    case 3: // Medium
      return "var(--accent-primary)"; // #ff6b35
    case 4: // Low
      return "var(--text-muted)"; // #666666
    default: // None or unknown
      return "transparent";
  }
}

function CheckpointIndicator() {
  return (
    <Badge
      data-testid="checkpoint-indicator"
      className="text-xs"
      style={{
        backgroundColor: "var(--accent-secondary)",
        color: "var(--bg-base)",
        border: "none",
      }}
    >
      Checkpoint
    </Badge>
  );
}

export function TaskCard({
  task,
  onSelect,
  isDragging,
  isSelected,
  reviewStatus,
  needsQA,
  prepStatus,
  testStatus,
  hasCheckpoint,
}: TaskCardProps) {
  const { attributes, listeners, setNodeRef } = useDraggable({ id: task.id });

  // Build QA badge props conditionally to satisfy exactOptionalPropertyTypes
  const qaBadgeProps = {
    needsQA: needsQA ?? false,
    ...(prepStatus !== undefined && { prepStatus }),
    ...(testStatus !== undefined && { testStatus }),
  };

  // Card styles based on state
  const getCardStyles = (): React.CSSProperties => {
    const baseStyles: React.CSSProperties = {
      backgroundColor: "#242424", // --bg-elevated
      borderLeft: `3px solid ${getPriorityColor(task.priority)}`,
      borderRadius: "8px",
      boxShadow: "0 1px 2px rgba(0,0,0,0.2), 0 1px 3px rgba(0,0,0,0.1)",
      cursor: isDragging ? "grabbing" : "grab",
      transition: "transform 150ms ease, box-shadow 150ms ease, border-color 150ms ease",
    };

    if (isDragging) {
      return {
        ...baseStyles,
        transform: "scale(1.02) rotate(2deg)",
        boxShadow: "var(--shadow-md)",
        opacity: 0.9,
        zIndex: 50,
      };
    }

    if (isSelected) {
      return {
        ...baseStyles,
        border: "2px solid var(--accent-primary)",
        borderLeft: `3px solid ${getPriorityColor(task.priority)}`,
        background: "var(--accent-muted)",
        boxShadow: "0 0 0 4px rgba(255, 107, 53, 0.15)",
      };
    }

    return baseStyles;
  };

  return (
    <div
      ref={setNodeRef}
      {...attributes}
      {...listeners}
      data-testid={`task-card-${task.id}`}
      onClick={() => onSelect?.(task.id)}
      className="group relative p-3 hover:translate-y-[-2px] hover:shadow-[var(--shadow-sm)] focus-visible:outline-none focus-visible:shadow-[var(--shadow-glow)]"
      style={getCardStyles()}
      tabIndex={0}
    >
      {/* Drag handle - appears on hover */}
      <div
        data-testid="drag-handle"
        className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity cursor-grab"
      >
        <GripVertical
          className="w-4 h-4 hover:text-[var(--text-secondary)]"
          style={{ color: "var(--text-muted)" }}
        />
      </div>

      {/* Card content */}
      <div className="pr-6">
        {/* Title */}
        <div
          data-testid="task-title"
          className="text-sm font-medium truncate"
          style={{
            color: "var(--text-primary)",
            letterSpacing: "var(--tracking-tight)",
          }}
        >
          {task.title}
        </div>

        {/* Description - 2 line clamp */}
        {task.description && (
          <div
            className="text-xs mt-1 line-clamp-2"
            style={{ color: "var(--text-secondary)" }}
          >
            {task.description}
          </div>
        )}

        {/* Badge row */}
        <div className="flex flex-wrap items-center gap-1.5 mt-2">
          <Badge variant="secondary" className="text-xs">
            {task.category}
          </Badge>
          {reviewStatus && <StatusBadge type="review" status={reviewStatus} />}
          <TaskQABadge {...qaBadgeProps} />
          {hasCheckpoint && <CheckpointIndicator />}
        </div>
      </div>
    </div>
  );
}
