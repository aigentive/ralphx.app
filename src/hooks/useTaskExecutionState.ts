/**
 * useTaskExecutionState hook - Combines task data and step progress for execution state tracking
 *
 * Provides real-time execution state information combining task status,
 * step progress, and timing data for reactive UI components.
 */

import { useMemo, useState, useEffect } from "react";
import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/tauri";
import { taskKeys } from "./useTasks";
import { useStepProgress } from "./useTaskSteps";
import type { Task } from "@/types/task";
import type { StepProgressSummary } from "@/types/task-step";

/**
 * Execution phase derived from task status
 */
export type ExecutionPhase = "idle" | "executing" | "qa" | "review" | "done";

/**
 * Task execution state combining status, progress, and timing
 */
export interface TaskExecutionState {
  /** Whether task has recent activity (executing or in active workflow) */
  isActive: boolean;
  /** Duration in seconds since task started, null if not started */
  duration: number | null;
  /** Current execution phase based on internal status */
  phase: ExecutionPhase;
  /** Step progress summary, null if no steps */
  stepProgress: StepProgressSummary | null;
}

/**
 * Determine execution phase from internal status
 */
function getExecutionPhase(status: string): ExecutionPhase {
  if (status === "executing") {
    return "executing";
  }
  if (status.startsWith("qa_")) {
    return "qa";
  }
  if (status === "pending_review" || status === "revision_needed") {
    return "review";
  }
  if (status === "approved" || status === "failed" || status === "cancelled") {
    return "done";
  }
  return "idle";
}

/**
 * Calculate duration since task started
 */
function calculateDuration(startedAt: string | null): number | null {
  if (!startedAt) return null;

  const start = new Date(startedAt);
  const now = new Date();
  const diffMs = now.getTime() - start.getTime();
  return Math.floor(diffMs / 1000); // Return seconds
}

/**
 * Hook to get task execution state
 *
 * Combines task data and step progress to provide a comprehensive view
 * of task execution state. Updates duration every second when task is active.
 *
 * @param taskId - The task ID to track execution state for
 * @returns Task execution state with activity, duration, phase, and step progress
 *
 * @example
 * ```tsx
 * const { isActive, duration, phase, stepProgress } = useTaskExecutionState("task-123");
 *
 * if (isActive) {
 *   return (
 *     <div>
 *       Phase: {phase}
 *       Duration: {formatDuration(duration)}
 *       Progress: {stepProgress?.completed}/{stepProgress?.total}
 *     </div>
 *   );
 * }
 * ```
 */
export function useTaskExecutionState(taskId: string): TaskExecutionState {
  // Fetch task data
  const { data: task } = useQuery<Task, Error>({
    queryKey: taskKeys.detail(taskId),
    queryFn: () => api.tasks.get(taskId),
    enabled: Boolean(taskId),
    staleTime: 5_000, // 5 seconds for active tasks
  });

  // Fetch step progress (auto-polls when in progress)
  const { data: stepProgress } = useStepProgress(taskId);

  // State for live duration updates
  const [currentTime, setCurrentTime] = useState(() => Date.now());

  // Update current time every second when task is executing
  useEffect(() => {
    if (!task?.startedAt) return;

    const phase = getExecutionPhase(task.internalStatus);
    if (phase === "idle" || phase === "done") return;

    const interval = setInterval(() => {
      setCurrentTime(Date.now());
    }, 1000);

    return () => clearInterval(interval);
  }, [task?.startedAt, task?.internalStatus]);

  // Compute execution state
  const executionState = useMemo<TaskExecutionState>(() => {
    if (!task) {
      return {
        isActive: false,
        duration: null,
        phase: "idle",
        stepProgress: null,
      };
    }

    const phase = getExecutionPhase(task.internalStatus);
    // Recalculate duration on each render when active (triggered by currentTime state)
    const duration = calculateDuration(task.startedAt);
    const isActive = phase !== "idle" && phase !== "done";

    return {
      isActive,
      duration,
      phase,
      stepProgress: stepProgress ?? null,
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [task, stepProgress, currentTime]); // currentTime triggers recalc every second

  return executionState;
}

/**
 * Format duration in seconds to human-readable string
 * @param seconds Duration in seconds
 * @returns Formatted string like "2m 15s" or "1h 23m"
 *
 * @example
 * ```tsx
 * formatDuration(135); // "2m 15s"
 * formatDuration(3665); // "1h 1m"
 * formatDuration(45); // "45s"
 * ```
 */
export function formatDuration(seconds: number | null): string {
  if (seconds === null || seconds < 0) return "0s";

  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = seconds % 60;

  const parts: string[] = [];
  if (hours > 0) parts.push(`${hours}h`);
  if (minutes > 0) parts.push(`${minutes}m`);
  if (secs > 0 || parts.length === 0) parts.push(`${secs}s`);

  return parts.join(" ");
}
