/**
 * Hook for fetching and managing merge pipeline data
 */

import { useQuery } from "@tanstack/react-query";
import { mergePipelineApi } from "@/api/merge-pipeline";

/**
 * Query key factory for merge pipeline queries
 */
export const mergePipelineKeys = {
  all: ["merge-pipeline"] as const,
  pipeline: () => [...mergePipelineKeys.all, "list"] as const,
};

/**
 * Hook to fetch the merge pipeline (active, waiting, needs attention)
 */
export function useMergePipeline() {
  return useQuery({
    queryKey: mergePipelineKeys.pipeline(),
    queryFn: () => mergePipelineApi.getMergePipeline(),
    refetchInterval: 5000, // Poll every 5 seconds for real-time updates
  });
}
