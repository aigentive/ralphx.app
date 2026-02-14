/**
 * Plan candidate sorting hook
 *
 * Sorts plan candidates into tiers based on task execution status:
 * - Tier 0: Plans with actively executing tasks (activeNow > 0)
 * - Tier 1: Plans with incomplete work but no active execution (incomplete > 0, activeNow === 0)
 * - Tier 2: Plans with all tasks complete or no tasks
 *
 * Within each tier, candidates are sorted by most recently accepted first.
 */

import { useMemo } from "react";
import type { PlanCandidate } from "@/stores/planStore";

/**
 * Get the tier number for a plan candidate.
 * Lower tier numbers have higher priority.
 */
function getCandidateTier(c: PlanCandidate): number {
  if (c.taskStats.activeNow > 0) return 0; // Tier 0: actively executing
  if (c.taskStats.incomplete > 0) return 1; // Tier 1: has remaining work
  return 2; // Tier 2: all complete or no tasks
}

/**
 * Pure function to sort plan candidates by tier and acceptance date.
 * Returns a new array; does not mutate the input.
 *
 * Sorting rules:
 * 1. Primary: tier (0 > 1 > 2)
 * 2. Secondary: acceptedAt (most recent first within tier)
 */
export function sortPlanCandidates(
  candidates: PlanCandidate[]
): PlanCandidate[] {
  return [...candidates].sort((a, b) => {
    const aTier = getCandidateTier(a);
    const bTier = getCandidateTier(b);

    if (aTier !== bTier) {
      return aTier - bTier;
    }

    // Within same tier: most recently accepted first
    return (
      new Date(b.acceptedAt).getTime() - new Date(a.acceptedAt).getTime()
    );
  });
}

/**
 * Hook wrapper that memoizes the sort result.
 * Returns a new sorted array when the input array reference changes.
 */
export function usePlanCandidateSort(
  candidates: PlanCandidate[]
): PlanCandidate[] {
  return useMemo(() => sortPlanCandidates(candidates), [candidates]);
}
