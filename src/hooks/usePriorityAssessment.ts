/**
 * usePriorityAssessment hook - TanStack Query wrapper for priority assessment
 *
 * Provides mutations for assessing priority of individual proposals or
 * batch-assessing all proposals in a session.
 */

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { ideationApi, type PriorityAssessmentResponse } from "@/api/ideation";
import { proposalKeys } from "./useProposals";
import { ideationKeys } from "./useIdeation";

/**
 * Hook for priority assessment mutations
 *
 * @returns Object with mutation functions for priority assessment
 *
 * @example
 * ```tsx
 * const { assessPriority, assessAllPriorities } = usePriorityAssessment();
 *
 * // Assess single proposal
 * const handleAssess = async (proposalId: string) => {
 *   const result = await assessPriority.mutateAsync(proposalId);
 *   console.log(`Priority: ${result.priority} (${result.score}/100)`);
 *   console.log(`Reason: ${result.reason}`);
 * };
 *
 * // Assess all proposals in session
 * const handleAssessAll = async (sessionId: string) => {
 *   const results = await assessAllPriorities.mutateAsync(sessionId);
 *   console.log(`Assessed ${results.length} proposals`);
 * };
 * ```
 */
export function usePriorityAssessment() {
  const queryClient = useQueryClient();

  const assessPriority = useMutation<PriorityAssessmentResponse, Error, string>({
    mutationFn: (proposalId) => ideationApi.proposals.assessPriority(proposalId),
    onSuccess: (result) => {
      // Invalidate the proposal to refresh with new priority
      queryClient.invalidateQueries({
        queryKey: proposalKeys.detail(result.proposalId),
      });
      // Also invalidate proposal lists
      queryClient.invalidateQueries({
        queryKey: proposalKeys.lists(),
      });
    },
  });

  const assessAllPriorities = useMutation<PriorityAssessmentResponse[], Error, string>({
    mutationFn: (sessionId) => ideationApi.proposals.assessAllPriorities(sessionId),
    onSuccess: (_results, sessionId) => {
      // Invalidate the proposal list for this session
      queryClient.invalidateQueries({
        queryKey: proposalKeys.list(sessionId),
      });
      // Also invalidate session with data
      queryClient.invalidateQueries({
        queryKey: ideationKeys.sessionWithData(sessionId),
      });
    },
  });

  return {
    assessPriority,
    assessAllPriorities,
  };
}
