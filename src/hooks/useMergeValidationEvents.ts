/**
 * useMergeValidationEvents hook - Real-time merge validation step listener
 *
 * Thin wrapper around useValidationEvents that filters for merge context.
 * Subscribes to merge:validation_step events from the backend and accumulates
 * validation step data for display in MergingTaskDetail.
 */

import { useValidationEvents } from "./useValidationEvents";
import type { MergeValidationStepEvent } from "@/types/events";

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
  return useValidationEvents(taskId, "merge");
}
