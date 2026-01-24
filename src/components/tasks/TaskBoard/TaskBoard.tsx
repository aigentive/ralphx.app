/**
 * TaskBoard - Main kanban board component with drag-drop support
 */

import { useState } from "react";
import { DndContext, DragOverlay, type DragStartEvent, type DragEndEvent, type DragOverEvent } from "@dnd-kit/core";
import { useTaskBoard } from "./hooks";
import { TaskBoardSkeleton } from "./TaskBoardSkeleton";
import { Column } from "./Column";
import { TaskCard } from "./TaskCard";
import type { Task } from "@/types/task";

export interface TaskBoardProps {
  projectId: string;
  workflowId: string;
}

export function TaskBoard({ projectId, workflowId }: TaskBoardProps) {
  const { columns, onDragEnd, isLoading, error } = useTaskBoard(projectId, workflowId);
  const [activeTask, setActiveTask] = useState<Task | null>(null);
  const [overColumnId, setOverColumnId] = useState<string | null>(null);

  if (isLoading) {
    return <TaskBoardSkeleton />;
  }

  if (error) {
    return (
      <div data-testid="task-board-error" className="p-4 rounded-lg" style={{ backgroundColor: "var(--bg-surface)", color: "var(--status-error)" }}>
        Error: {error.message}
      </div>
    );
  }

  const handleDragStart = (event: DragStartEvent) => {
    const task = columns.flatMap((c) => c.tasks).find((t) => t.id === event.active.id);
    setActiveTask(task || null);
  };

  const handleDragOver = (event: DragOverEvent) => {
    setOverColumnId(event.over?.id.toString() || null);
  };

  const handleDragEnd = (event: DragEndEvent) => {
    setActiveTask(null);
    setOverColumnId(null);
    onDragEnd(event);
  };

  const handleDragCancel = () => {
    setActiveTask(null);
    setOverColumnId(null);
  };

  // Locked columns that can't receive drops
  const lockedColumns = ["in_progress", "in_review", "done"];

  return (
    <DndContext onDragStart={handleDragStart} onDragOver={handleDragOver} onDragEnd={handleDragEnd} onDragCancel={handleDragCancel}>
      <div data-testid="task-board" className="flex gap-4 overflow-x-auto p-4" style={{ backgroundColor: "var(--bg-base)" }}>
        {columns.map((column) => (
          <Column
            key={column.id}
            column={column}
            isOver={overColumnId === column.id}
            isInvalid={overColumnId === column.id && lockedColumns.includes(column.id)}
          />
        ))}
      </div>
      <DragOverlay>
        {activeTask && <TaskCard task={activeTask} isDragging />}
      </DragOverlay>
    </DndContext>
  );
}
