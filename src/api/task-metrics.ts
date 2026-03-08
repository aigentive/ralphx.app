/**
 * Tauri API wrapper for per-task metrics.
 *
 * Provides type-safe access to the `get_task_metrics` command which returns
 * step counts, review cycles, execution time, and total age for a single task.
 * Not cached on the backend — fetched on-demand from the task detail view.
 */

import { invoke } from "@tauri-apps/api/core";
import { z } from "zod";

// ============================================================================
// Schema
// ============================================================================

/**
 * Per-task metrics returned by the `get_task_metrics` command.
 * Backend uses camelCase (`serde(rename_all = "camelCase")`).
 */
export const TaskMetricsSchema = z.object({
  /** Total steps (all statuses) */
  stepCount: z.number(),
  /** Steps with status = 'completed' */
  completedStepCount: z.number(),
  /** Number of review cycles for this task */
  reviewCount: z.number(),
  /** Approved reviews */
  approvedReviewCount: z.number(),
  /** Time spent in 'executing' or 're_executing' phases, in minutes */
  executionMinutes: z.number(),
  /** Total elapsed time from task creation to now (or merge), in hours */
  totalAgeHours: z.number(),
});

export type TaskMetrics = z.infer<typeof TaskMetricsSchema>;

/** Complexity tier derived from execution time and step count. */
export type ComplexityTier = "Simple" | "Medium" | "Complex";

/**
 * Derive complexity tier from task metrics.
 *
 * Heuristic:
 * - Simple: execution < 10 min AND steps <= 5
 * - Complex: execution >= 30 min OR steps >= 10
 * - Medium: everything else
 */
export function deriveComplexityTier(metrics: TaskMetrics): ComplexityTier {
  const { executionMinutes, stepCount } = metrics;
  if (executionMinutes >= 30 || stepCount >= 10) return "Complex";
  if (executionMinutes < 10 && stepCount <= 5) return "Simple";
  return "Medium";
}

// ============================================================================
// API
// ============================================================================

/**
 * Fetch metrics for a single task.
 *
 * @param taskId - The task ID to fetch metrics for
 * @returns Per-task metrics
 */
export async function getTaskMetrics(taskId: string): Promise<TaskMetrics> {
  const result = await invoke("get_task_metrics", { taskId });
  return TaskMetricsSchema.parse(result);
}

export const taskMetricsApi = {
  getTaskMetrics,
} as const;
