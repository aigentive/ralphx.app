/**
 * Column - Droppable column for the kanban board
 *
 * Design spec: specs/design/pages/kanban-board.md
 * - Fixed width: 300px (min 280px, max 320px)
 * - Glass effect header with backdrop-blur
 * - Orange accent dot before title
 * - shadcn Badge for count
 * - Empty state with Lucide Inbox icon
 * - Drop zone with orange glow on drag-over
 */

import { useDroppable } from "@dnd-kit/core";
import { Inbox, XCircle } from "lucide-react";
import type { BoardColumn } from "./hooks";
import { TaskCard } from "./TaskCard";
import { Badge } from "@/components/ui/badge";

interface ColumnProps {
  column: BoardColumn;
  isOver?: boolean;
  isInvalid?: boolean;
  onTaskSelect?: (taskId: string) => void;
  hiddenTaskId?: string | null;
}

function InvalidDropIcon() {
  return (
    <div data-testid="invalid-drop-icon" className="flex items-center justify-center" style={{ color: "var(--status-error)" }}>
      <XCircle className="w-5 h-5" />
    </div>
  );
}

function EmptyState() {
  return (
    <div
      className="flex flex-col items-center justify-center gap-3 p-6 rounded-lg"
      style={{
        border: "2px dashed var(--border-subtle)",
      }}
    >
      <Inbox className="w-6 h-6" style={{ color: "var(--text-muted)" }} />
      <p className="text-sm" style={{ color: "var(--text-muted)" }}>
        No tasks
      </p>
    </div>
  );
}

export function Column({ column, isOver, isInvalid, onTaskSelect, hiddenTaskId }: ColumnProps) {
  const { setNodeRef } = useDroppable({ id: column.id });

  // Drop zone styles
  const getDropZoneStyles = (): React.CSSProperties => {
    if (isOver && isInvalid) {
      return {
        border: "2px dashed var(--status-error)",
        background: "rgba(239, 68, 68, 0.05)",
      };
    }
    if (isOver) {
      return {
        border: "2px dashed var(--accent-primary)",
        background: "var(--accent-muted)",
        boxShadow: "inset 0 0 20px rgba(255, 107, 53, 0.1)",
      };
    }
    return {
      border: "2px dashed transparent",
    };
  };

  return (
    <div
      data-testid={`column-${column.id}`}
      className="flex-shrink-0 flex flex-col h-full"
      style={{
        width: "300px",
        minWidth: "280px",
        maxWidth: "320px",
        scrollSnapAlign: "start",
      }}
    >
      {/* Glass effect header */}
      <div
        className="flex items-center gap-2 px-2 py-1.5 rounded-md mb-2 bg-bg-surface/85 backdrop-blur-md"
      >
        {/* Orange accent dot */}
        <span className="w-1.5 h-1.5 rounded-full flex-shrink-0 bg-accent-primary" />
        <h3 className="text-xs font-medium flex-1 text-text-primary tracking-tight m-0">
          {column.name}
        </h3>
        <Badge
          variant="secondary"
          className="text-[10px] px-1.5 py-0 bg-bg-elevated text-text-secondary"
        >
          {column.tasks.length}
        </Badge>
        {isOver && isInvalid && <InvalidDropIcon />}
      </div>

      {/* Drop zone with task list */}
      <div
        ref={setNodeRef}
        data-testid={`drop-zone-${column.id}`}
        className="flex-1 flex flex-col gap-3 p-3 rounded-lg transition-all bg-bg-surface/50"
        style={{
          minHeight: "100px",
          ...getDropZoneStyles(),
        }}
      >
        {column.tasks.length === 0 ? (
          <EmptyState />
        ) : (
          column.tasks.map((task) => (
            <TaskCard
              key={task.id}
              task={task}
              isHidden={task.id === hiddenTaskId}
              {...(onTaskSelect !== undefined && { onSelect: onTaskSelect })}
            />
          ))
        )}
      </div>
    </div>
  );
}
