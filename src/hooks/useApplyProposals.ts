/**
 * useApplyProposals hook - TanStack Query wrapper for applying proposals to Kanban
 *
 * Provides a mutation for converting selected proposals to tasks on the Kanban board.
 */

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { ideationApi, type ApplyProposalsResultResponse, type ApplyProposalsInput } from "@/api/ideation";
import { proposalKeys } from "./useProposals";
import { ideationKeys } from "./useIdeation";
import { taskKeys } from "./useTasks";

/**
 * Hook for applying proposals to the Kanban board
 *
 * @returns Object with apply mutation and related state
 *
 * @example
 * ```tsx
 * const { apply } = useApplyProposals();
 *
 * const handleApply = async () => {
 *   const result = await apply.mutateAsync({
 *     sessionId: "session-123",
 *     proposalIds: ["proposal-1", "proposal-2"],
 *     targetColumn: "backlog",
 *     preserveDependencies: true,
 *   });
 *
 *   if (result.warnings.length > 0) {
 *     toast.warning(`Applied with warnings: ${result.warnings.join(", ")}`);
 *   } else {
 *     toast.success(`Created ${result.createdTaskIds.length} tasks`);
 *   }
 *
 *   if (result.sessionConverted) {
 *     toast.info("All proposals applied - session converted");
 *     navigate("/kanban");
 *   }
 * };
 * ```
 */
export function useApplyProposals() {
  const queryClient = useQueryClient();

  const apply = useMutation<ApplyProposalsResultResponse, Error, ApplyProposalsInput>({
    mutationFn: (input) => ideationApi.apply.toKanban(input),
    onSuccess: (result, variables) => {
      // Invalidate task queries since new tasks were created
      queryClient.invalidateQueries({
        queryKey: taskKeys.all,
      });

      // Invalidate proposals since their status and createdTaskId changed
      queryClient.invalidateQueries({
        queryKey: proposalKeys.list(variables.sessionId),
      });

      // Invalidate session data
      queryClient.invalidateQueries({
        queryKey: ideationKeys.sessionWithData(variables.sessionId),
      });

      // If session was converted, also invalidate session list
      if (result.sessionConverted) {
        queryClient.invalidateQueries({
          queryKey: ideationKeys.sessions(),
        });
      }
    },
  });

  return {
    apply,
  };
}
