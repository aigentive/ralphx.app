/**
 * Review event hooks - Tauri review event listeners with type-safe validation
 *
 * Uses EventBus abstraction for browser/Tauri compatibility.
 */

import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { useEventBus } from "@/providers/EventProvider";
import { ReviewEventSchema } from "@/types/events";
import { reviewKeys } from "@/hooks/useReviews";

/**
 * Hook to listen for review events
 *
 * Listens to 'review:update' events and invalidates TanStack Query caches
 * to trigger refetching of review-related data.
 *
 * @example
 * ```tsx
 * function ReviewsPanel() {
 *   useReviewEvents(); // Auto-refreshes review data on backend events
 *   const { data } = usePendingReviews(projectId);
 *   return <ReviewList reviews={data} />;
 * }
 * ```
 */
export function useReviewEvents() {
  const bus = useEventBus();
  const queryClient = useQueryClient();

  useEffect(() => {
    return bus.subscribe<unknown>("review:update", (payload) => {
      // Runtime validation of backend events
      const parsed = ReviewEventSchema.safeParse(payload);

      if (!parsed.success) {
        console.error("Invalid review event:", parsed.error.message);
        return;
      }

      const reviewEvent = parsed.data;

      // Always invalidate pending reviews (all events affect this)
      queryClient.invalidateQueries({
        queryKey: reviewKeys.pending(),
      });

      // Also invalidate tasks awaiting review (for ReviewsPanel badges)
      queryClient.invalidateQueries({
        queryKey: reviewKeys.tasksAwaitingReview(),
      });

      // For completed events, also invalidate task-specific queries
      if (reviewEvent.type === "completed") {
        queryClient.invalidateQueries({
          queryKey: reviewKeys.byTaskId(reviewEvent.taskId),
        });
        queryClient.invalidateQueries({
          queryKey: reviewKeys.stateHistoryById(reviewEvent.taskId),
        });
      }
    });
  }, [bus, queryClient]);
}
