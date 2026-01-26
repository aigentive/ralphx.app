/**
 * useTaskBoard hook - Manages task board state and drag-drop operations
 */

import { useMemo, useCallback } from "react";
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
import type { Task, InternalStatus, TaskListResponse } from "@/types/task";
import type { WorkflowColumn, WorkflowSchema } from "@/types/workflow";

export interface UseTaskBoardResult {
  columns: WorkflowColumn[];
  onDragEnd: (event: DragEndEvent) => void;
  isLoading: boolean;
  error: Error | null;
}

export const workflowKeys = {
  all: ["workflows"] as const,
  detail: (id: string) => [...workflowKeys.all, id] as const,
};

export function useTaskBoard(
  projectId: string,
  workflowId: string
): UseTaskBoardResult {
  const queryClient = useQueryClient();
  const showArchived = useUiStore((s) => s.showArchived);

  const {
    data: workflow,
    isLoading: workflowLoading,
    error: workflowError,
  } = useQuery<WorkflowSchema, Error>({
    queryKey: workflowKeys.detail(workflowId),
    queryFn: () => api.workflows.get(workflowId),
  });

  const moveMutation = useMutation({
    mutationFn: ({ taskId, toStatus }: { taskId: string; toStatus: string }) =>
      api.tasks.move(taskId, toStatus),
    // Optimistic update - immediately move task across columns in infinite query caches
    onMutate: async ({ taskId, toStatus }) => {
      if (!workflow) return;

      // Cancel all outgoing infinite query refetches
      await Promise.all(
        workflow.columns.map((col) =>
          queryClient.cancelQueries({
            queryKey: infiniteTaskKeys.list({
              projectId,
              status: col.mapsTo,
              includeArchived: showArchived,
            }),
          })
        )
      );

      // Store snapshots for rollback
      const snapshots = new Map();
      workflow.columns.forEach((col) => {
        const key = infiniteTaskKeys.list({
          projectId,
          status: col.mapsTo,
          includeArchived: showArchived,
        });
        snapshots.set(col.id, queryClient.getQueryData(key));
      });

      // Find the task and its current column by checking each column's cache
      let movedTask: Task | undefined;
      let fromColumn: WorkflowColumn | undefined;

      for (const col of workflow.columns) {
        const key = infiniteTaskKeys.list({
          projectId,
          status: col.mapsTo,
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
        status: fromColumn.mapsTo,
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
      const toColumn = workflow.columns.find((c) => c.mapsTo === toStatus);
      if (toColumn && movedTask) {
        const toKey = infiniteTaskKeys.list({
          projectId,
          status: toColumn.mapsTo,
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
      if (!context?.snapshots || !workflow) return;
      workflow.columns.forEach((col) => {
        const snapshot = context.snapshots.get(col.id);
        if (snapshot) {
          const key = infiniteTaskKeys.list({
            projectId,
            status: col.mapsTo,
            includeArchived: showArchived,
          });
          queryClient.setQueryData(key, snapshot);
        }
      });
    },
    // Sync with server after mutation settles
    onSettled: () => {
      if (!workflow) return;
      workflow.columns.forEach((col) => {
        queryClient.invalidateQueries({
          queryKey: infiniteTaskKeys.list({
            projectId,
            status: col.mapsTo,
            includeArchived: showArchived,
          }),
        });
      });
    },
  });

  const columns = useMemo<WorkflowColumn[]>(() => {
    if (!workflow) return [];
    return workflow.columns;
  }, [workflow]);

  const onDragEnd = useCallback(
    (event: DragEndEvent) => {
      const { active, over } = event;
      if (!over || !workflow) return;

      const taskId = String(active.id);
      const targetColumn = workflow.columns.find(
        (c) => c.id === String(over.id)
      );
      if (!targetColumn) return;

      // Find the task's current status from the query cache
      let currentStatus: string | undefined;
      for (const col of workflow.columns) {
        const key = infiniteTaskKeys.list({
          projectId,
          status: col.mapsTo,
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
    [workflow, projectId, showArchived, queryClient, moveMutation]
  );

  return {
    columns,
    onDragEnd,
    isLoading: workflowLoading,
    error: workflowError ?? null,
  };
}
