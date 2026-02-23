/**
 * useAuditTrail - Unified audit trail hook
 * Merges review state history and activity events into a chronological audit trail
 */

import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { api, type ReviewNoteResponse } from "@/lib/tauri";
import { activityEventsApi } from "@/api/activity-events";
import type { ActivityEventResponse } from "@/api/activity-events.types";
import { reviewKeys } from "@/hooks/useReviews";

// ============================================================================
// Types
// ============================================================================

export interface AuditEntry {
  id: string;
  timestamp: string;
  source: "review" | "activity";
  type: string;
  actor: string;
  description: string;
  metadata?: string | undefined;
  status?: string | undefined;
}

// ============================================================================
// Query Keys
// ============================================================================

const auditTrailKeys = {
  all: ["auditTrail"] as const,
  byTask: (taskId: string) => [...auditTrailKeys.all, taskId] as const,
};

// ============================================================================
// Mappers
// ============================================================================

function mapReviewToAuditEntry(review: ReviewNoteResponse): AuditEntry {
  const actorLabel = review.reviewer === "ai" ? "AI Reviewer" : "Human Reviewer";

  const outcomeLabels: Record<string, string> = {
    approved: "Approved",
    changes_requested: "Changes Requested",
    rejected: "Rejected",
  };

  return {
    id: `review-${review.id}`,
    timestamp: review.created_at,
    source: "review",
    type: outcomeLabels[review.outcome] ?? review.outcome,
    actor: actorLabel,
    description: review.notes ?? review.summary ?? "",
    metadata: review.issues?.length
      ? `${review.issues.length} issue${review.issues.length === 1 ? "" : "s"} found`
      : undefined,
  };
}

function mapActivityToAuditEntry(event: ActivityEventResponse): AuditEntry {
  const roleLabels: Record<string, string> = {
    agent: "Agent",
    system: "System",
    user: "User",
  };

  return {
    id: `activity-${event.id}`,
    timestamp: event.createdAt,
    source: "activity",
    type: event.eventType,
    actor: roleLabels[event.role] ?? event.role,
    description: event.content,
    metadata: event.metadata ?? undefined,
    status: event.internalStatus ?? undefined,
  };
}

// ============================================================================
// Hook
// ============================================================================

export function useAuditTrail(
  taskId: string,
  options: { enabled?: boolean } = {}
) {
  const { enabled = true } = options;

  const reviewQuery = useQuery<ReviewNoteResponse[], Error>({
    queryKey: reviewKeys.stateHistoryById(taskId),
    queryFn: () => api.reviews.getTaskStateHistory(taskId),
    enabled: enabled && !!taskId,
    staleTime: 30 * 1000,
    gcTime: 5 * 60 * 1000,
    refetchOnWindowFocus: false,
  });

  const activityQuery = useQuery({
    queryKey: auditTrailKeys.byTask(taskId),
    queryFn: () => activityEventsApi.task.list(taskId, { limit: 100 }),
    enabled: enabled && !!taskId,
    staleTime: 30 * 1000,
    gcTime: 5 * 60 * 1000,
    refetchOnWindowFocus: false,
  });

  const entries = useMemo(() => {
    const reviewEntries = (reviewQuery.data ?? []).map(mapReviewToAuditEntry);
    const activityEntries = (activityQuery.data?.events ?? []).map(mapActivityToAuditEntry);

    return [...reviewEntries, ...activityEntries].sort(
      (a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime()
    );
  }, [reviewQuery.data, activityQuery.data]);

  const isLoading = reviewQuery.isLoading || activityQuery.isLoading;
  const isEmpty = entries.length === 0;
  const error = reviewQuery.error?.message ?? activityQuery.error?.message ?? null;

  return {
    entries,
    isLoading,
    isEmpty,
    error,
    refetch: () => {
      reviewQuery.refetch();
      activityQuery.refetch();
    },
  };
}
