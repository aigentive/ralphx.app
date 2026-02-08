/**
 * useMergeValidationEvents hook - Real-time merge validation step listener
 *
 * Subscribes to merge:validation_step events from the backend and accumulates
 * validation step data for display in MergingTaskDetail.
 *
 * Uses EventBus abstraction for browser/Tauri compatibility.
 */

import { useState, useEffect } from "react";
import { useEventBus } from "@/providers/EventProvider";
import {
  MergeValidationStepEventSchema,
  type MergeValidationStepEvent,
} from "@/types/events";

/**
 * Hook to listen for merge validation step events for a specific task.
 *
 * Updates existing steps (running→success/failed) by matching on command+phase,
 * or appends new steps.
 *
 * @param taskId - The task ID to filter events for
 * @returns Array of validation step events (live, accumulated)
 */
export function useMergeValidationEvents(taskId: string): MergeValidationStepEvent[] {
  const [steps, setSteps] = useState<MergeValidationStepEvent[]>([]);
  const bus = useEventBus();

  useEffect(() => {
    setSteps([]);
    const unsub = bus.subscribe<unknown>("merge:validation_step", (payload) => {
      const parsed = MergeValidationStepEventSchema.safeParse(payload);
      if (!parsed.success || parsed.data.task_id !== taskId) return;
      const step = parsed.data;
      setSteps((prev) => {
        const idx = prev.findIndex(
          (s) => s.command === step.command && s.phase === step.phase
        );
        if (idx >= 0) {
          const updated = [...prev];
          updated[idx] = step;
          return updated;
        }
        return [...prev, step];
      });
    });
    return unsub;
  }, [bus, taskId]);

  return steps;
}
