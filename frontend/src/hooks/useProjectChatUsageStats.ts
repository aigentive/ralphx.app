import { useQuery } from "@tanstack/react-query";
import { getProjectChatUsageStats } from "@/api/metrics";
import type { ScopeUsageStats } from "@/api/metrics";

export const projectChatUsageStatsKeys = {
  all: ["project-chat-usage-stats"] as const,
  byProject: (projectId: string) => [...projectChatUsageStatsKeys.all, projectId] as const,
};

export function useProjectChatUsageStats(projectId: string | undefined) {
  return useQuery<ScopeUsageStats, Error>({
    queryKey: projectChatUsageStatsKeys.byProject(projectId ?? ""),
    queryFn: () => getProjectChatUsageStats(projectId!),
    enabled: !!projectId,
    staleTime: 30_000,
    gcTime: 5 * 60_000,
  });
}
