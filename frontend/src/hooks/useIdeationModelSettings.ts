/**
 * useIdeationModelSettings hook - TanStack Query integration for ideation model settings
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
  ideationModelApi,
  defaultIdeationModelSettings,
} from "@/api/ideation-model";
import type { IdeationModelResponse } from "@/api/ideation-model";

function modelQueryKey(projectId: string | null) {
  return ["ideation", "model", projectId] as const;
}

export function useIdeationModelSettings(projectId: string | null) {
  const queryClient = useQueryClient();
  const queryKey = modelQueryKey(projectId);

  const query = useQuery({
    queryKey,
    queryFn: () => ideationModelApi.get(projectId),
    staleTime: 1000 * 60 * 5, // 5 minutes
    gcTime: 1000 * 60 * 10, // 10 minutes
    placeholderData: defaultIdeationModelSettings,
  });

  const mutation = useMutation({
    mutationFn: (updates: { primaryModel?: string; verifierModel?: string; verifierSubagentModel?: string }) =>
      ideationModelApi.update({ projectId, ...updates }),
    onMutate: async (updates) => {
      // Cancel outgoing refetches for all model queries (prefix-level) to prevent stale refetch races
      await queryClient.cancelQueries({ queryKey: ["ideation", "model"] });

      // Snapshot previous value for rollback
      const previous =
        queryClient.getQueryData<IdeationModelResponse>(queryKey);

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
      // Invalidate all model queries so sibling "inherit" rows refetch from consistent state
      void queryClient.invalidateQueries({ queryKey: ["ideation", "model"] });
    },
    onSuccess: (updated) => {
      // Replace with server response (includes resolved effective values)
      queryClient.setQueryData(queryKey, updated);
      // Invalidate all model queries so sibling "inherit" rows get fresh effective values
      void queryClient.invalidateQueries({ queryKey: ["ideation", "model"] });
    },
  });

  return {
    settings: query.data ?? defaultIdeationModelSettings,
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
