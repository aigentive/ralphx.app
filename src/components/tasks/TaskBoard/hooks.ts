/**
 * useTaskBoard hook - Manages task board state and drag-drop operations
 */

import { useMemo, useCallback } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import type { DragEndEvent } from "@dnd-kit/core";
import { api } from "@/lib/tauri";
import { taskKeys } from "@/hooks/useTasks";
import type { Task, InternalStatus } from "@/types/task";
import type { WorkflowColumn, WorkflowSchema } from "@/types/workflow";

export interface BoardColumn extends WorkflowColumn {
  tasks: Task[];
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

export function useTaskBoard(projectId: string, workflowId: string): UseTaskBoardResult {
  const queryClient = useQueryClient();

  const { data: tasks = [], isLoading: tasksLoading, error: tasksError } = useQuery<Task[], Error>({
    queryKey: taskKeys.list(projectId),
    queryFn: () => api.tasks.list(projectId),
  });

  const { data: workflow, isLoading: workflowLoading, error: workflowError } = useQuery<WorkflowSchema, Error>({
    queryKey: workflowKeys.detail(workflowId),
    queryFn: () => api.workflows.get(workflowId),
  });

  const moveMutation = useMutation({
    mutationFn: ({ taskId, toStatus }: { taskId: string; toStatus: string }) =>
      api.tasks.move(taskId, toStatus),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: taskKeys.list(projectId) });
    },
  });

  const columns = useMemo<BoardColumn[]>(() => {
    if (!workflow) return [];
    return workflow.columns.map((column) => ({
      ...column,
      tasks: tasks
        .filter((task) => task.internalStatus === column.mapsTo)
        .sort((a, b) => a.priority - b.priority),
    }));
  }, [workflow, tasks]);

  const onDragEnd = useCallback((event: DragEndEvent) => {
    const { active, over } = event;
    if (!over || !workflow) return;

    const taskId = String(active.id);
    const targetColumn = workflow.columns.find((c) => c.id === String(over.id));
    if (!targetColumn) return;

    const task = tasks.find((t) => t.id === taskId);
    if (!task || targetColumn.mapsTo === task.internalStatus) return;

    moveMutation.mutate({ taskId, toStatus: targetColumn.mapsTo as InternalStatus });
  }, [workflow, tasks, moveMutation]);

  return {
    columns,
    onDragEnd,
    isLoading: tasksLoading || workflowLoading,
    error: tasksError || workflowError || null,
  };
}
