/**
 * TaskBoard - Main kanban board component with drag-drop support
 *
 * Design spec: specs/design/pages/kanban-board.md
 * - Radial gradient background for warmth
 * - Horizontal scroll with CSS scroll-snap
 * - Fade edges at overflow boundaries
 * - 24px (--space-6) gutters between columns
 */

import { useState, useCallback, useEffect } from "react";
import {
  DndContext,
  DragOverlay,
  PointerSensor,
  useSensor,
  useSensors,
  type DragStartEvent,
  type DragEndEvent,
  type DragOverEvent,
} from "@dnd-kit/core";
import { useTaskBoard } from "./hooks";
import { TaskBoardSkeleton } from "./TaskBoardSkeleton";
import { Column } from "./Column";
import { TaskCard } from "./TaskCard";
import { useUiStore } from "@/stores/uiStore";
import type { Task } from "@/types/task";

export interface TaskBoardProps {
  projectId: string;
  workflowId: string;
}

export function TaskBoard({ projectId, workflowId }: TaskBoardProps) {
  const { columns, onDragEnd, isLoading, error } = useTaskBoard(
    projectId,
    workflowId
  );
  const [activeTask, setActiveTask] = useState<Task | null>(null);
  const [overColumnId, setOverColumnId] = useState<string | null>(null);
  const [movingTaskId, setMovingTaskId] = useState<string | null>(null);
  const openModal = useUiStore((s) => s.openModal);

  // Clear movingTaskId after React has re-rendered with new position
  useEffect(() => {
    if (movingTaskId) {
      const id = requestAnimationFrame(() => {
        setMovingTaskId(null);
      });
      return () => cancelAnimationFrame(id);
    }
  }, [movingTaskId]);

  // Distance-based activation - drag starts after moving 8px
  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: {
        distance: 8,
      },
    })
  );

  const handleTaskSelect = useCallback(
    (taskId: string) => {
      const task = columns.flatMap((c) => c.tasks).find((t) => t.id === taskId);
      if (task) {
        openModal("task-detail", { task });
      }
    },
    [columns, openModal]
  );

  if (isLoading) {
    return <TaskBoardSkeleton />;
  }

  if (error) {
    return (
      <div
        data-testid="task-board-error"
        className="p-4 rounded-lg"
        style={{
          backgroundColor: "var(--bg-surface)",
          color: "var(--status-error)",
        }}
      >
        Error: {error.message}
      </div>
    );
  }

  const handleDragStart = (event: DragStartEvent) => {
    const task = columns
      .flatMap((c) => c.tasks)
      .find((t) => t.id === event.active.id);
    setActiveTask(task || null);
  };

  const handleDragOver = (event: DragOverEvent) => {
    setOverColumnId(event.over?.id.toString() || null);
  };

  const handleDragEnd = (event: DragEndEvent) => {
    const taskId = String(event.active.id);
    // Keep the moved task hidden until after re-render
    setMovingTaskId(taskId);
    // Trigger optimistic update FIRST (onMutate is synchronous)
    onDragEnd(event);
    setActiveTask(null);
    setOverColumnId(null);
  };

  const handleDragCancel = () => {
    setActiveTask(null);
    setOverColumnId(null);
  };

  // Locked columns that can't receive drops
  const lockedColumns = ["in_progress", "in_review", "done"];

  return (
    <DndContext
      sensors={sensors}
      onDragStart={handleDragStart}
      onDragOver={handleDragOver}
      onDragEnd={handleDragEnd}
      onDragCancel={handleDragCancel}
    >
      {/* TaskBoard container with radial gradient and scroll-snap */}
      <div
        data-testid="task-board"
        className="task-board relative flex items-stretch gap-3 py-6 overflow-x-auto h-full"
        style={{
          background:
            "radial-gradient(ellipse at top, rgba(255,107,53,0.03) 0%, var(--bg-base) 50%)",
          scrollSnapType: "x proximity",
          scrollPaddingLeft: "16px",
        }}
      >
        {/* Left spacer for scroll padding */}
        <div className="w-4 flex-shrink-0" aria-hidden="true" />

        {columns.map((column) => (
          <Column
            key={column.id}
            column={column}
            isOver={overColumnId === column.id}
            isInvalid={
              overColumnId === column.id && lockedColumns.includes(column.id)
            }
            onTaskSelect={handleTaskSelect}
            hiddenTaskId={movingTaskId}
          />
        ))}
      </div>
      <DragOverlay dropAnimation={null}>
        {activeTask && <TaskCard task={activeTask} isDragging />}
      </DragOverlay>
    </DndContext>
  );
}
