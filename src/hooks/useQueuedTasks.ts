/**
 * useQueuedTasks hook - Fetch tasks in "ready" status with plan association
 *
 * Used by QueuedTasksPopover to show tasks waiting for execution slots.
 * Tasks are ordered by priority (desc) then created_at (asc) - oldest first within same priority.
 */

import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/tauri";
import { ideationApi } from "@/api/ideation";
import type { Task } from "@/types/task";

export interface QueuedTask extends Task {
  /** Plan title from ideation session, or "Standalone" if no session */
  planTitle: string;
}

/**
 * Hook to fetch queued (ready status) tasks with plan association
 *
 * @param projectId - The project ID
 * @returns Query result with queued tasks including plan titles
 */
export function useQueuedTasks(projectId: string) {
  return useQuery<QueuedTask[], Error>({
    queryKey: ["queued-tasks", projectId],
    queryFn: async () => {
      // Fetch tasks in ready status
      const response = await api.tasks.list({
        projectId,
        statuses: ["ready"],
        limit: 100, // Should be enough for queue display
      });

      const tasks = response.tasks;

      // Get unique session IDs
      const sessionIds = [
        ...new Set(
          tasks
            .map((t) => t.ideationSessionId)
            .filter((id): id is string => id != null)
        ),
      ];

      // Fetch all sessions in parallel
      const sessionMap = new Map<string, string>();
      if (sessionIds.length > 0) {
        const sessions = await Promise.all(
          sessionIds.map((id) => ideationApi.sessions.get(id))
        );
        sessions.forEach((session) => {
          if (session) {
            sessionMap.set(session.id, session.title || "Untitled Plan");
          }
        });
      }

      // Map tasks to QueuedTask with plan titles
      const queuedTasks: QueuedTask[] = tasks.map((task) => ({
        ...task,
        planTitle: task.ideationSessionId
          ? sessionMap.get(task.ideationSessionId) || "Unknown Plan"
          : "Standalone",
      }));

      // Sort by priority (desc) then createdAt (asc) - oldest first within same priority
      queuedTasks.sort((a, b) => {
        if (a.priority !== b.priority) {
          return b.priority - a.priority; // Higher priority first
        }
        // Same priority: oldest first
        return new Date(a.createdAt).getTime() - new Date(b.createdAt).getTime();
      });

      return queuedTasks;
    },
    enabled: Boolean(projectId),
  });
}
