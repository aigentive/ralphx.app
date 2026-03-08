/**
 * useMetricsConfig - Fetch and save per-project EME calibration config
 */

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { projectStatsApi } from "@/api/project-stats";
import { projectStatsKeys } from "@/hooks/useProjectStats";
import type { MetricsConfig } from "@/types/project-stats";

// ============================================================================
// Query key factory
// ============================================================================

export const metricsConfigKeys = {
  all: ["metrics-config"] as const,
  detail: (projectId: string) =>
    [...metricsConfigKeys.all, "detail", projectId] as const,
};

// ============================================================================
// Hooks
// ============================================================================

/**
 * Fetch the EME calibration config for a project.
 *
 * @param projectId - The project to fetch config for
 * @returns TanStack Query result with MetricsConfig data
 */
export function useMetricsConfig(projectId: string) {
  return useQuery<MetricsConfig, Error>({
    queryKey: metricsConfigKeys.detail(projectId),
    queryFn: () => projectStatsApi.getMetricsConfig(projectId),
    staleTime: 10 * 60 * 1000,
  });
}

/**
 * Save the EME calibration config for a project.
 * On success, invalidates both the metrics config cache and the project stats
 * cache so the EME estimate recomputes with the new config.
 *
 * @param projectId - The project to save config for
 * @returns TanStack Mutation result
 */
export function useSaveMetricsConfig(projectId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (config: MetricsConfig) =>
      projectStatsApi.saveMetricsConfig(projectId, config),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: metricsConfigKeys.detail(projectId),
      });
      queryClient.invalidateQueries({
        queryKey: projectStatsKeys.byProject(projectId),
      });
    },
  });
}
