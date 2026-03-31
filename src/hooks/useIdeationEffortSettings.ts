/**
 * useIdeationEffortSettings hook - TanStack Query integration for ideation effort settings
 *
 * Provides:
 * - Loading state (no spinner — placeholderData renders immediately)
 * - Error handling
 * - Optimistic updates with rollback on failure
 *
 * Call with projectId=null for global settings, projectId=<id> for per-project overrides.
 */

import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  ideationEffortApi,
  defaultIdeationEffortSettings,
} from "@/api/ideation-effort";
import type { IdeationEffortResponse } from "@/api/ideation-effort";

function effortQueryKey(projectId: string | null) {
  return ["ideation", "effort", projectId] as const;
}

export function useIdeationEffortSettings(projectId: string | null) {
  const queryClient = useQueryClient();
  const queryKey = effortQueryKey(projectId);

  const query = useQuery({
    queryKey,
    queryFn: () => ideationEffortApi.get(projectId),
    staleTime: 1000 * 60 * 5, // 5 minutes
    gcTime: 1000 * 60 * 10, // 10 minutes
    placeholderData: defaultIdeationEffortSettings,
  });

  const mutation = useMutation({
    mutationFn: (updates: { primaryEffort?: string; verifierEffort?: string }) =>
      ideationEffortApi.update({ projectId, ...updates }),
    onMutate: async (updates) => {
      // Cancel outgoing refetches
      await queryClient.cancelQueries({ queryKey });

      // Snapshot previous value for rollback
      const previous =
        queryClient.getQueryData<IdeationEffortResponse>(queryKey);

      // Optimistically apply the change
      if (previous) {
        queryClient.setQueryData(queryKey, { ...previous, ...updates });
      }

      return { previous };
    },
    onError: (_err, _updates, context) => {
      // Rollback on error
      if (context?.previous) {
        queryClient.setQueryData(queryKey, context.previous);
      }
    },
    onSuccess: (updated) => {
      // Replace with server response (includes resolved effective values)
      queryClient.setQueryData(queryKey, updated);
    },
  });

  return {
    settings: query.data ?? defaultIdeationEffortSettings,
    isLoading: query.isLoading,
    isPlaceholderData: query.isPlaceholderData,
    isError: query.isError,
    error: query.error,
    updateSettings: mutation.mutate,
    isUpdating: mutation.isPending,
    saveError: mutation.error,
    resetError: mutation.reset,
  };
}
