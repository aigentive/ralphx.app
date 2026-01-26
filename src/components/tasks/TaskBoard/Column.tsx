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

import { useDroppable, useDndContext } from "@dnd-kit/core";
import { Inbox, XCircle, Loader2 } from "lucide-react";
import { useRef, useEffect, useState, useMemo } from "react";
import type { WorkflowColumn } from "@/types/workflow";
import { TaskCard } from "./TaskCard";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { InlineTaskAdd } from "../InlineTaskAdd";
import {
  useInfiniteTasksQuery,
  flattenPages,
} from "@/hooks/useInfiniteTasksQuery";

interface ColumnProps {
  column: WorkflowColumn;
  projectId: string;
  showArchived: boolean;
  isOver?: boolean;
  isInvalid?: boolean;
  onTaskSelect?: (taskId: string) => void;
  hiddenTaskId?: string | null;
  /** Optional search results to display instead of fetched tasks */
  searchTasks?: Task[] | undefined;
  /** Optional match count badge for search mode */
  matchCount?: number | undefined;
}

import type { Task } from "@/types/task";

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

function TaskSkeleton() {
  return (
    <div className="rounded-lg p-3 space-y-2" style={{ background: "var(--bg-elevated)" }}>
      <Skeleton className="h-4 w-3/4" />
      <Skeleton className="h-3 w-1/2" />
    </div>
  );
}

export function Column({ column, projectId, showArchived, isOver, isInvalid, onTaskSelect, hiddenTaskId, searchTasks, matchCount }: ColumnProps) {
  const { setNodeRef } = useDroppable({ id: column.id });
  const sentinelRef = useRef<HTMLDivElement>(null);
  const [isHovered, setIsHovered] = useState(false);
  const { active } = useDndContext();
  const isDragging = active !== null;

  // Each column manages its own infinite query
  const {
    data,
    fetchNextPage,
    hasNextPage,
    isFetchingNextPage,
    isLoading,
  } = useInfiniteTasksQuery({
    projectId,
    status: column.mapsTo,
    includeArchived: showArchived,
  });

  // Use search tasks if provided (search mode), otherwise use fetched tasks
  const tasks = useMemo(() => {
    if (searchTasks) {
      // In search mode, use provided search results
      return searchTasks.sort((a, b) => a.priority - b.priority);
    }
    // In normal mode, flatten paginated data
    const flattened = flattenPages(data);
    return flattened.sort((a, b) => a.priority - b.priority);
  }, [data, searchTasks]);

  // Infinite scroll with IntersectionObserver
  useEffect(() => {
    const sentinel = sentinelRef.current;
    if (!sentinel) return;

    const observer = new IntersectionObserver(
      (entries) => {
        const entry = entries[0];
        // Trigger fetchNextPage when sentinel is visible AND there's more data AND not already fetching
        if (
          entry &&
          entry.isIntersecting &&
          hasNextPage &&
          !isFetchingNextPage
        ) {
          fetchNextPage();
        }
      },
      {
        root: null, // viewport
        rootMargin: "100px", // Load slightly before reaching the bottom
        threshold: 0.1,
      }
    );

    observer.observe(sentinel);

    return () => {
      observer.disconnect();
    };
  }, [hasNextPage, isFetchingNextPage, fetchNextPage]);

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

  // Determine if this column should show InlineTaskAdd
  const showInlineAdd =
    isHovered &&
    !isDragging &&
    (column.id === "draft" || column.id === "backlog");

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
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
    >
      {/* Glass effect header */}
      <div
        className="flex items-center gap-2 px-2.5 py-1.5 rounded-md mb-2 bg-bg-surface/85 backdrop-blur-md"
      >
        {/* Orange accent dot */}
        <span className="w-1.5 h-1.5 rounded-full flex-shrink-0 bg-accent-primary" />
        <h3 className="text-[0.92rem] font-medium flex-1 text-text-primary tracking-tight m-0">
          {column.name}
        </h3>
        <Badge
          variant="secondary"
          className="text-[10px] px-1.5 py-0 bg-bg-elevated text-text-secondary"
        >
          {tasks.length}
          {matchCount !== undefined && ` (${matchCount})`}
        </Badge>
        {isOver && isInvalid && <InvalidDropIcon />}
      </div>

      {/* Drop zone with task list - scrollable */}
      <div
        ref={setNodeRef}
        data-testid={`drop-zone-${column.id}`}
        className="flex-1 flex flex-col gap-3 p-3 rounded-lg transition-all bg-bg-surface/50 overflow-y-auto"
        style={getDropZoneStyles()}
      >
        {/* Show skeleton cards during initial load */}
        {isLoading ? (
          <>
            <TaskSkeleton />
            <TaskSkeleton />
            <TaskSkeleton />
          </>
        ) : tasks.length === 0 ? (
          <EmptyState />
        ) : (
          <>
            {tasks.map((task) => (
              <TaskCard
                key={task.id}
                task={task}
                isHidden={task.id === hiddenTaskId}
                {...(onTaskSelect !== undefined && { onSelect: onTaskSelect })}
              />
            ))}

            {/* Sentinel element for infinite scroll */}
            <div ref={sentinelRef} className="h-1" aria-hidden="true" />

            {/* Loading spinner when fetching next page */}
            {isFetchingNextPage && (
              <div className="flex items-center justify-center py-3">
                <Loader2 className="w-5 h-5 animate-spin" style={{ color: "var(--accent-primary)" }} />
              </div>
            )}
          </>
        )}

        {/* Inline task add - appears on hover (only in draft/backlog columns, not during drag) */}
        {showInlineAdd && (
          <InlineTaskAdd projectId={projectId} columnId={column.id} />
        )}
      </div>
    </div>
  );
}
