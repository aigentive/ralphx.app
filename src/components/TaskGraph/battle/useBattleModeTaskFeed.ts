import { useEffect } from "react";
import { useEventBus } from "@/providers/EventProvider";
import { TaskEventSchema, TaskStatusChangedEventSchema } from "@/types/events";
import type { InternalStatus } from "@/types/status";

export interface BattleTaskStatusEvent {
  taskId: string;
  status: InternalStatus;
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
        status: parsed.data.to,
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
        status: parsed.data.new_status,
      });
    });

    return () => {
      unsubscribeTaskEvent();
      unsubscribeLegacyStatus();
    };
  }, [active, allowedTaskIds, bus, onStatusEvent]);
}
