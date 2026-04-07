import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import {
  defaultAgentHarnessLanes,
  agentHarnessApi,
  type UpdateAgentHarnessLaneInput,
} from "@/api/ideation-harness";

function harnessQueryKey(projectId: string | null) {
  return ["agent", "harness", projectId] as const;
}

export function useAgentHarnessSettings(projectId: string | null) {
  const queryClient = useQueryClient();
  const queryKey = harnessQueryKey(projectId);

  const query = useQuery({
    queryKey,
    queryFn: () => agentHarnessApi.get(projectId),
    staleTime: 1000 * 60 * 5,
    gcTime: 1000 * 60 * 10,
    placeholderData: defaultAgentHarnessLanes,
  });

  const mutation = useMutation({
    mutationFn: (input: Omit<UpdateAgentHarnessLaneInput, "projectId">) =>
      agentHarnessApi.update({ projectId, ...input }),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["agent", "harness"] });
    },
  });

  return {
    lanes: query.data ?? defaultAgentHarnessLanes,
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
