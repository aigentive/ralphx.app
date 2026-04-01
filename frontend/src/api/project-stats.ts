/**
 * project-stats API — Tauri invoke wrapper for get_project_stats
 *
 * Uses Zod schema validation to ensure type safety at the boundary.
 */

import { invoke } from "@tauri-apps/api/core";
import { MetricsConfigSchema, ProjectStatsSchema } from "@/types/project-stats";
import type { MetricsConfig, ProjectStats } from "@/types/project-stats";

// ============================================================================
// API object
// ============================================================================

export const projectStatsApi = {
  /**
   * Fetch aggregated statistics for a project.
   *
   * @param projectId - The project ID to fetch stats for
   * @returns Validated ProjectStats object
   */
  async getProjectStats(projectId: string): Promise<ProjectStats> {
    const result = await invoke("get_project_stats", { projectId });
    return ProjectStatsSchema.parse(result);
  },

  /**
   * Fetch the EME calibration config for a project.
   *
   * @param projectId - The project ID
   * @returns Validated MetricsConfig object
   */
  async getMetricsConfig(projectId: string): Promise<MetricsConfig> {
    const result = await invoke("get_metrics_config", { projectId });
    return MetricsConfigSchema.parse(result);
  },

  /**
   * Save the EME calibration config for a project.
   *
   * @param projectId - The project ID
   * @param config - The calibration config to save
   */
  async saveMetricsConfig(projectId: string, config: MetricsConfig): Promise<void> {
    await invoke("save_metrics_config", { projectId, config });
  },
};
