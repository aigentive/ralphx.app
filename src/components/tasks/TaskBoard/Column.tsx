/**
 * Column - Droppable column for the kanban board
 *
 * Design: macOS Tahoe (2025)
 * - Clean, flat surfaces - no gradients or glows
 * - Subtle rounded rectangles for grouping
 * - Small, understated section headers
 * - Minimal visual noise
 */

import { useDroppable, useDndContext } from "@dnd-kit/core";
import { Inbox, XCircle, Loader2 } from "lucide-react";
import { useRef, useEffect, useState, useMemo, useCallback } from "react";
import type { WorkflowColumnResponse } from "@/lib/api/workflows";
import type { StateGroup } from "@/types/workflow";
import type { InternalStatus } from "@/types/status";
import type { Task } from "@/types/task";
import { TaskCard } from "./TaskCard";
import { ColumnGroup } from "./ColumnGroup";
import { Skeleton } from "@/components/ui/skeleton";
import { InlineTaskAdd } from "../InlineTaskAdd";
import {
  useInfiniteTasksQuery,
  flattenPages,
} from "@/hooks/useInfiniteTasksQuery";
import {
  getCollapsedGroups,
  saveCollapsedGroups,
  getGroupIcon,
} from "./Column.utils.tsx";

interface ColumnProps {
  column: WorkflowColumnResponse;
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
  /** Optional state groups for multi-state columns */
  groups?: StateGroup[];
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
    <div className="flex flex-col items-center justify-center gap-2 py-8 px-4">
      <Inbox className="w-8 h-8" style={{ color: "hsl(220 10% 35%)" }} />
      <p style={{ fontSize: "12px", color: "hsl(220 10% 45%)" }}>
        No tasks
      </p>
    </div>
  );
}

function TaskSkeleton() {
  return (
    <div
      className="rounded-lg p-2.5 space-y-2"
      style={{ background: "hsl(220 10% 12%)" }}
    >
      <Skeleton className="h-3 w-3/4 rounded" style={{ background: "hsl(220 10% 18%)" }} />
      <Skeleton className="h-2.5 w-1/2 rounded" style={{ background: "hsl(220 10% 16%)" }} />
    </div>
  );
}

