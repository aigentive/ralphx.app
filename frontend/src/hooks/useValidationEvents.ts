/**
 * useValidationEvents hook - Generalized validation step listener
 *
 * Subscribes to merge:validation_step events from the backend and accumulates
 * validation step data for display in task detail views.
 *
 * Supports filtering by context (e.g., "merge" vs "execution" vs "review") when the backend
 * emits context-qualified events.
 *
 * Also listens for merge:validation_start to clear stale steps when a new validation run begins.
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
 * Hook to listen for validation step events for a specific task.
 *
 * Updates existing steps (running→success/failed) by matching on command+phase,
 * or appends new steps. Clears all steps when a merge:validation_start event
 * is received for the same task (new validation run starting).
 *
 * @param taskId - The task ID to filter events for
 * @param context - Optional context filter ("merge" | "execution" | "review") to filter events by context
 * @returns Array of validation step events (live, accumulated)
 */
export function useValidationEvents(
  taskId: string,
  context?: "merge" | "execution" | "review"
): MergeValidationStepEvent[] {
  const [steps, setSteps] = useState<MergeValidationStepEvent[]>([]);
  const bus = useEventBus();

  useEffect(() => {
    setSteps([]);

    // Listen for validation_start to clear stale steps when a new run begins
    const unsubStart = bus.subscribe<unknown>("merge:validation_start", (payload) => {
      const data = payload as { task_id?: string } | null;
      if (data?.task_id === taskId) {
        setSteps([]);
      }
    });

    const unsub = bus.subscribe<unknown>("merge:validation_step", (payload) => {
      const parsed = MergeValidationStepEventSchema.safeParse(payload);
      if (!parsed.success || parsed.data.task_id !== taskId) return;

      // Filter by context if specified and event has context field
      if (context && parsed.data.context && parsed.data.context !== context) return;

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
    return () => {
      unsubStart();
      unsub();
    };
  }, [bus, taskId, context]);

  return steps;
}
