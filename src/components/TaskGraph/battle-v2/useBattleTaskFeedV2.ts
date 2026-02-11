import { useEffect } from "react";
import { useEventBus } from "@/providers/EventProvider";
import { TaskEventSchema, TaskStatusChangedEventSchema } from "@/types/events";
import type { BattleTaskSyncEvent } from "./types";

interface UseBattleTaskFeedV2Options {
  active: boolean;
  allowedTaskIds: Set<string>;
  onEvent: (event: BattleTaskSyncEvent) => void;
}

export function useBattleTaskFeedV2({ active, allowedTaskIds, onEvent }: UseBattleTaskFeedV2Options) {
  const bus = useEventBus();

  useEffect(() => {
    if (!active) return;

    const unsubTaskEvent = bus.subscribe<unknown>("task:event", (payload) => {
      const parsed = TaskEventSchema.safeParse(payload);
      if (!parsed.success || parsed.data.type !== "status_changed") return;
      const taskId = parsed.data.taskId;
      if (!allowedTaskIds.has(taskId)) return;

      onEvent({
        taskId,
        fromStatus: parsed.data.from,
        toStatus: parsed.data.to,
        timestamp: Date.now(),
        source: "task:event",
      });
    });

    const unsubLegacy = bus.subscribe<unknown>("task:status_changed", (payload) => {
      const parsed = TaskStatusChangedEventSchema.safeParse(payload);
      if (!parsed.success) return;
      const taskId = parsed.data.task_id;
      if (!allowedTaskIds.has(taskId)) return;

      onEvent({
        taskId,
        fromStatus: parsed.data.old_status,
        toStatus: parsed.data.new_status,
        timestamp: Date.now(),
        source: "task:status_changed",
      });
    });

    return () => {
      unsubTaskEvent();
      unsubLegacy();
    };
  }, [active, allowedTaskIds, bus, onEvent]);
}