export function Column({ column, projectId, showArchived, isOver, isInvalid, onTaskSelect, hiddenTaskId, searchTasks, matchCount, groups }: ColumnProps) {
  const { setNodeRef } = useDroppable({ id: column.id });
  const sentinelRef = useRef<HTMLDivElement>(null);
  const [isHovered, setIsHovered] = useState(false);
  const [isInlineAddExpanded, setIsInlineAddExpanded] = useState(false);
  const { active } = useDndContext();
  const isDragging = active !== null;

  // Track collapsed state for groups (persisted to localStorage)
  const [collapsedGroups, setCollapsedGroups] = useState<Set<string>>(() =>
    getCollapsedGroups(column.id)
  );

  // Handler for toggling group collapse state
  const handleToggleGroup = useCallback(
    (groupId: string) => {
      setCollapsedGroups((prev) => {
        const next = new Set(prev);
        if (next.has(groupId)) {
          next.delete(groupId);
        } else {
          next.add(groupId);
        }
        saveCollapsedGroups(column.id, next);
        return next;
      });
    },
    [column.id]
  );

  // Each column manages its own infinite query
  // Note: Backend only supports single status filter. For multi-state columns,
  // Gather all statuses from groups if they exist, otherwise use mapsTo
  const columnStatuses = useMemo(() => {
    if (groups && groups.length > 0) {
      // Collect all unique statuses from all groups
      const allStatuses = new Set<InternalStatus>();
      groups.forEach((group) => {
        group.statuses.forEach((status) => allStatuses.add(status));
      });
      return Array.from(allStatuses);
    }
    // No groups - use the primary mapsTo status
    return [column.mapsTo];
  }, [groups, column.mapsTo]);

  const {
    data,
    fetchNextPage,
    hasNextPage,
    isFetchingNextPage,
    isLoading,
  } = useInfiniteTasksQuery({
    projectId,
    statuses: columnStatuses,
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

  // Group tasks by their internal status when groups are defined
  const tasksByGroup = useMemo(() => {
    if (!groups || groups.length === 0) return null;

    const grouped = new Map<string, Task[]>();
    groups.forEach((group) => {
      grouped.set(group.id, []);
    });

    tasks.forEach((task) => {
      // Find which group this task belongs to based on its internalStatus
      const matchingGroup = groups.find((g) =>
        g.statuses.includes(task.internalStatus as InternalStatus)
      );
      if (matchingGroup) {
        const existing = grouped.get(matchingGroup.id) || [];
        grouped.set(matchingGroup.id, [...existing, task]);
      }
    });

    return grouped;
  }, [groups, tasks]);

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

  // Drop zone styles (macOS Tahoe - clean, flat)
  const getDropZoneStyles = (): React.CSSProperties => {
    if (isOver && isInvalid) {
      return {
        background: "hsla(0 70% 50% / 0.08)",
        borderRadius: "10px",
      };
    }
    if (isOver) {
      return {
        background: "hsla(220 60% 50% / 0.1)",
        borderRadius: "10px",
      };
    }
    return {
      background: "transparent",
      borderRadius: "10px",
    };
  };

  // Determine if this column should show InlineTaskAdd
  // Show when hovering OR when the inline add form is expanded (to preserve state)
  const showInlineAdd =
    (isHovered || isInlineAddExpanded) &&
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
      {/* Column header - macOS Tahoe: small, gray, understated like Finder section headers */}
      <div className="flex items-center gap-2 px-2 py-1.5 mb-1">
        {/* Column title - small caps style like Finder */}
        <h3
          className="flex-1 m-0 truncate"
          style={{
            fontSize: "11px",
            fontWeight: 600,
            color: "hsl(220 10% 50%)",
            textTransform: "uppercase",
            letterSpacing: "0.02em",
          }}
        >
          {column.name}
        </h3>

        {/* Count - simple, muted */}
        <span
          style={{
            fontSize: "11px",
            fontWeight: 500,
            color: "hsl(220 10% 40%)",
            fontVariantNumeric: "tabular-nums",
          }}
        >
          {tasks.length}
          {matchCount !== undefined && ` / ${matchCount}`}
        </span>

        {/* Invalid drop indicator */}
        {isOver && isInvalid && <InvalidDropIcon />}
      </div>

      {/* Drop zone with task list - clean, minimal */}
      <div
        ref={setNodeRef}
        data-testid={`drop-zone-${column.id}`}
        className="flex-1 flex flex-col gap-1.5 p-1 overflow-y-auto transition-colors duration-150"
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
        ) : groups && groups.length > 0 && tasksByGroup ? (
          /* Grouped rendering - render ColumnGroup for each group */
          <>
            {groups.map((group) => {
              const groupTasks = tasksByGroup.get(group.id) || [];
              // Only render groups that have tasks
              if (groupTasks.length === 0) return null;

              return (
                <ColumnGroup
                  key={group.id}
                  label={group.label}
                  count={groupTasks.length}
                  icon={getGroupIcon(group.icon)}
                  {...(group.accentColor && { accentColor: group.accentColor })}
                  collapsed={collapsedGroups.has(group.id)}
                  onToggle={() => handleToggleGroup(group.id)}
                >
                  {groupTasks.map((task) => (
                    <TaskCard
                      key={task.id}
                      task={task}
                      isHidden={task.id === hiddenTaskId}
                      {...(onTaskSelect !== undefined && { onSelect: onTaskSelect })}
                    />
                  ))}
                </ColumnGroup>
              );
            })}

            {/* Sentinel element for infinite scroll */}
            <div ref={sentinelRef} className="h-1" aria-hidden="true" />

            {/* Loading spinner when fetching next page - simple */}
            {isFetchingNextPage && (
              <div className="flex items-center justify-center py-3">
                <Loader2 className="w-4 h-4 animate-spin" style={{ color: "hsl(220 10% 40%)" }} />
              </div>
            )}
          </>
        ) : (
          /* Ungrouped rendering - render tasks directly */
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

            {/* Loading spinner when fetching next page - simple */}
            {isFetchingNextPage && (
              <div className="flex items-center justify-center py-3">
                <Loader2 className="w-4 h-4 animate-spin" style={{ color: "hsl(220 10% 40%)" }} />
              </div>
            )}
          </>
        )}

        {/* Inline task add - appears on hover (only in draft/backlog columns, not during drag) */}
        {showInlineAdd && (
          <InlineTaskAdd
            projectId={projectId}
            columnId={column.id}
            onExpandedChange={setIsInlineAddExpanded}
          />
        )}
      </div>
    </div>
  );
}
