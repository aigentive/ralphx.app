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

export function Column({ column, isOver, isInvalid, onTaskSelect }: ColumnProps) {
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
      className="flex-shrink-0 flex flex-col"
      style={{
        width: "300px",
        minWidth: "280px",
        maxWidth: "320px",
        scrollSnapAlign: "start",
      }}
    >
      {/* Glass effect header */}
      <div
        className="flex items-center gap-2 px-3 py-2 rounded-lg mb-3"
        style={{
          background: "rgba(26, 26, 26, 0.85)",
          backdropFilter: "blur(12px)",
          WebkitBackdropFilter: "blur(12px)",
        }}
      >
        {/* Orange accent dot */}
        <span
          className="w-1.5 h-1.5 rounded-full flex-shrink-0"
          style={{ backgroundColor: "var(--accent-primary)" }}
        />
        <h3
          className="text-sm font-semibold flex-1"
          style={{
            color: "var(--text-primary)",
            letterSpacing: "var(--tracking-tight)",
          }}
        >
          {column.name}
        </h3>
        <Badge variant="secondary">{column.tasks.length}</Badge>
        {isOver && isInvalid && <InvalidDropIcon />}
      </div>

      {/* Drop zone with task list */}
      <div
        ref={setNodeRef}
        data-testid={`drop-zone-${column.id}`}
        className="flex-1 p-2 space-y-2 rounded-lg transition-all"
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
              {...(onTaskSelect !== undefined && { onSelect: onTaskSelect })}
            />
          ))
        )}
      </div>
    </div>
  );
}
