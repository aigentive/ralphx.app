/**
 * useMergeProgressEvents hook - Real-time high-level merge phase listener
 *
 * Subscribes to task:merge_progress events from the backend and accumulates
 * phase timeline data for display in merge detail views.
 *
 * Also listens for task:merge_phases to receive the dynamic phase list
 * derived from project analysis.
 *
 * Uses EventBus abstraction for browser/Tauri compatibility.
 */

import { useState, useEffect } from "react";
import { useEventBus } from "@/providers/EventProvider";
import {
  MergeProgressEventSchema,
  MergePhaseListEventSchema,
  type MergeProgressEvent,
  type MergePhaseInfo,
} from "@/types/events";

export type MergePhase = MergeProgressEvent["phase"];
export type MergePhaseStatus = MergeProgressEvent["status"];

export interface MergeProgressData {
  /** Live accumulated phase events */
  phases: MergeProgressEvent[];
  /** Dynamic phase list from project analysis (if received) */
  phaseList: MergePhaseInfo[] | null;
}

/**
 * Hook to listen for high-level merge progress events for a specific task.
 *
 * Updates existing phases (started→passed/failed) by matching on phase,
 * or appends new phases. Also captures the dynamic phase list from
 * the task:merge_phases event.
 *
 * @param taskId - The task ID to filter events for
 * @returns Object with phases array and phaseList
 */
export function useMergeProgressEvents(taskId: string): MergeProgressData {
  const [phases, setPhases] = useState<MergeProgressEvent[]>([]);
  const [phaseList, setPhaseList] = useState<MergePhaseInfo[] | null>(null);
  const bus = useEventBus();

  useEffect(() => {
    setPhases([]);
    setPhaseList(null);

    // Listen for individual phase progress events
    const unsubProgress = bus.subscribe<unknown>("task:merge_progress", (payload) => {
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

    // Listen for the dynamic phase list event
    const unsubPhaseList = bus.subscribe<unknown>("task:merge_phases", (payload) => {
      const parsed = MergePhaseListEventSchema.safeParse(payload);
      if (!parsed.success || parsed.data.task_id !== taskId) return;
      setPhaseList(parsed.data.phases);
    });

    return () => {
      unsubProgress();
      unsubPhaseList();
    };
  }, [bus, taskId]);

  return { phases, phaseList };
}
