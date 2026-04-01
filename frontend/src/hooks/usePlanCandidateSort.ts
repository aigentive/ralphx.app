/**
 * Plan candidate sorting hook
 *
 * Sorts plan candidates by priority tier and recency.
 * Tier 0 (activeNow > 0) > Tier 1 (incomplete > 0) > Tier 2 (complete/empty)
 * Within same tier: most recent (acceptedAt DESC) first
 */

import { useMemo } from "react";
import type { PlanCandidate } from "@/stores/planStore";

/**
 * Determine the priority tier for a plan candidate
 * @param candidate - The plan candidate to evaluate
 * @returns Tier number (0 = highest priority, 2 = lowest)
 */
export function getCandidateTier(candidate: PlanCandidate): number {
  const { activeNow, incomplete } = candidate.taskStats;

  // Tier 0: Has tasks actively executing
  if (activeNow > 0) {
    return 0;
  }

  // Tier 1: Has incomplete tasks but none active
  if (incomplete > 0) {
    return 1;
  }

  // Tier 2: All complete or no tasks
  return 2;
}

/**
 * Sort plan candidates by tier and recency
 * @param candidates - Array of plan candidates to sort
 * @returns Sorted array (does not mutate original)
 */
export function sortPlanCandidates(candidates: PlanCandidate[]): PlanCandidate[] {
  // Create shallow copy to avoid mutating original
  return [...candidates].sort((a, b) => {
    const tierA = getCandidateTier(a);
    const tierB = getCandidateTier(b);

    // Primary sort: tier (ascending - lower tier = higher priority)
    if (tierA !== tierB) {
      return tierA - tierB;
    }

    // Secondary sort: acceptedAt (descending - most recent first)
    const dateA = new Date(a.acceptedAt).getTime();
    const dateB = new Date(b.acceptedAt).getTime();
    return dateB - dateA;
  });
}

/**
 * Hook to sort plan candidates with memoization
 * @param candidates - Array of plan candidates to sort
 * @returns Sorted array of plan candidates
 */
export function usePlanCandidateSort(candidates: PlanCandidate[]): PlanCandidate[] {
  return useMemo(() => sortPlanCandidates(candidates), [candidates]);
}
