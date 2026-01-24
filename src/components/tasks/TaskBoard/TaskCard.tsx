/**
 * TaskCard - Draggable task card for the kanban board
 */

import { useDraggable } from "@dnd-kit/core";
import type { Task } from "@/types/task";
import { StatusBadge, type ReviewStatus } from "@/components/ui/StatusBadge";
import { TaskQABadge } from "@/components/qa/TaskQABadge";
import type { QAPrepStatus } from "@/types/qa-config";
import type { QAOverallStatus } from "@/types/qa";

interface TaskCardProps {
  task: Task;
  onSelect?: (taskId: string) => void;
  isDragging?: boolean;
  reviewStatus?: ReviewStatus;
  /** Whether this task needs QA */
  needsQA?: boolean;
  /** QA prep status */
  prepStatus?: QAPrepStatus;
  /** Overall QA test status */
  testStatus?: QAOverallStatus;
  hasCheckpoint?: boolean;
}

function DragHandle() {
  return (
    <div data-testid="drag-handle" className="cursor-grab opacity-0 group-hover:opacity-100 transition-opacity" style={{ color: "var(--text-muted)" }}>
      <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor">
        <circle cx="5" cy="4" r="1.5" /><circle cx="11" cy="4" r="1.5" />
        <circle cx="5" cy="8" r="1.5" /><circle cx="11" cy="8" r="1.5" />
        <circle cx="5" cy="12" r="1.5" /><circle cx="11" cy="12" r="1.5" />
      </svg>
    </div>
  );
}

function CheckpointIndicator() {
  return (
    <div data-testid="checkpoint-indicator" className="px-1.5 py-0.5 rounded text-xs font-medium" style={{ backgroundColor: "var(--accent-secondary)", color: "var(--bg-base)" }}>
      Checkpoint
    </div>
  );
}

export function TaskCard({
  task,
  onSelect,
  isDragging,
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

  return (
    <div
      ref={setNodeRef}
      {...attributes}
      {...listeners}
      data-testid={`task-card-${task.id}`}
      onClick={() => onSelect?.(task.id)}
      className={`group p-3 rounded-md cursor-pointer transition-all ${isDragging ? "opacity-50" : ""}`}
      style={{ backgroundColor: "var(--bg-elevated)", borderColor: "var(--border-subtle)" }}
    >
      <div className="flex items-start gap-2">
        <DragHandle />
        <div className="flex-1 min-w-0">
          <div data-testid="task-title" className="truncate font-medium" style={{ color: "var(--text-primary)" }}>
            {task.title}
          </div>
          <div className="flex flex-wrap items-center gap-1.5 mt-2">
            <span className="px-1.5 py-0.5 rounded text-xs" style={{ backgroundColor: "var(--bg-hover)", color: "var(--text-secondary)" }}>
              {task.category}
            </span>
            <span data-testid="priority-indicator" className="text-xs" style={{ color: "var(--text-muted)" }}>
              P{task.priority}
            </span>
            {reviewStatus && <StatusBadge type="review" status={reviewStatus} />}
            <TaskQABadge {...qaBadgeProps} />
            {hasCheckpoint && <CheckpointIndicator />}
          </div>
        </div>
      </div>
    </div>
  );
}
