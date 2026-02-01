/**
 * TaskBoard - Main kanban board component with drag-drop support
 *
 * Design: macOS Tahoe "Liquid Glass" (2024 WWDC aesthetic)
 * - Multi-layer translucent depth with precise backdrop-blur
 * - Warm ambient luminosity from accent glow
 * - Horizontal scroll with momentum and snap
 * - Premium Apple-grade typography and spacing
 */

import { useState, useEffect, useMemo } from "react";
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
import { useQuery, useQueryClient, type InfiniteData } from "@tanstack/react-query";
import { useEventBus } from "@/providers/EventProvider";
import { useTaskBoard } from "./hooks";
import { TaskBoardSkeleton } from "./TaskBoardSkeleton";
import { Column } from "./Column";
import { TaskCard } from "./TaskCard";
import { useUiStore } from "@/stores/uiStore";
import { Toggle } from "@/components/ui/toggle";
import { Archive } from "lucide-react";
import { api } from "@/lib/tauri";
import { useTaskSearch } from "@/hooks/useTaskSearch";
import { TaskSearchBar } from "../TaskSearchBar";
import { EmptySearchState } from "../EmptySearchState";
import { infiniteTaskKeys } from "@/hooks/useInfiniteTasksQuery";
import { defaultWorkflow, type WorkflowColumn } from "@/types/workflow";
import type { Task, TaskListResponse, InternalStatus } from "@/types/task";

/**
 * Get all statuses for a column from its groups, or fallback to mapsTo
 */
function getColumnStatuses(col: WorkflowColumn): InternalStatus[] {
  if (col.groups && col.groups.length > 0) {
    const allStatuses = new Set<InternalStatus>();
    col.groups.forEach((group) => {
      group.statuses.forEach((status) => allStatuses.add(status));
    });
    return Array.from(allStatuses);
  }
  return [col.mapsTo];
}

export interface TaskBoardProps {
  projectId: string;
}

