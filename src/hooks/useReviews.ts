/**
 * Review hooks - React hooks for review data fetching
 *
 * Provides hooks for:
 * - usePendingReviews: Pending reviews for a project
 * - useReviewsByTaskId: All reviews for a specific task
 * - useTaskStateHistory: State transition history for a task
 * - useTasksAwaitingReview: Tasks awaiting AI or human review
 */

import { useEffect, useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { api, type ReviewResponse, type ReviewNoteResponse } from "@/lib/tauri";
import { tasksApi } from "@/api/tasks";
import { useReviewStore } from "@/stores/reviewStore";
import type { Task } from "@/types/task";
import type { InternalStatus } from "@/types/status";

// ============================================================================
// Query Keys
// ============================================================================

export const reviewKeys = {
  all: ["reviews"] as const,
  pending: () => [...reviewKeys.all, "pending"] as const,
  pendingByProject: (projectId: string) =>
    [...reviewKeys.pending(), projectId] as const,
  byTask: () => [...reviewKeys.all, "byTask"] as const,
  byTaskId: (taskId: string) => [...reviewKeys.byTask(), taskId] as const,
  stateHistory: () => [...reviewKeys.all, "stateHistory"] as const,
  stateHistoryById: (taskId: string) =>
    [...reviewKeys.stateHistory(), taskId] as const,
  tasksAwaitingReview: () => [...reviewKeys.all, "tasksAwaiting"] as const,
  tasksAwaitingReviewByProject: (projectId: string) =>
    [...reviewKeys.tasksAwaitingReview(), projectId] as const,
};

// ============================================================================
// Status Constants for Review Filtering
// ============================================================================

/** Statuses where task is in AI review phase */
const AI_REVIEW_STATUSES: InternalStatus[] = ["pending_review", "reviewing"];

/** Statuses where task awaits human review decision */
const HUMAN_REVIEW_STATUSES: InternalStatus[] = ["review_passed", "escalated"];

// ============================================================================
// usePendingReviews
// ============================================================================

/**
 * Hook to fetch pending reviews for a project
 *
 * @param projectId - The project ID to fetch reviews for
 * @param options - Hook options
 * @returns Pending reviews, loading state, and computed properties
 *
 * @example
 * ```tsx
 * const { data, isLoading, count, isEmpty } = usePendingReviews("project-123");
 *
 * if (isLoading) return <Spinner />;
 * if (isEmpty) return <p>No pending reviews</p>;
 * return <ReviewList reviews={data} />;
 * ```
 */
export function usePendingReviews(
  projectId: string,
  options: { enabled?: boolean } = {}
) {
  const { enabled = true } = options;

  const setPendingReviews = useReviewStore((s) => s.setPendingReviews);
  const setLoading = useReviewStore((s) => s.setLoading);
  const setError = useReviewStore((s) => s.setError);

  const query = useQuery<ReviewResponse[], Error>({
    queryKey: reviewKeys.pendingByProject(projectId),
    queryFn: () => api.reviews.getPending(projectId),
    enabled: enabled && !!projectId,
    staleTime: 30 * 1000, // 30 seconds
  });

  // Sync loading state to store
  useEffect(() => {
    if (query.isLoading) {
      setLoading(true);
    }
  }, [query.isLoading, setLoading]);

  // Sync data to store
  useEffect(() => {
    if (query.data) {
      setPendingReviews(query.data);
    }
  }, [query.data, setPendingReviews]);

  // Sync error to store
  useEffect(() => {
    if (query.error) {
      setError(query.error.message);
    }
  }, [query.error, setError]);

  const data = query.data ?? [];
  const isEmpty = data.length === 0;
  const count = data.length;

  return {
    /** Pending reviews array */
    data,
    /** Whether data is loading */
    isLoading: query.isLoading,
    /** Error message if any */
    error: query.error?.message ?? null,
    /** Whether there are no pending reviews */
    isEmpty,
    /** Number of pending reviews */
    count,
    /** Refetch reviews from backend */
    refetch: query.refetch,
  };
}

// ============================================================================
// useReviewsByTaskId
// ============================================================================

/**
 * Hook to fetch all reviews for a specific task
 *
 * @param taskId - The task ID to fetch reviews for
 * @param options - Hook options
 * @returns Task reviews, loading state, and computed properties
 *
 * @example
 * ```tsx
 * const { data, hasAiReview, hasHumanReview, latestReview } = useReviewsByTaskId("task-123");
 *
 * if (hasAiReview && !hasHumanReview) {
 *   return <p>Awaiting human review</p>;
 * }
 * ```
 */
export function useReviewsByTaskId(
  taskId: string,
  options: { enabled?: boolean } = {}
) {
  const { enabled = true } = options;

  const query = useQuery<ReviewResponse[], Error>({
    queryKey: reviewKeys.byTaskId(taskId),
    queryFn: () => api.reviews.getByTaskId(taskId),
    enabled: enabled && !!taskId,
    staleTime: 30 * 1000, // 30 seconds
  });

  // Wrap data in useMemo to prevent creating new array reference on every render
  const data = useMemo(() => query.data ?? [], [query.data]);

  // Computed properties
  const hasAiReview = useMemo(
    () => data.some((r) => r.reviewer_type === "ai"),
    [data]
  );

  const hasHumanReview = useMemo(
    () => data.some((r) => r.reviewer_type === "human"),
    [data]
  );

  const latestReview = useMemo(() => {
    if (data.length === 0) return null;
    return data.reduce((latest, review) =>
      new Date(review.created_at) > new Date(latest.created_at) ? review : latest
    );
  }, [data]);

  return {
    /** All reviews for this task */
    data,
    /** Whether data is loading */
    isLoading: query.isLoading,
    /** Error message if any */
    error: query.error?.message ?? null,
    /** Whether the task has an AI review */
    hasAiReview,
    /** Whether the task has a human review */
    hasHumanReview,
    /** The most recent review */
    latestReview,
    /** Refetch reviews from backend */
    refetch: query.refetch,
  };
}

// ============================================================================
// useTaskStateHistory
// ============================================================================

/**
 * Hook to fetch state transition history for a task
 *
 * @param taskId - The task ID to fetch history for
 * @param options - Hook options
 * @returns State history, loading state, and computed properties
 *
 * @example
 * ```tsx
 * const { data, isEmpty, latestEntry } = useTaskStateHistory("task-123");
 *
 * if (isEmpty) return <p>No history</p>;
 * return <Timeline entries={data} />;
 * ```
 */
export function useTaskStateHistory(
  taskId: string,
  options: { enabled?: boolean } = {}
) {
  const { enabled = true } = options;

  const query = useQuery<ReviewNoteResponse[], Error>({
    queryKey: reviewKeys.stateHistoryById(taskId),
    queryFn: () => api.reviews.getTaskStateHistory(taskId),
    enabled: enabled && !!taskId,
    staleTime: 30 * 1000, // 30 seconds
  });

  // Sort by created_at descending (newest first)
  const data = useMemo(() => {
    const entries = query.data ?? [];
    return [...entries].sort(
      (a, b) =>
        new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
    );
  }, [query.data]);

  const isEmpty = data.length === 0;

  const latestEntry = useMemo(() => {
    if (data.length === 0) return null;
    return data[0];
  }, [data]);

  return {
    /** State history entries (sorted newest first) */
    data,
    /** Whether data is loading */
    isLoading: query.isLoading,
    /** Error message if any */
    error: query.error?.message ?? null,
    /** Whether there is no history */
    isEmpty,
    /** The most recent history entry */
    latestEntry,
    /** Refetch history from backend */
    refetch: query.refetch,
  };
}

// ============================================================================
// useTasksAwaitingReview
// ============================================================================

/**
 * Hook to fetch tasks awaiting review for a project
 *
 * Returns tasks grouped by review type:
 * - AI: Tasks in pending_review, reviewing
 * - Human: Tasks in review_passed, escalated
 *
 * @param projectId - The project ID to fetch tasks for
 * @param options - Hook options
 * @returns Tasks grouped by review type, loading state, and computed properties
 *
 * @example
 * ```tsx
 * const { allTasks, aiTasks, humanTasks, isLoading } = useTasksAwaitingReview("project-123");
 *
 * // Filter by tab
 * const displayTasks = activeTab === "ai" ? aiTasks : humanTasks;
 * ```
 */
export function useTasksAwaitingReview(
  projectId: string,
  options: { enabled?: boolean } = {}
) {
  const { enabled = true } = options;

  const query = useQuery<Task[], Error>({
    queryKey: reviewKeys.tasksAwaitingReviewByProject(projectId),
    queryFn: () => tasksApi.getTasksAwaitingReview(projectId),
    enabled: enabled && !!projectId,
    staleTime: 30 * 1000, // 30 seconds
  });

  // Wrap data in useMemo to prevent creating new array reference on every render
  const allTasks = useMemo(() => query.data ?? [], [query.data]);

  // Group tasks by review type
  const aiTasks = useMemo(
    () => allTasks.filter((t) => AI_REVIEW_STATUSES.includes(t.internalStatus)),
    [allTasks]
  );

  const humanTasks = useMemo(
    () => allTasks.filter((t) => HUMAN_REVIEW_STATUSES.includes(t.internalStatus)),
    [allTasks]
  );

  const isEmpty = allTasks.length === 0;
  const aiCount = aiTasks.length;
  const humanCount = humanTasks.length;
  const totalCount = allTasks.length;

  return {
    /** All tasks awaiting review */
    allTasks,
    /** Tasks in AI review phase (pending_review, reviewing) */
    aiTasks,
    /** Tasks awaiting human review (review_passed, escalated) */
    humanTasks,
    /** Whether data is loading */
    isLoading: query.isLoading,
    /** Error message if any */
    error: query.error?.message ?? null,
    /** Whether there are no tasks awaiting review */
    isEmpty,
    /** Number of tasks in AI review */
    aiCount,
    /** Number of tasks awaiting human review */
    humanCount,
    /** Total number of tasks awaiting review */
    totalCount,
    /** Refetch tasks from backend */
    refetch: query.refetch,
  };
}
