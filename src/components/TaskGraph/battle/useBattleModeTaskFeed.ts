import { useEffect } from "react";
import { useEventBus } from "@/providers/EventProvider";
import { TaskEventSchema, TaskStatusChangedEventSchema } from "@/types/events";
import type { InternalStatus } from "@/types/status";

export interface BattleTaskStatusEvent {
  taskId: string;
  fromStatus: InternalStatus | null;
  toStatus: InternalStatus;
  timestamp: number;
  source: "task:event" | "task:status_changed";
}

interface UseBattleModeTaskFeedOptions {
  active: boolean;
  allowedTaskIds: Set<string>;
  onStatusEvent: (event: BattleTaskStatusEvent) => void;
}

export function useBattleModeTaskFeed({
  active,
  allowedTaskIds,
  onStatusEvent,
}: UseBattleModeTaskFeedOptions) {
  const bus = useEventBus();

  useEffect(() => {
    if (!active) return;

    const unsubscribeTaskEvent = bus.subscribe<unknown>("task:event", (payload) => {
      const parsed = TaskEventSchema.safeParse(payload);
      if (!parsed.success || parsed.data.type !== "status_changed") {
        return;
      }

      const taskId = parsed.data.taskId;
      if (!allowedTaskIds.has(taskId)) return;

      onStatusEvent({
        taskId,
        fromStatus: parsed.data.from,
        toStatus: parsed.data.to,
        timestamp: Date.now(),
        source: "task:event",
      });
    });

    const unsubscribeLegacyStatus = bus.subscribe<unknown>("task:status_changed", (payload) => {
      const parsed = TaskStatusChangedEventSchema.safeParse(payload);
      if (!parsed.success) {
        return;
      }

      const taskId = parsed.data.task_id;
      if (!allowedTaskIds.has(taskId)) return;

      onStatusEvent({
        taskId,
        fromStatus: parsed.data.old_status,
        toStatus: parsed.data.new_status,
        timestamp: Date.now(),
        source: "task:status_changed",
      });
    });

    return () => {
      unsubscribeTaskEvent();
      unsubscribeLegacyStatus();
    };
  }, [active, allowedTaskIds, bus, onStatusEvent]);
}