export function TaskBoard({ projectId }: TaskBoardProps) {
  const queryClient = useQueryClient();
  const eventBus = useEventBus();
  const { columns, onDragEnd, isLoading, error } = useTaskBoard(projectId);
  const [activeTask, setActiveTask] = useState<Task | null>(null);
  const [overColumnId, setOverColumnId] = useState<string | null>(null);
  const [movingTaskId, setMovingTaskId] = useState<string | null>(null);
  const [searchOpen, setSearchOpen] = useState(false);
  const openModal = useUiStore((s) => s.openModal);
  const showArchived = useUiStore((s) => s.showArchived);
  const setShowArchived = useUiStore((s) => s.setShowArchived);
  const boardSearchQuery = useUiStore((s) => s.boardSearchQuery);
  const setBoardSearchQuery = useUiStore((s) => s.setBoardSearchQuery);

  // Fetch archived count to show/hide the toggle
  const { data: archivedCount = 0 } = useQuery({
    queryKey: ["archived-count", projectId],
    queryFn: () => api.tasks.getArchivedCount(projectId),
  });

  // Search functionality
  const {
    data: searchResults = [],
    isLoading: isSearchLoading,
  } = useTaskSearch({
    projectId,
    query: boardSearchQuery,
    includeArchived: showArchived,
  });

  // Check if search is active
  const isSearchActive = searchOpen && boardSearchQuery && boardSearchQuery.length >= 2;

  // When search is active, group search results by column
  const searchTasksByColumn = useMemo(() => {
    if (!isSearchActive) {
      return new Map<string, Task[]>();
    }

    // Group search results by their internalStatus to distribute to columns
    const tasksByStatus = new Map<string, Task[]>();
    searchResults.forEach((task) => {
      const existing = tasksByStatus.get(task.internalStatus) || [];
      tasksByStatus.set(task.internalStatus, [...existing, task]);
    });

    // Map to column IDs
    const tasksByColumn = new Map<string, Task[]>();
    columns.forEach((column) => {
      const tasks = tasksByStatus.get(column.mapsTo) || [];
      if (tasks.length > 0) {
        tasksByColumn.set(column.id, tasks);
      }
    });

    return tasksByColumn;
  }, [columns, isSearchActive, searchResults]);

  // During search, filter columns to only show those with matches
  const displayColumns = useMemo(() => {
    if (!isSearchActive) {
      return columns;
    }
    // Only show columns with search results
    return columns.filter((col) => searchTasksByColumn.has(col.id));
  }, [columns, isSearchActive, searchTasksByColumn]);

  // Clear movingTaskId after React has re-rendered with new position
  useEffect(() => {
    if (!movingTaskId) return;
    const id = requestAnimationFrame(() => {
      setMovingTaskId(null);
    });
    return () => cancelAnimationFrame(id);
  }, [movingTaskId]);

  // Keyboard shortcuts: Cmd+N for new task, Cmd+F for search, Escape to close search
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Guard: ignore if user is typing in an input/textarea
      const activeElement = document.activeElement;
      if (
        activeElement instanceof HTMLInputElement ||
        activeElement instanceof HTMLTextAreaElement ||
        activeElement?.hasAttribute('contenteditable')
      ) {
        return;
      }

      // Cmd+N / Ctrl+N: Open task creation modal
      if ((e.metaKey || e.ctrlKey) && e.key === 'n') {
        e.preventDefault();
        openModal('task-create', { projectId });
      }

      // Cmd+F / Ctrl+F: Open search
      if ((e.metaKey || e.ctrlKey) && e.key === 'f') {
        e.preventDefault();
        setSearchOpen(true);
      }

      // Escape: Close search
      if (e.key === 'Escape' && searchOpen) {
        setSearchOpen(false);
        setBoardSearchQuery(null);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [searchOpen, setBoardSearchQuery, openModal, projectId]);

  // Listen for archive/restore/delete events for real-time updates
  useEffect(() => {
    const unsubscribers: (() => void)[] = [];

    // Listen for task:archived events
    const unsubArchived = eventBus.subscribe<{ task_id: string; project_id: string }>(
      'task:archived',
      (payload) => {
        // Only invalidate if the event is for the current project
        if (payload.project_id === projectId) {
          // Invalidate infinite task queries for all columns
          queryClient.invalidateQueries({
            queryKey: infiniteTaskKeys.all,
          });
          // Invalidate archived count
          queryClient.invalidateQueries({
            queryKey: ['archived-count', projectId],
          });
        }
      }
    );
    unsubscribers.push(unsubArchived);

    // Listen for task:restored events
    const unsubRestored = eventBus.subscribe<{ task_id: string; project_id: string }>(
      'task:restored',
      (payload) => {
        if (payload.project_id === projectId) {
          queryClient.invalidateQueries({
            queryKey: infiniteTaskKeys.all,
          });
          queryClient.invalidateQueries({
            queryKey: ['archived-count', projectId],
          });
        }
      }
    );
    unsubscribers.push(unsubRestored);

    // Listen for task:deleted events (permanent delete)
    const unsubDeleted = eventBus.subscribe<{ task_id: string; project_id: string }>(
      'task:deleted',
      (payload) => {
        if (payload.project_id === projectId) {
          queryClient.invalidateQueries({
            queryKey: infiniteTaskKeys.all,
          });
          queryClient.invalidateQueries({
            queryKey: ['archived-count', projectId],
          });
        }
      }
    );
    unsubscribers.push(unsubDeleted);

    return () => {
      unsubscribers.forEach((unsub) => unsub());
    };
  }, [projectId, queryClient, eventBus]);

  // Distance-based activation - drag starts after moving 8px
  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: {
        distance: 8,
      },
    })
  );

  // Task selection is now handled by TaskCard directly via setSelectedTaskId
  // which shows the TaskDetailOverlay in the split layout

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
        Error: {error.message || String(error)}
      </div>
    );
  }

  const handleDragStart = (event: DragStartEvent) => {
    const taskId = String(event.active.id);

    // Search for the task in the query cache (similar to onDragEnd in hooks.ts)
    let foundTask: Task | null = null;
    for (const col of columns) {
      const key = infiniteTaskKeys.list({
        projectId,
        statuses: getColumnStatuses(col),
        includeArchived: showArchived,
      });
      const data = queryClient.getQueryData<InfiniteData<TaskListResponse>>(key);
      if (data?.pages) {
        for (const page of data.pages) {
          const task = page.tasks.find((t: Task) => t.id === taskId);
          if (task) {
            foundTask = task;
            break;
          }
        }
      }
      if (foundTask) break;
    }

    setActiveTask(foundTask);
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

  // Check if we should show empty state
  const hasSearchResults = searchResults.length > 0;
  const showEmptyState = isSearchActive && !hasSearchResults && !isSearchLoading;

  return (
    <DndContext
      sensors={sensors}
      onDragStart={handleDragStart}
      onDragOver={handleDragOver}
      onDragEnd={handleDragEnd}
      onDragCancel={handleDragCancel}
    >
      {/* Container for the entire board including header */}
      <div className="flex flex-col h-full">
        {/* Header bar - macOS Tahoe: minimal, flat */}
        {(searchOpen || archivedCount > 0) && (
          <div className="px-4 py-2 flex items-center gap-3">
            {/* Search Bar (when search is open) */}
            {searchOpen && (
              <div className="flex-1 max-w-md">
                <TaskSearchBar
                  value={boardSearchQuery || ''}
                  onChange={setBoardSearchQuery}
                  onClose={() => {
                    setSearchOpen(false);
                    setBoardSearchQuery(null);
                  }}
                  resultCount={searchResults.length}
                  isSearching={isSearchLoading}
                />
              </div>
            )}

            {/* Show Archived toggle - simple Tahoe style */}
            {archivedCount > 0 && (
              <Toggle
                pressed={showArchived}
                onPressedChange={setShowArchived}
                aria-label="Toggle show archived tasks"
                className="gap-1.5 h-7 px-2.5 rounded-md text-xs font-medium transition-colors"
                style={{
                  background: showArchived
                    ? "hsla(220 60% 50% / 0.2)"
                    : "transparent",
                  color: showArchived
                    ? "hsl(220 80% 70%)"
                    : "hsl(220 10% 55%)",
                }}
              >
                <Archive className="h-3.5 w-3.5" />
                <span>Archived ({archivedCount})</span>
              </Toggle>
            )}
          </div>
        )}

        {/* TaskBoard container - macOS Tahoe: clean, flat, minimal */}
        <div
          data-testid="task-board"
          className="task-board relative flex items-stretch gap-3 py-4 overflow-x-auto flex-1"
          style={{
            /* Solid dark background with subtle cool tint - like Tahoe Finder */
            background: "hsl(220 10% 8%)",
            scrollSnapType: "x proximity",
            scrollPaddingLeft: "16px",
            scrollPaddingRight: "16px",
            WebkitOverflowScrolling: "touch",
          }}
        >
          {/* Show empty search state when search has no results */}
          {showEmptyState ? (
            <div className="flex-1 flex items-center justify-center">
              <EmptySearchState
                searchQuery={boardSearchQuery || ''}
                onCreateTask={() => {
                  openModal('task-create', {
                    projectId,
                    defaultTitle: boardSearchQuery || undefined,
                  });
                }}
                onClearSearch={() => {
                  setBoardSearchQuery(null);
                }}
                showArchived={showArchived}
              />
            </div>
          ) : (
            <>
              {/* Left spacer for scroll padding */}
              <div className="w-4 flex-shrink-0" aria-hidden="true" />

              {displayColumns.map((column) => {
                // In search mode, provide search results to column
                const searchTasks = isSearchActive
                  ? searchTasksByColumn.get(column.id)
                  : undefined;
                const matchCount = isSearchActive
                  ? searchTasks?.length
                  : undefined;

                // Look up groups from the default workflow for this column
                const workflowColumn = defaultWorkflow.columns.find(
                  (c) => c.id === column.id
                );
                const groups = workflowColumn?.groups;

                return (
                  <Column
                    key={column.id}
                    column={column}
                    projectId={projectId}
                    showArchived={showArchived}
                    isOver={overColumnId === column.id}
                    isInvalid={
                      overColumnId === column.id && lockedColumns.includes(column.id)
                    }
                    hiddenTaskId={movingTaskId}
                    searchTasks={searchTasks}
                    matchCount={matchCount}
                    {...(groups && { groups })}
                  />
                );
              })}
            </>
          )}
        </div>
        {/* Drag overlay with premium floating appearance */}
        <DragOverlay
          dropAnimation={{
            duration: 200,
            easing: "cubic-bezier(0.34, 1.56, 0.64, 1)",
          }}
        >
          {activeTask && <TaskCard task={activeTask} isDragging />}
        </DragOverlay>
      </div>
    </DndContext>
  );
}
