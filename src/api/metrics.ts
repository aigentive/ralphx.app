import { typedInvoke } from "@/lib/tauri";
import { ProjectStatsSchema, ProjectTrendsSchema } from "@/types/project-stats";
import type { ProjectStats, ProjectTrends } from "@/types/project-stats";

export async function getProjectStats(projectId: string): Promise<ProjectStats> {
  return typedInvoke("get_project_stats", { projectId }, ProjectStatsSchema);
}

export async function getProjectTrends(projectId: string): Promise<ProjectTrends> {
  return typedInvoke("get_project_trends", { projectId }, ProjectTrendsSchema);
}
