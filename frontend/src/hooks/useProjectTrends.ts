import { useQuery } from "@tanstack/react-query";
import { getProjectTrends } from "@/api/metrics";
import type { ProjectTrends } from "@/types/project-stats";

export const projectTrendsKeys = {
  all: ["projectTrends"] as const,
  detail: (projectId: string, weekStartDay?: number, tzOffsetMinutes?: number) =>
    [...projectTrendsKeys.all, projectId, ...(weekStartDay !== undefined ? [weekStartDay] : []), ...(tzOffsetMinutes !== undefined ? [tzOffsetMinutes] : [])] as const,
};

export function useProjectTrends(projectId: string | undefined, weekStartDay?: number, tzOffsetMinutes?: number) {
  return useQuery<ProjectTrends, Error>({
    queryKey: projectTrendsKeys.detail(projectId ?? "", weekStartDay, tzOffsetMinutes),
    queryFn: () => getProjectTrends(projectId!, weekStartDay, tzOffsetMinutes),
    enabled: !!projectId,
    staleTime: 5 * 60 * 1000, // 5 minutes
  });
}
