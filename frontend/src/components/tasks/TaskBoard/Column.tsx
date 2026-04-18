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
  ContextMenu,
  ContextMenuContent,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { GroupContextMenuItems } from "@/components/tasks/GroupContextMenuItems";
import { resolveGroupCleanupParams, type GroupInfo } from "@/lib/task-actions";
import { useConfirmation } from "@/hooks/useConfirmation";
import { useTaskMutation } from "@/hooks/useTaskMutation";
import {
  useInfiniteTasksQuery,
  flattenPages,
} from "@/hooks/useInfiniteTasksQuery";
import {
  getCollapsedGroups,
  saveCollapsedGroups,
  getGroupIcon,
} from "./Column.utils.tsx";
import { CollapsedQuickAdd } from "./CollapsedQuickAdd";
import { useProjectStats } from "@/hooks/useProjectStats";
import { formatMinutesHuman } from "@/lib/formatters";

interface ColumnProps {
  column: WorkflowColumnResponse;
  projectId: string;
  showArchived: boolean;
  showMergeTasks: boolean;
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
  /** Hide right border for last column */
  isLast?: boolean;
  /** Optional ideation session ID to filter tasks by plan */
  ideationSessionId?: string | null | undefined;
  /** Optional execution plan ID to filter tasks (mutually exclusive with ideationSessionId) */
  executionPlanId?: string | null | undefined;
  /** Whether this column is collapsed */
  isCollapsed?: boolean;
  /** Callback to toggle collapse state */
  onToggleCollapse?: () => void;
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
      <Inbox className="w-8 h-8" style={{ color: "var(--text-muted)" }} />
      <p style={{ fontSize: "12px", color: "var(--text-muted)" }}>
        No tasks
      </p>
    </div>
  );
}

function TaskSkeleton() {
  return (
    <div
      className="rounded-lg p-2.5 space-y-2"
      style={{ background: "var(--bg-surface)" }}
    >
      <Skeleton className="h-3 w-3/4 rounded" style={{ background: "var(--bg-elevated)" }} />
      <Skeleton className="h-2.5 w-1/2 rounded" style={{ background: "var(--overlay-weak)" }} />
    </div>
  );
}

function isCompletedDoneGroup(group: StateGroup): boolean {
  return (
    group.id === "completed" ||
    group.statuses.some((status) => status === "approved" || status === "merged")
  );
}

/**
 * Format average minutes into a compact human-readable string.
 * Examples: "2m", "7h 30m", "3d"
 */
function formatAvgCycleTime(avgMinutes: number): string {
  if (avgMinutes < 60) {
    return `${Math.round(avgMinutes)}m`;
  }
  const hours = avgMinutes / 60;
  if (hours < 24) {
    return formatMinutesHuman(avgMinutes);
  }
  const days = hours / 24;
  const wholeDays = Math.floor(days);
  const remainingHours = Math.round((days - wholeDays) * 24);
  if (remainingHours > 0) return `${wholeDays}d ${remainingHours}h`;
  return `${wholeDays}d`;
}

