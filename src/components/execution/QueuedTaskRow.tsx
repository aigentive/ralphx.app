/**
 * QueuedTaskRow - Single row in QueuedTasksPopover showing task queue position
 *
 * Displays:
 * - Position number in queue
 * - Task title
 * - Plan association (plan title or "Standalone")
 * - Priority badge
 */

import { PriorityBadge } from "@/components/Ideation/PriorityBadge";
import { priorityFromScore } from "@/lib/priority";
import type { QueuedTask } from "@/hooks/useQueuedTasks";

interface QueuedTaskRowProps {
  /** Queue position (1-indexed) */
  position: number;
  /** Task with plan title */
  task: QueuedTask;
}

export function QueuedTaskRow({ position, task }: QueuedTaskRowProps) {
  const priority = priorityFromScore(task.priority);

  return (
    <div
      data-testid="queued-task-row"
      className="flex items-start gap-3 py-2.5 px-3 rounded-md transition-colors hover:bg-[hsl(220_10%_15%)]"
    >
      {/* Position Number */}
      <span
        className="text-sm font-medium shrink-0 mt-0.5"
        style={{ color: "hsl(220 10% 45%)" }}
      >
        {position}.
      </span>

      {/* Task Info */}
      <div className="flex-1 min-w-0">
        {/* Task Title */}
        <div
          className="text-sm font-medium truncate"
          style={{ color: "hsl(220 10% 90%)" }}
        >
          {task.title}
        </div>

        {/* Plan Association & Priority */}
        <div className="flex items-center gap-2 mt-1">
          <span
            className="text-xs"
            style={{ color: "hsl(220 10% 55%)" }}
          >
            {task.planTitle}
          </span>
          <span style={{ color: "hsl(220 10% 35%)" }}>•</span>
          <PriorityBadge priority={priority} size="compact" />
        </div>
      </div>
    </div>
  );
}
