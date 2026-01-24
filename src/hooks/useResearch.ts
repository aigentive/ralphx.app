/**
 * useResearch hooks - TanStack Query wrappers for research process operations
 *
 * Provides hooks for:
 * - useResearchProcesses: Fetch all research processes, optionally filtered by status
 * - useResearchProcess: Fetch a single research process by ID
 * - useResearchPresets: Fetch available depth presets
 * - Mutation hooks for start/pause/resume/stop operations
 */

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import * as researchApi from "@/lib/api/research";
import type {
  ResearchProcessResponse,
  ResearchPresetResponse,
  StartResearchInput,
} from "@/lib/api/research";

// ============================================================================
// Query Keys
// ============================================================================

/**
 * Query key factory for research
 */
export const researchKeys = {
  all: ["research"] as const,
  processes: () => [...researchKeys.all, "processes"] as const,
  processList: (status?: string) =>
    [...researchKeys.processes(), "list", status] as const,
  processDetails: () => [...researchKeys.processes(), "detail"] as const,
  processDetail: (id: string) => [...researchKeys.processDetails(), id] as const,
  presets: () => [...researchKeys.all, "presets"] as const,
};

// ============================================================================
// Query Hooks
// ============================================================================

/**
 * Hook to fetch all research processes, optionally filtered by status
 *
 * @param status - Optional status filter (pending, running, paused, completed, failed)
 * @returns TanStack Query result with processes array
 *
 * @example
 * ```tsx
 * const { data: processes } = useResearchProcesses("running");
 * return <ProcessList processes={processes} />;
 * ```
 */
export function useResearchProcesses(status?: string) {
  return useQuery<ResearchProcessResponse[], Error>({
    queryKey: researchKeys.processList(status),
    queryFn: () => researchApi.getResearchProcesses(status),
    staleTime: 10 * 1000, // 10 seconds (research status changes frequently)
    refetchInterval: 30 * 1000, // Auto-refetch every 30s for running processes
  });
}

/**
 * Hook to fetch a single research process by ID
 *
 * @param id - The research process ID to fetch
 * @returns TanStack Query result with process data or null
 */
export function useResearchProcess(id: string) {
  return useQuery<ResearchProcessResponse | null, Error>({
    queryKey: researchKeys.processDetail(id),
    queryFn: () => researchApi.getResearchProcess(id),
    enabled: !!id,
    staleTime: 10 * 1000, // 10 seconds
    refetchInterval: (query) => {
      // Auto-refetch running/paused processes
      const status = query.state.data?.status;
      if (status === "running" || status === "paused") {
        return 10 * 1000; // 10 seconds
      }
      return false;
    },
  });
}

/**
 * Hook to fetch available research depth presets
 *
 * @returns TanStack Query result with presets array
 */
export function useResearchPresets() {
  return useQuery<ResearchPresetResponse[], Error>({
    queryKey: researchKeys.presets(),
    queryFn: researchApi.getResearchPresets,
    staleTime: 5 * 60 * 1000, // 5 minutes (presets rarely change)
  });
}

// ============================================================================
// Mutation Hooks
// ============================================================================

/**
 * Hook to start a new research process
 *
 * @returns TanStack Mutation for starting research
 */
export function useStartResearch() {
  const queryClient = useQueryClient();

  return useMutation<ResearchProcessResponse, Error, StartResearchInput>({
    mutationFn: researchApi.startResearch,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: researchKeys.processes() });
    },
  });
}

/**
 * Hook to pause a running research process
 *
 * @returns TanStack Mutation for pausing research
 */
export function usePauseResearch() {
  const queryClient = useQueryClient();

  return useMutation<ResearchProcessResponse, Error, string>({
    mutationFn: researchApi.pauseResearch,
    onSuccess: (process) => {
      queryClient.invalidateQueries({ queryKey: researchKeys.processes() });
      queryClient.invalidateQueries({
        queryKey: researchKeys.processDetail(process.id),
      });
    },
  });
}

/**
 * Hook to resume a paused research process
 *
 * @returns TanStack Mutation for resuming research
 */
export function useResumeResearch() {
  const queryClient = useQueryClient();

  return useMutation<ResearchProcessResponse, Error, string>({
    mutationFn: researchApi.resumeResearch,
    onSuccess: (process) => {
      queryClient.invalidateQueries({ queryKey: researchKeys.processes() });
      queryClient.invalidateQueries({
        queryKey: researchKeys.processDetail(process.id),
      });
    },
  });
}

/**
 * Hook to stop/cancel a research process
 *
 * @returns TanStack Mutation for stopping research
 */
export function useStopResearch() {
  const queryClient = useQueryClient();

  return useMutation<ResearchProcessResponse, Error, string>({
    mutationFn: researchApi.stopResearch,
    onSuccess: (process) => {
      queryClient.invalidateQueries({ queryKey: researchKeys.processes() });
      queryClient.invalidateQueries({
        queryKey: researchKeys.processDetail(process.id),
      });
    },
  });
}
