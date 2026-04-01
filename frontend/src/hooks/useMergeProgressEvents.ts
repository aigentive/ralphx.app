/**
 * useMergeProgressEvents hook - Real-time high-level merge phase listener
 *
 * Subscribes to task:merge_progress events from the backend and accumulates
 * phase timeline data for display in merge detail views.
 *
 * Also listens for task:merge_phases to receive the dynamic phase list
 * derived from project analysis.
 *
 * Hydrates initial state from backend on mount (events fire before frontend subscribes),
 * then merges live events on top.
 *
 * Uses EventBus abstraction for browser/Tauri compatibility.
 */

import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
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

/** Merge hydrated events into state, deduplicating by phase */
function mergeEvents(
  existing: MergeProgressEvent[],
  incoming: MergeProgressEvent[],
): MergeProgressEvent[] {
  const result = [...existing];
  for (const event of incoming) {
    const idx = result.findIndex((p) => p.phase === event.phase);
    if (idx >= 0) {
      result[idx] = event;
    } else {
      result.push(event);
    }
  }
  return result;
}

/**
 * Hook to listen for high-level merge progress events for a specific task.
 *
 * On mount, hydrates from backend store (catches events that fired before
 * the component mounted). Then subscribes to live events and merges them.
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

    // Hydrate from backend store (catches events emitted before mount)
    invoke<MergeProgressEvent[]>("get_merge_progress", { taskId })
      .then((stored) => {
        if (stored && stored.length > 0) {
          setPhases((prev) => mergeEvents(prev, stored));
        }
      })
      .catch(() => {
        // Silently ignore — Tauri not available in browser mock mode
      });

    invoke<MergePhaseInfo[] | null>("get_merge_phase_list", { taskId })
      .then((stored) => {
        if (stored) {
          setPhaseList((prev) => prev ?? stored);
        }
      })
      .catch(() => {
        // Silently ignore — Tauri not available in browser mock mode
      });

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
