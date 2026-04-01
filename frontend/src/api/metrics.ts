import { typedInvoke } from "@/lib/tauri";
import { ProjectStatsSchema, ProjectTrendsSchema } from "@/types/project-stats";
import type { ProjectStats, ProjectTrends } from "@/types/project-stats";

export async function getProjectStats(
  projectId: string,
  weekStartDay?: number,
  tzOffsetMinutes?: number,
): Promise<ProjectStats> {
  return typedInvoke(
    "get_project_stats",
    {
      projectId,
      ...(weekStartDay !== undefined && { weekStartDay }),
      ...(tzOffsetMinutes !== undefined && { tzOffsetMinutes }),
    },
    ProjectStatsSchema,
  );
}

export async function getProjectTrends(
  projectId: string,
  weekStartDay?: number,
  tzOffsetMinutes?: number,
): Promise<ProjectTrends> {
  return typedInvoke(
    "get_project_trends",
    {
      projectId,
      ...(weekStartDay !== undefined && { weekStartDay }),
      ...(tzOffsetMinutes !== undefined && { tzOffsetMinutes }),
    },
    ProjectTrendsSchema,
  );
}
