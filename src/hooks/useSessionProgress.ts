import { useMemo } from "react";
import { useTasks } from "@/hooks/useTasks";
import type { IdeationSession } from "@/types/ideation";
import { getStatusCounts, type StatusCounts } from "@/types/status";

export type SessionProgress = StatusCounts;

interface UseSessionProgressResult {
  progressMap: Map<string, SessionProgress>;
  isLoading: boolean;
}

/**
 * Computes task progress per accepted ideation session.
 *
 * Uses the single cached useTasks(projectId) query, groups tasks by
 * ideationSessionId, and returns a Map<sessionId, SessionProgress>
 * with idle/active/done/total counts.
 */
export function useSessionProgress(
  projectId: string,
  sessions: IdeationSession[]
): UseSessionProgressResult {
  const { data: allTasks, isLoading } = useTasks(projectId);

  const progressMap = useMemo(() => {
    const map = new Map<string, SessionProgress>();

    // Only compute for accepted sessions
    const acceptedSessionIds = new Set(
      sessions.filter((s) => s.status === "accepted").map((s) => s.id)
    );

    if (acceptedSessionIds.size === 0 || !allTasks) return map;

    // Group tasks by ideationSessionId in a single pass
    const tasksBySession = new Map<string, typeof allTasks>();
    for (const task of allTasks) {
      if (!task.ideationSessionId) continue;
      if (!acceptedSessionIds.has(task.ideationSessionId)) continue;

      let group = tasksBySession.get(task.ideationSessionId);
      if (!group) {
        group = [];
        tasksBySession.set(task.ideationSessionId, group);
      }
      group.push(task);
    }

    // Compute StatusCounts for each accepted session
    for (const sessionId of acceptedSessionIds) {
      const tasks = tasksBySession.get(sessionId) ?? [];
      map.set(sessionId, getStatusCounts(tasks));
    }

    return map;
  }, [allTasks, sessions]);

  return { progressMap, isLoading };
}
