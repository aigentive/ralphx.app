/**
 * useColumnTaskCounts — reactive per-column task counts from React Query cache
 *
 * Subscribes to queryClient.getQueryCache() via useSyncExternalStore for
 * sub-frame reactivity including optimistic drag-drop updates.
 * Uses ref-based memoization to prevent infinite re-renders from new Map instances.
 */

import { useRef, useCallback, useSyncExternalStore } from "react";
import { useQueryClient, type InfiniteData } from "@tanstack/react-query";
import { infiniteTaskKeys } from "@/hooks/useInfiniteTasksQuery";
import type { WorkflowColumn } from "@/types/workflow";
import type { InternalStatus } from "@/types/status";
import type { Task, TaskListResponse } from "@/types/task";

/**
 * Extract all statuses for a column, handling groups
 */
function getColumnStatuses(col: WorkflowColumn): InternalStatus[] {
  if (col.groups && col.groups.length > 0) {
    const allStatuses = new Set<InternalStatus>();
    for (const group of col.groups) {
      for (const status of group.statuses) {
        allStatuses.add(status);
      }
    }
    return Array.from(allStatuses);
  }
  return [col.mapsTo];
}

/**
 * Compare two Map<string, number> for shallow equality
 */
function mapsEqual(a: Map<string, number>, b: Map<string, number>): boolean {
  if (a.size !== b.size) return false;
  for (const [key, val] of a) {
    if (b.get(key) !== val) return false;
  }
  return true;
}

/**
 * Hook that provides reactive per-column task counts by subscribing to
 * the React Query cache. Returns a stable Map<string, number> reference
 * that only changes when actual counts differ.
 *
 * @param columns - Workflow columns to count tasks for
 * @param projectId - Current project ID
 * @param showArchived - Whether archived tasks are included
 * @param ideationSessionId - Optional plan filter
 * @param showMergeTasks - Whether merge tasks are included in counts
 * @param executionPlanId - Optional execution plan filter (mutually exclusive with ideationSessionId)
 * @returns Map from column ID to task count
 */
export function useColumnTaskCounts(
  columns: WorkflowColumn[],
  projectId: string,
  showArchived: boolean,
  ideationSessionId?: string | null,
  showMergeTasks: boolean = true,
  executionPlanId?: string | null,
): Map<string, number> {
  const queryClient = useQueryClient();
  const prevRef = useRef<Map<string, number>>(new Map());

  const getSnapshot = useCallback((): Map<string, number> => {
    const next = new Map<string, number>();

    for (const col of columns) {
      const key = infiniteTaskKeys.list({
        projectId,
        statuses: getColumnStatuses(col),
        includeArchived: showArchived,
        ideationSessionId,
        executionPlanId,
      });
      const data = queryClient.getQueryData<InfiniteData<TaskListResponse>>(key);
      let count = 0;
      if (data?.pages) {
        for (const page of data.pages) {
          if (showMergeTasks) {
            count += page.tasks.length;
          } else {
            count += page.tasks.filter((t: Task) => t.category !== "plan_merge").length;
          }
        }
      }
      next.set(col.id, count);
    }

    // Ref-based memoization: return previous reference if values unchanged
    if (mapsEqual(prevRef.current, next)) {
      return prevRef.current;
    }
    prevRef.current = next;
    return next;
  }, [columns, projectId, showArchived, ideationSessionId, showMergeTasks, executionPlanId, queryClient]);

  const subscribe = useCallback(
    (onStoreChange: () => void) => queryClient.getQueryCache().subscribe(onStoreChange),
    [queryClient],
  );

  return useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
}
