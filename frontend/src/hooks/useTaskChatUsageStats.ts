import { useQuery } from "@tanstack/react-query";
import { getTaskChatUsageStats } from "@/api/metrics";
import type { ScopeUsageStats } from "@/api/metrics";

export const taskChatUsageStatsKeys = {
  all: ["task-chat-usage-stats"] as const,
  byTask: (taskId: string) => [...taskChatUsageStatsKeys.all, taskId] as const,
};

export function useTaskChatUsageStats(taskId: string | undefined) {
  return useQuery<ScopeUsageStats, Error>({
    queryKey: taskChatUsageStatsKeys.byTask(taskId ?? ""),
    queryFn: () => getTaskChatUsageStats(taskId!),
    enabled: !!taskId,
    staleTime: 30_000,
    gcTime: 5 * 60_000,
  });
}
