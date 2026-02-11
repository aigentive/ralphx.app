/**
 * QueuedTasksPopover - Shows tasks waiting in the ready queue
 *
 * Displays ordered list of tasks in "ready" status waiting for execution slots.
 * Tasks are ordered by scheduling priority (priority score desc, then created_at asc).
 */

import { Info } from "lucide-react";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { QueuedTaskRow } from "./QueuedTaskRow";
import { useQueuedTasks } from "@/hooks/useQueuedTasks";

interface QueuedTasksPopoverProps {
  /** The project ID */
  projectId: string;
  /** Number of queued tasks (for display) */
  queuedCount: number;
  /** Children to use as trigger (e.g., "Queued: 5" text) */
  children: React.ReactNode;
}

export function QueuedTasksPopover({
  projectId,
  queuedCount,
  children,
}: QueuedTasksPopoverProps) {
  const { data: queuedTasks, isLoading } = useQueuedTasks(projectId);

  return (
    <Popover>
      <PopoverTrigger asChild>{children}</PopoverTrigger>
      <PopoverContent
        side="top"
        align="start"
        className="w-[400px] p-0"
        style={{
          background: "hsla(220 10% 10% / 0.95)",
          backdropFilter: "blur(20px) saturate(180%)",
          WebkitBackdropFilter: "blur(20px) saturate(180%)",
          border: "1px solid hsla(220 20% 100% / 0.08)",
          borderRadius: "10px",
          boxShadow: `
            0 4px 16px hsla(220 20% 0% / 0.4),
            0 12px 32px hsla(220 20% 0% / 0.3)
          `,
        }}
      >
        {/* Header */}
        <div
          className="px-4 py-3 border-b"
          style={{
            borderColor: "hsl(220 10% 18%)",
          }}
        >
          <h3
            className="text-sm font-semibold"
            style={{ color: "hsl(220 10% 90%)" }}
          >
            Queued Tasks ({queuedCount})
          </h3>
        </div>

        {/* Task List */}
        <div
          className="max-h-[400px] overflow-y-auto px-2 py-2"
          style={{
            /* Custom scrollbar for macOS Tahoe theme */
            scrollbarWidth: "thin",
            scrollbarColor: "hsl(220 10% 25%) transparent",
          }}
        >
          {isLoading ? (
            <div
              className="text-sm text-center py-6"
              style={{ color: "hsl(220 10% 55%)" }}
            >
              Loading...
            </div>
          ) : !queuedTasks || queuedTasks.length === 0 ? (
            <div
              className="text-sm text-center py-6"
              style={{ color: "hsl(220 10% 55%)" }}
            >
              No tasks queued
            </div>
          ) : (
            queuedTasks.map((task, index) => (
              <QueuedTaskRow
                key={task.id}
                position={index + 1}
                task={task}
              />
            ))
          )}
        </div>

        {/* Info Footer */}
        <div
          className="px-4 py-3 border-t"
          style={{
            borderColor: "hsl(220 10% 18%)",
            backgroundColor: "hsl(220 10% 8%)",
          }}
        >
          <div className="flex items-start gap-2">
            <Info
              className="w-4 h-4 shrink-0 mt-0.5"
              style={{ color: "hsl(220 10% 45%)" }}
            />
            <p
              className="text-xs leading-relaxed"
              style={{ color: "hsl(220 10% 55%)" }}
            >
              Queued tasks are in "ready" status waiting for an execution slot.
              Tasks run in priority order, oldest first within same priority.
            </p>
          </div>
        </div>
      </PopoverContent>
    </Popover>
  );
}
