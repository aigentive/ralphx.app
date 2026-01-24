/**
 * Column - Droppable column for the kanban board
 */

import { useDroppable } from "@dnd-kit/core";
import type { BoardColumn } from "./hooks";
import { TaskCard } from "./TaskCard";

interface ColumnProps {
  column: BoardColumn;
  isOver?: boolean;
  isInvalid?: boolean;
  onTaskSelect?: (taskId: string) => void;
}

function InvalidDropIcon() {
  return (
    <div data-testid="invalid-drop-icon" className="flex items-center justify-center" style={{ color: "var(--status-error)" }}>
      <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
        <circle cx="12" cy="12" r="10" />
        <path d="M15 9l-6 6M9 9l6 6" />
      </svg>
    </div>
  );
}

export function Column({ column, isOver, isInvalid, onTaskSelect }: ColumnProps) {
  const { setNodeRef } = useDroppable({ id: column.id });

  const getBorderColor = () => {
    if (isOver && isInvalid) return "var(--status-error)";
    if (isOver) return "var(--accent-primary)";
    return "var(--border-subtle)";
  };

  return (
    <div
      ref={setNodeRef}
      data-testid={`column-${column.id}`}
      className="flex-shrink-0 w-72 rounded-lg border-2 transition-colors"
      style={{ backgroundColor: "var(--bg-surface)", borderColor: getBorderColor() }}
    >
      {/* Header */}
      <div className="flex items-center justify-between p-3 border-b" style={{ borderColor: "var(--border-subtle)" }}>
        <span className="font-medium" style={{ color: "var(--text-primary)" }}>{column.name}</span>
        <div className="flex items-center gap-2">
          <span className="px-2 py-0.5 rounded-full text-xs font-medium" style={{ backgroundColor: "var(--bg-hover)", color: "var(--text-secondary)" }}>
            {column.tasks.length}
          </span>
          {isOver && isInvalid && <InvalidDropIcon />}
        </div>
      </div>

      {/* Task list */}
      <div className="p-2 space-y-2 min-h-[100px]">
        {column.tasks.map((task) => (
          <TaskCard
            key={task.id}
            task={task}
            {...(onTaskSelect !== undefined && { onSelect: onTaskSelect })}
          />
        ))}
      </div>
    </div>
  );
}
