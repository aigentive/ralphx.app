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
import {
  useInfiniteTasksQuery,
  flattenPages,
  infiniteTaskKeys,
} from "@/hooks/useInfiniteTasksQuery";
import type { Task, InternalStatus, TaskListResponse } from "@/types/task";
import type { WorkflowColumn, WorkflowSchema } from "@/types/workflow";

export interface BoardColumn extends WorkflowColumn {
  tasks: Task[];
  fetchNextPage?: () => void;
  hasNextPage?: boolean;
  isFetchingNextPage?: boolean;
  isLoading?: boolean;
}

export interface UseTaskBoardResult {
  columns: BoardColumn[];
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

  // Create infinite queries for each column based on their mapped status
  const columnQueries = useMemo(() => {
    if (!workflow) return new Map();

    const queries = new Map<
      string,
      ReturnType<typeof useInfiniteTasksQuery>
    >();

    workflow.columns.forEach((column) => {
      // eslint-disable-next-line react-hooks/rules-of-hooks
      const query = useInfiniteTasksQuery({
        projectId,
        status: column.mapsTo,
        includeArchived: showArchived,
      });
      queries.set(column.id, query);
    });

    return queries;
  }, [workflow, projectId, showArchived]);

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

      // Find the task and its current column
      let movedTask: Task | undefined;
      let fromColumn: WorkflowColumn | undefined;

      workflow.columns.forEach((col) => {
        const query = columnQueries.get(col.id);
        const tasks = flattenPages(query?.data);
        const task = tasks.find((t) => t.id === taskId);
        if (task) {
          movedTask = task;
          fromColumn = col;
        }
      });

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

  const columns = useMemo<BoardColumn[]>(() => {
    if (!workflow) return [];

    return workflow.columns.map((column) => {
      const query = columnQueries.get(column.id);
      const tasks = query ? flattenPages(query.data) : [];

      return {
        ...column,
        tasks: tasks.sort((a, b) => a.priority - b.priority),
        fetchNextPage: query?.fetchNextPage,
        hasNextPage: query?.hasNextPage,
        isFetchingNextPage: query?.isFetchingNextPage,
        isLoading: query?.isLoading,
      };
    });
  }, [workflow, columnQueries]);

  const onDragEnd = useCallback(
    (event: DragEndEvent) => {
      const { active, over } = event;
      if (!over || !workflow) return;

      const taskId = String(active.id);
      const targetColumn = workflow.columns.find(
        (c) => c.id === String(over.id)
      );
      if (!targetColumn) return;

      // Find the task across all columns
      let task: Task | undefined;
      for (const column of columns) {
        task = column.tasks.find((t) => t.id === taskId);
        if (task) break;
      }

      if (!task || targetColumn.mapsTo === task.internalStatus) return;

      moveMutation.mutate({
        taskId,
        toStatus: targetColumn.mapsTo as InternalStatus,
      });
    },
    [workflow, columns, moveMutation]
  );

  // Determine overall loading state - loading if workflow is loading OR any initial column load
  const isLoading = useMemo(() => {
    if (workflowLoading) return true;
    // Check if any column is in initial loading state
    return Array.from(columnQueries.values()).some((q) => q.isLoading);
  }, [workflowLoading, columnQueries]);

  // Determine overall error state
  const error = useMemo(() => {
    if (workflowError) return workflowError;
    // Check if any column has an error
    for (const query of columnQueries.values()) {
      if (query.error) return query.error;
    }
    return null;
  }, [workflowError, columnQueries]);

  return {
    columns,
    onDragEnd,
    isLoading,
    error,
  };
}
