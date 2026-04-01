/**
 * useReviewMutations hook - TanStack Query mutations for review operations
 *
 * Provides mutations for:
 * - Approving reviews (human approval after AI passes)
 * - Requesting changes (human requesting revisions)
 *
 * With automatic cache invalidation and toast notifications.
 */

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { api } from "@/lib/tauri";
import { reviewKeys } from "./useReviews";
import { taskKeys } from "./useTasks";

/**
 * Hook for review mutation operations
 *
 * @returns Object containing review mutations
 *
 * @example
 * ```tsx
 * const { approve, requestChanges } = useReviewMutations();
 *
 * // Approve a review (human confirms AI approval)
 * approve.mutate({ reviewId: "review-123", notes: "Looks good!" });
 *
 * // Request changes
 * requestChanges.mutate({
 *   reviewId: "review-123",
 *   notes: "Please fix the error handling",
 *   fixDescription: "Add null check in auth.ts"
 * });
 * ```
 */
export function useReviewMutations() {
  const queryClient = useQueryClient();

  const approve = useMutation({
    mutationFn: ({ reviewId, notes }: { reviewId: string; notes?: string }) =>
      api.reviews.approve({
        review_id: reviewId,
        ...(notes !== undefined && { notes }),
      }),
    onSuccess: () => {
      // Invalidate review queries
      queryClient.invalidateQueries({ queryKey: reviewKeys.all });
      // Invalidate task queries since status may have changed
      queryClient.invalidateQueries({ queryKey: taskKeys.all });
      toast.success("Review approved");
    },
    onError: (error: Error) => {
      toast.error(`Failed to approve review: ${error.message}`);
    },
  });

  const requestChanges = useMutation({
    mutationFn: ({
      reviewId,
      notes,
      fixDescription,
    }: {
      reviewId: string;
      notes: string;
      fixDescription?: string;
    }) =>
      api.reviews.requestChanges({
        review_id: reviewId,
        notes,
        ...(fixDescription !== undefined && { fix_description: fixDescription }),
      }),
    onSuccess: (fixTaskId) => {
      // Invalidate review queries
      queryClient.invalidateQueries({ queryKey: reviewKeys.all });
      // Invalidate task queries since status changed and fix task may have been created
      queryClient.invalidateQueries({ queryKey: taskKeys.all });

      if (fixTaskId) {
        toast.success("Changes requested - fix task created");
      } else {
        toast.success("Changes requested");
      }
    },
    onError: (error: Error) => {
      toast.error(`Failed to request changes: ${error.message}`);
    },
  });

  return {
    /** Approve a review (human confirms after AI approval) */
    approve,
    /** Request changes on a review */
    requestChanges,
    /** Whether an approval is in progress */
    isApproving: approve.isPending,
    /** Whether a request changes operation is in progress */
    isRequestingChanges: requestChanges.isPending,
  };
}
