/**
 * Column - Droppable column for the kanban board
 *
 * Design: macOS Tahoe Liquid Glass
 * - Frosted glass header with backdrop-blur
 * - Clean, flat surfaces
 * - Subtle accent dot
 * - Minimal drop zone styling
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
    <div className="flex flex-col items-center justify-center gap-2 py-8 px-4">
      <div
        className="w-10 h-10 rounded-xl flex items-center justify-center"
        style={{
          background: "rgba(255,255,255,0.03)",
          border: "1px solid rgba(255,255,255,0.06)",
        }}
      >
        <Inbox className="w-5 h-5 text-white/25" />
      </div>
      <p className="text-xs text-white/35">No tasks</p>
    </div>
  );
}

function TaskSkeleton() {
  return (
    <div className="rounded-lg p-2.5 space-y-2 bg-gradient-to-br from-white/[0.03] to-white/[0.01] border border-white/[0.06]">
      <Skeleton className="h-3 w-3/4 bg-white/5" />
      <Skeleton className="h-2.5 w-1/2 bg-white/5" />
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

  // Drop zone styles (macOS Tahoe - clean, subtle)
  const getDropZoneStyles = (): React.CSSProperties => {
    if (isOver && isInvalid) {
      return {
        border: "1px dashed rgba(239, 68, 68, 0.4)",
        background: "rgba(239, 68, 68, 0.03)",
      };
    }
    if (isOver) {
      return {
        border: "1px dashed rgba(255, 107, 53, 0.4)",
        background: "rgba(255, 107, 53, 0.03)",
      };
    }
    return {
      border: "1px solid transparent",
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
        width: "280px",
        minWidth: "260px",
        maxWidth: "300px",
        scrollSnapAlign: "start",
      }}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
    >
      {/* Liquid Glass header (macOS Tahoe) */}
      <div
        className="flex items-center gap-2.5 px-3 py-2 rounded-lg mb-2"
        style={{
          background: "rgba(255,255,255,0.03)",
          backdropFilter: "blur(20px)",
          WebkitBackdropFilter: "blur(20px)",
          border: "1px solid rgba(255,255,255,0.06)",
        }}
      >
        {/* Accent dot */}
        <span className="w-1.5 h-1.5 rounded-full flex-shrink-0 bg-[#ff6b35]" />
        <h3 className="text-[13px] font-medium flex-1 text-white/80 tracking-tight m-0">
          {column.name}
        </h3>
        <Badge
          variant="secondary"
          className="text-[10px] px-1.5 py-0.5 bg-white/[0.03] text-white/40 border-white/[0.06]"
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
        className="flex-1 flex flex-col gap-2 p-2 rounded-lg transition-all bg-white/[0.02] overflow-y-auto"
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
