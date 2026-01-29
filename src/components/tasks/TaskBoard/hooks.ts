/**
 * useTaskBoard hook - Manages task board state and drag-drop operations
 */

import { useCallback } from "react";
import {
  useQuery,
  useMutation,
  useQueryClient,
  type InfiniteData,
} from "@tanstack/react-query";
import type { DragEndEvent } from "@dnd-kit/core";
import { api } from "@/lib/tauri";
import { useUiStore } from "@/stores/uiStore";
import { infiniteTaskKeys } from "@/hooks/useInfiniteTasksQuery";
import { getActiveWorkflowColumns, type WorkflowColumnResponse } from "@/lib/api/workflows";
import { workflowKeys } from "@/hooks/useWorkflows";
import type { Task, InternalStatus, TaskListResponse } from "@/types/task";

/**
 * Get all statuses for a column from its groups, or fallback to mapsTo
 */
function getColumnStatuses(col: WorkflowColumnResponse): InternalStatus[] {
  if (col.groups && col.groups.length > 0) {
    const allStatuses = new Set<InternalStatus>();
    col.groups.forEach((group) => {
      group.statuses.forEach((status) => allStatuses.add(status));
    });
    return Array.from(allStatuses);
  }
  return [col.mapsTo];
}

export interface UseTaskBoardResult {
  columns: WorkflowColumnResponse[];
  onDragEnd: (event: DragEndEvent) => void;
  isLoading: boolean;
  error: Error | null;
}

export function useTaskBoard(
  projectId: string
): UseTaskBoardResult {
  const queryClient = useQueryClient();
  const showArchived = useUiStore((s) => s.showArchived);

  const {
    data: columns = [],
    isLoading: columnsLoading,
    error: columnsError,
  } = useQuery<WorkflowColumnResponse[], Error>({
    queryKey: workflowKeys.activeColumns(),
    queryFn: getActiveWorkflowColumns,
    staleTime: 60 * 1000, // 1 minute
  });

  const moveMutation = useMutation({
    mutationFn: ({ taskId, toStatus }: { taskId: string; toStatus: string }) =>
      api.tasks.move(taskId, toStatus),
    // Optimistic update - immediately move task across columns in infinite query caches
    onMutate: async ({ taskId, toStatus }) => {
      if (!columns.length) return;

      // Cancel all outgoing infinite query refetches
      await Promise.all(
        columns.map((col) =>
          queryClient.cancelQueries({
            queryKey: infiniteTaskKeys.list({
              projectId,
              statuses: getColumnStatuses(col),
              includeArchived: showArchived,
            }),
          })
        )
      );

      // Store snapshots for rollback
      const snapshots = new Map();
      columns.forEach((col) => {
        const key = infiniteTaskKeys.list({
          projectId,
          statuses: getColumnStatuses(col),
          includeArchived: showArchived,
        });
        snapshots.set(col.id, queryClient.getQueryData(key));
      });

      // Find the task and its current column by checking each column's cache
      let movedTask: Task | undefined;
      let fromColumn: WorkflowColumnResponse | undefined;

      for (const col of columns) {
        const key = infiniteTaskKeys.list({
          projectId,
          statuses: getColumnStatuses(col),
          includeArchived: showArchived,
        });
        const data = queryClient.getQueryData<InfiniteData<TaskListResponse>>(key);
        if (data?.pages) {
          for (const page of data.pages) {
            const task = page.tasks.find((t: Task) => t.id === taskId);
            if (task) {
              movedTask = task;
              fromColumn = col;
              break;
            }
          }
        }
        if (movedTask) break;
      }

      if (!movedTask || !fromColumn) return { snapshots };

      // Remove from source column's cache
      const fromKey = infiniteTaskKeys.list({
        projectId,
        statuses: getColumnStatuses(fromColumn),
        includeArchived: showArchived,
      });
      queryClient.setQueryData<InfiniteData<TaskListResponse>>(
        fromKey,
        (old) => {
          if (!old?.pages) return old;
          return {
            ...old,
            pages: old.pages.map((page) => ({
              ...page,
              tasks: page.tasks.filter((t: Task) => t.id !== taskId),
            })),
          };
        }
      );

      // Add to target column's cache (first page)
      const toColumn = columns.find((c) => c.mapsTo === toStatus);
      if (toColumn && movedTask) {
        const toKey = infiniteTaskKeys.list({
          projectId,
          statuses: getColumnStatuses(toColumn),
          includeArchived: showArchived,
        });
        queryClient.setQueryData<InfiniteData<TaskListResponse>>(
          toKey,
          (old) => {
            if (!old?.pages) return old;
            const updatedTask = {
              ...movedTask,
              internalStatus: toStatus as InternalStatus,
            } as Task;
            return {
              ...old,
              pages: old.pages.map((page, idx: number) =>
                idx === 0
                  ? {
                      ...page,
                      tasks: [updatedTask, ...page.tasks],
                    }
                  : page
              ),
            };
          }
        );
      }

      return { snapshots };
    },
    // Rollback on error
    onError: (_err, _variables, context) => {
      if (!context?.snapshots || !columns.length) return;
      columns.forEach((col) => {
        const snapshot = context.snapshots.get(col.id);
        if (snapshot) {
          const key = infiniteTaskKeys.list({
            projectId,
            statuses: getColumnStatuses(col),
            includeArchived: showArchived,
          });
          queryClient.setQueryData(key, snapshot);
        }
      });
    },
    // Sync with server after mutation settles
    onSettled: () => {
      if (!columns.length) return;
      columns.forEach((col) => {
        queryClient.invalidateQueries({
          queryKey: infiniteTaskKeys.list({
            projectId,
            statuses: getColumnStatuses(col),
            includeArchived: showArchived,
          }),
        });
      });
    },
  });

  const onDragEnd = useCallback(
    (event: DragEndEvent) => {
      const { active, over } = event;
      if (!over || !columns.length) return;

      const taskId = String(active.id);
      const targetColumn = columns.find(
        (c) => c.id === String(over.id)
      );
      if (!targetColumn) return;

      // Find the task's current status from the query cache
      let currentStatus: string | undefined;
      for (const col of columns) {
        const key = infiniteTaskKeys.list({
          projectId,
          statuses: getColumnStatuses(col),
          includeArchived: showArchived,
        });
        const data = queryClient.getQueryData<InfiniteData<TaskListResponse>>(key);
        if (data?.pages) {
          for (const page of data.pages) {
            const task = page.tasks.find((t: Task) => t.id === taskId);
            if (task) {
              currentStatus = task.internalStatus;
              break;
            }
          }
        }
        if (currentStatus) break;
      }

      if (!currentStatus || targetColumn.mapsTo === currentStatus) return;

      moveMutation.mutate({
        taskId,
        toStatus: targetColumn.mapsTo as InternalStatus,
      });
    },
    [columns, projectId, showArchived, queryClient, moveMutation]
  );

  return {
    columns,
    onDragEnd,
    isLoading: columnsLoading,
    error: columnsError ?? null,
  };
}
