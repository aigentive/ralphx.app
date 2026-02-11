/**
 * useMergeProgressEvents hook - Real-time high-level merge phase listener
 *
 * Subscribes to task:merge_progress events from the backend and accumulates
 * phase timeline data for display in merge detail views.
 *
 * Uses EventBus abstraction for browser/Tauri compatibility.
 */

import { useState, useEffect } from "react";
import { useEventBus } from "@/providers/EventProvider";
import {
  MergeProgressEventSchema,
  type MergeProgressEvent,
} from "@/types/events";

export type MergePhase = MergeProgressEvent["phase"];
export type MergePhaseStatus = MergeProgressEvent["status"];

/**
 * Hook to listen for high-level merge progress events for a specific task.
 *
 * Updates existing phases (started→passed/failed) by matching on phase,
 * or appends new phases.
 *
 * @param taskId - The task ID to filter events for
 * @returns Array of merge progress events (live, accumulated)
 */
export function useMergeProgressEvents(taskId: string): MergeProgressEvent[] {
  const [phases, setPhases] = useState<MergeProgressEvent[]>([]);
  const bus = useEventBus();

  useEffect(() => {
    setPhases([]);
    const unsub = bus.subscribe<unknown>("task:merge_progress", (payload) => {
      const parsed = MergeProgressEventSchema.safeParse(payload);
      if (!parsed.success || parsed.data.task_id !== taskId) return;
      const event = parsed.data;
      setPhases((prev) => {
        const idx = prev.findIndex((p) => p.phase === event.phase);
        if (idx >= 0) {
          const updated = [...prev];
          updated[idx] = event;
          return updated;
        }
        return [...prev, event];
      });
    });
    return unsub;
  }, [bus, taskId]);

  return phases;
}
