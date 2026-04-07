import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  defaultIdeationHarnessLanes,
  ideationHarnessApi,
  type UpdateIdeationHarnessLaneInput,
} from "@/api/ideation-harness";

function harnessQueryKey(projectId: string | null) {
  return ["ideation", "harness", projectId] as const;
}

export function useIdeationHarnessSettings(projectId: string | null) {
  const queryClient = useQueryClient();
  const queryKey = harnessQueryKey(projectId);

  const query = useQuery({
    queryKey,
    queryFn: () => ideationHarnessApi.get(projectId),
    staleTime: 1000 * 60 * 5,
    gcTime: 1000 * 60 * 10,
    placeholderData: defaultIdeationHarnessLanes,
  });

  const mutation = useMutation({
    mutationFn: (input: Omit<UpdateIdeationHarnessLaneInput, "projectId">) =>
      ideationHarnessApi.update({ projectId, ...input }),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["ideation", "harness"] });
    },
  });

  return {
    lanes: query.data ?? defaultIdeationHarnessLanes,
    isLoading: query.isLoading,
    isPlaceholderData: query.isPlaceholderData,
    isError: query.isError,
    error: query.error,
    updateLane: mutation.mutate,
    isUpdating: mutation.isPending,
    saveError: mutation.error,
    resetError: mutation.reset,
  };
}