export function Column({ column, projectId, showArchived, showMergeTasks, isOver, isInvalid, onTaskSelect, hiddenTaskId, searchTasks, matchCount, groups, isLast = false, ideationSessionId, executionPlanId, isCollapsed = false, onToggleCollapse }: ColumnProps) {
  const { setNodeRef } = useDroppable({ id: column.id });
  const sentinelRef = useRef<HTMLDivElement>(null);
  const { active } = useDndContext();
  const isDragging = active !== null;

  // Fetch project stats to show avg cycle time per column phase
  const { data: projectStats } = useProjectStats(projectId);

  // Find avg cycle time for the phase matching this column's mapsTo status
  const avgCycleTime = useMemo(() => {
    if (!projectStats) return null;
    const phase = projectStats.cycleTimeBreakdown.find(
      (p) => p.phase === column.mapsTo
    );
    if (!phase || phase.sampleSize === 0) return null;
    return phase.avgMinutes;
  }, [projectStats, column.mapsTo]);

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
    ideationSessionId,
    executionPlanId,
  });

  // Use search tasks if provided (search mode), otherwise use fetched tasks
  const tasks = useMemo(() => {
    let result: Task[];
    if (searchTasks) {
      // In search mode, use provided search results
      result = searchTasks.sort((a, b) => a.priority - b.priority);
    } else {
      // In normal mode, flatten paginated data
      const flattened = flattenPages(data);
      result = flattened.sort((a, b) => a.priority - b.priority);
    }
    // Filter out merge tasks when toggle is off
    if (!showMergeTasks) {
      result = result.filter((t) => t.category !== "plan_merge");
    }
    return result;
  }, [data, searchTasks, showMergeTasks]);

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

  // UX guard:
  // If Done has only completed tasks visible, force that subgroup open even if it was persisted collapsed.
  useEffect(() => {
    if (!groups || !tasksByGroup || column.mapsTo !== "approved") return;

    const nonEmptyGroups = groups.filter((group) => {
      const groupTasks = tasksByGroup.get(group.id) || [];
      return groupTasks.length > 0;
    });

    if (nonEmptyGroups.length !== 1) return;

    const onlyVisibleGroup = nonEmptyGroups[0];
    if (!onlyVisibleGroup || !isCompletedDoneGroup(onlyVisibleGroup)) return;

    setCollapsedGroups((prev) => {
      if (!prev.has(onlyVisibleGroup.id)) return prev;
      const next = new Set(prev);
      next.delete(onlyVisibleGroup.id);
      saveCollapsedGroups(column.id, next);
      return next;
    });
  }, [groups, tasksByGroup, column.mapsTo, column.id]);

  // Infinite scroll with IntersectionObserver
  // Re-run when isCollapsed changes so observer reconnects after expand
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
  }, [hasNextPage, isFetchingNextPage, fetchNextPage, isCollapsed]);

  // Drop zone styles (macOS Tahoe - clean, flat)
  const getDropZoneStyles = (): React.CSSProperties => {
    if (isOver && isInvalid) {
      return {
        background: "var(--status-error-muted)",
        borderRadius: "10px",
      };
    }
    if (isOver) {
      return {
        background: "var(--status-info-muted)",
        borderRadius: "10px",
      };
    }
    return {
      background: "transparent",
      borderRadius: "10px",
    };
  };

  // Confirmation dialog for column context menu
  const { confirm, confirmationDialogProps, ConfirmationDialog } = useConfirmation();
  const { cancelTasksInGroupMutation, pauseTasksInGroupMutation, resumeTasksInGroupMutation, archiveTasksInGroupMutation } = useTaskMutation(projectId);

  // Handler for "Cancel all" group action
  // Note: For multi-state columns, this only cancels tasks matching the primary status (column.mapsTo)
  const handleCancelAll = useCallback(() => {
    const { groupKind, groupId } = resolveGroupCleanupParams("column", column.mapsTo);
    cancelTasksInGroupMutation.mutate({ groupKind, groupId, projectId });
  }, [column.mapsTo, projectId, cancelTasksInGroupMutation]);

  const handlePauseAll = useCallback(() => {
    const { groupKind, groupId } = resolveGroupCleanupParams("column", column.mapsTo);
    pauseTasksInGroupMutation.mutate({ groupKind, groupId, projectId });
  }, [column.mapsTo, projectId, pauseTasksInGroupMutation]);

  const handleResumeAll = useCallback(() => {
    const { groupKind, groupId } = resolveGroupCleanupParams("column", column.mapsTo);
    resumeTasksInGroupMutation.mutate({ groupKind, groupId, projectId });
  }, [column.mapsTo, projectId, resumeTasksInGroupMutation]);

  const handleArchiveAll = useCallback(() => {
    const { groupKind, groupId } = resolveGroupCleanupParams("column", column.mapsTo);
    archiveTasksInGroupMutation.mutate({ groupKind, groupId, projectId });
  }, [column.mapsTo, projectId, archiveTasksInGroupMutation]);

  // Group info for task-level context menus (shows column group actions)
  const columnGroupInfo: GroupInfo = useMemo(() => ({
    groupLabel: column.name,
    groupKind: "column" as const,
    taskCount: tasks.length,
    groupId: column.mapsTo,
    projectId,
    onCancelAll: handleCancelAll,
    onPauseAll: handlePauseAll,
    onResumeAll: handleResumeAll,
    onArchiveAll: handleArchiveAll,
  }), [column.name, column.mapsTo, tasks.length, projectId, handleCancelAll, handlePauseAll, handleResumeAll, handleArchiveAll]);

  // Determine if this column should show InlineTaskAdd
  // Always visible in draft/backlog columns (not during drag)
  const showInlineAdd =
    !isDragging &&
    (column.id === "draft" || column.id === "backlog");

  // Keyboard handler for collapsed column (Enter/Space to expand)
  const handleCollapsedKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        onToggleCollapse?.();
      }
    },
    [onToggleCollapse]
  );

  // Collapsed rendering: 44px strip with vertical title, count badge, click-to-expand
  if (isCollapsed) {
    return (
      <div
        ref={setNodeRef}
        data-testid={`column-${column.id}`}
        role="button"
        tabIndex={0}
        aria-expanded={false}
        aria-label={`${column.name} column, ${tasks.length} tasks. Click to expand`}
        onClick={onToggleCollapse}
        onKeyDown={handleCollapsedKeyDown}
        className="flex-shrink-0 flex flex-col items-center h-full cursor-pointer"
        style={{
          width: "44px",
          minWidth: "44px",
          maxWidth: "44px",
          paddingLeft: "6px",
          paddingRight: "6px",
          paddingTop: "8px",
          scrollSnapAlign: "start",
          transition: "width 200ms ease, min-width 200ms ease, max-width 200ms ease",
          ...(!isLast && { borderRight: "1px solid var(--border-subtle)" }),
          // Drop zone highlight when dragging over collapsed column
          ...(isOver && !isInvalid && {
            background: "var(--status-info-muted)",
            borderRadius: "10px",
          }),
          ...(isOver && isInvalid && {
            background: "var(--status-error-muted)",
            borderRadius: "10px",
          }),
        }}
      >
        {/* Vertical column title - top-to-bottom reading */}
        <span
          className="select-none"
          style={{
            writingMode: "vertical-rl",
            transform: "rotate(180deg)",
            fontSize: "11px",
            fontWeight: 600,
            color: "var(--text-secondary)",
            textTransform: "uppercase",
            letterSpacing: "0.02em",
            whiteSpace: "nowrap",
            overflow: "hidden",
            textOverflow: "ellipsis",
            maxHeight: "calc(100% - 40px)",
          }}
        >
          {column.name}
        </span>

        {/* Task count badge */}
        <span
          className="mt-2"
          style={{
            fontSize: "10px",
            fontWeight: 500,
            color: "var(--text-muted)",
            fontVariantNumeric: "tabular-nums",
          }}
        >
          {tasks.length}
        </span>

        {/* Quick-add button for draft/backlog columns (hidden during drag) */}
        {!isDragging && (column.id === "draft" || column.id === "backlog") && (
          <CollapsedQuickAdd projectId={projectId} columnId={column.id} />
        )}
      </div>
    );
  }

  return (
    <div
      data-testid={`column-${column.id}`}
      className="flex-shrink-0 flex flex-col h-full"
      style={{
        width: "280px",
        minWidth: "260px",
        maxWidth: "300px",
        scrollSnapAlign: "start",
        paddingLeft: "12px",
        paddingRight: "12px",
        transition: "width 200ms ease, min-width 200ms ease, max-width 200ms ease",
        ...(!isLast && { borderRight: "1px solid var(--border-subtle)" }),
      }}
    >
      {/* Column header - macOS Tahoe: small, gray, understated like Finder section headers */}
      <div className="flex items-center gap-2 px-2 py-1.5 mb-1">
        {/* Column title - small caps style like Finder */}
        <h3
          className="flex-1 m-0 truncate"
          style={{
            fontSize: "11px",
            fontWeight: 600,
            color: "var(--text-secondary)",
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
            color: "var(--text-muted)",
            fontVariantNumeric: "tabular-nums",
          }}
        >
          {tasks.length}
          {matchCount !== undefined && ` / ${matchCount}`}
        </span>

        {/* Avg cycle time badge - shown when backend has enough data */}
        {avgCycleTime !== null && (
          <span
            title={`Avg time in ${column.name}: ${formatAvgCycleTime(avgCycleTime)}`}
            style={{
              fontSize: "10px",
              fontWeight: 500,
              color: "var(--text-muted)",
              fontVariantNumeric: "tabular-nums",
              letterSpacing: "0.01em",
            }}
          >
            {formatAvgCycleTime(avgCycleTime)}
          </span>
        )}

        {/* Invalid drop indicator */}
        {isOver && isInvalid && <InvalidDropIcon />}
      </div>

      {/* Drop zone with task list - clean, minimal */}
      <ContextMenu>
        <ContextMenuTrigger asChild>
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
                          groupInfo={columnGroupInfo}
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
                    <Loader2 className="w-4 h-4 animate-spin" style={{ color: "var(--text-muted)" }} />
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
                    groupInfo={columnGroupInfo}
                    {...(onTaskSelect !== undefined && { onSelect: onTaskSelect })}
                  />
                ))}

                {/* Sentinel element for infinite scroll */}
                <div ref={sentinelRef} className="h-1" aria-hidden="true" />

                {/* Loading spinner when fetching next page - simple */}
                {isFetchingNextPage && (
                  <div className="flex items-center justify-center py-3">
                    <Loader2 className="w-4 h-4 animate-spin" style={{ color: "var(--text-muted)" }} />
                  </div>
                )}
              </>
            )}

            {/* Inline task add - always visible in draft/backlog columns (hidden during drag) */}
            {showInlineAdd && (
              <InlineTaskAdd
                projectId={projectId}
                columnId={column.id}
              />
            )}
          </div>
        </ContextMenuTrigger>
        <ContextMenuContent>
          <GroupContextMenuItems
            groupLabel={column.name}
            groupKind="column"
            taskCount={tasks.length}
            projectId={projectId}
            groupId={column.mapsTo}
            onCancelAll={handleCancelAll}
            onPauseAll={handlePauseAll}
            onResumeAll={handleResumeAll}
            onArchiveAll={handleArchiveAll}
            confirm={confirm}
          />
        </ContextMenuContent>
        <ConfirmationDialog {...confirmationDialogProps} />
      </ContextMenu>
    </div>
  );
}
