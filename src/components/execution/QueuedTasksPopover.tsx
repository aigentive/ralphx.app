/**
 * QueuedTasksPopover - Compact queued tasks list
 *
 * Dense row-based layout showing tasks waiting for execution slots.
 */

import {
  Popover,
  PopoverAnchor,
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
  /** Optional anchor ref for popover positioning (aligns to status indicator) */
  anchorRef?: React.RefObject<HTMLDivElement | null>;
}

export function QueuedTasksPopover({
  projectId,
  queuedCount,
  children,
  anchorRef,
}: QueuedTasksPopoverProps) {
  const { data: queuedTasks, isLoading } = useQueuedTasks(projectId);

  return (
    <Popover>
      {anchorRef && <PopoverAnchor virtualRef={anchorRef as React.RefObject<HTMLDivElement>} />}
      <PopoverTrigger asChild>{children}</PopoverTrigger>
      <PopoverContent
        side="top"
        align="start"
        className="w-[400px] p-0"
        style={{
          backgroundColor: "hsl(220 10% 11%)",
          border: "1px solid hsla(220 20% 100% / 0.08)",
          borderRadius: "10px",
          boxShadow:
            "0 4px 16px hsla(220 20% 0% / 0.4), 0 12px 32px hsla(220 20% 0% / 0.3)",
        }}
      >
        {/* Header */}
        <div
          className="flex items-center justify-between px-3 py-2.5"
          style={{
            borderBottom: "1px solid hsla(220 20% 100% / 0.06)",
          }}
        >
          <h3
            className="text-xs font-semibold"
            style={{ color: "hsl(220 10% 80%)" }}
          >
            Queued Tasks ({queuedCount})
          </h3>
        </div>

        {/* Task List */}
        <div
          className="max-h-[320px] overflow-y-auto p-1.5"
          style={{
            scrollbarWidth: "thin",
            scrollbarColor: "hsla(220 10% 100% / 0.1) transparent",
          }}
        >
          {isLoading ? (
            <div
              className="py-6 text-center text-xs"
              style={{ color: "hsl(220 10% 42%)" }}
            >
              Loading...
            </div>
          ) : !queuedTasks || queuedTasks.length === 0 ? (
            <div
              className="py-6 text-center text-xs"
              style={{ color: "hsl(220 10% 42%)" }}
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

        {/* Footer */}
        <div
          className="px-3 py-2 text-[11px]"
          style={{
            borderTop: "1px solid hsla(220 20% 100% / 0.06)",
            color: "hsl(220 10% 42%)",
          }}
        >
          Ready tasks queued by priority, oldest first within same priority.
        </div>
      </PopoverContent>
    </Popover>
  );
}
