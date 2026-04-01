/**
 * useTeamHistory — TanStack Query hook for fetching historical team activity
 *
 * Fetches the most recent completed team session for a given context.
 * Used to hydrate TeamActivityPanel with past session data when no
 * live team is active (e.g., revisiting a completed task execution).
 */

import { useQuery } from "@tanstack/react-query";
import { getTeamHistory } from "@/api/team";
import { teamKeys } from "@/hooks/useTeamStatus";

export function useTeamHistory(contextType: string, contextId: string) {
  return useQuery({
    queryKey: teamKeys.history(contextType, contextId),
    queryFn: () => getTeamHistory(contextType, contextId),
    enabled: !!contextType && !!contextId,
    staleTime: 30_000,
  });
}
