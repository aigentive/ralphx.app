/**
 * TaskBoard - Main kanban board component with drag-drop support
 *
 * Design spec: specs/design/pages/kanban-board.md
 * - Radial gradient background for warmth
 * - Horizontal scroll with CSS scroll-snap
 * - Fade edges at overflow boundaries
 * - 24px (--space-6) gutters between columns
 */

import { useState, useCallback, useEffect, useMemo } from "react";
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
import { listen } from "@tauri-apps/api/event";
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
import type { Task, TaskListResponse } from "@/types/task";

export interface TaskBoardProps {
  projectId: string;
  workflowId: string;
}

export function TaskBoard({ projectId, workflowId }: TaskBoardProps) {
  const queryClient = useQueryClient();
  const { columns, onDragEnd, isLoading, error } = useTaskBoard(
    projectId,
    workflowId
  );
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
    const unlisteners: Promise<() => void>[] = [];

    // Listen for task:archived events
    const archivedListener = listen<{ task_id: string; project_id: string }>(
      'task:archived',
      (event) => {
        // Only invalidate if the event is for the current project
        if (event.payload.project_id === projectId) {
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
    unlisteners.push(archivedListener);

    // Listen for task:restored events
    const restoredListener = listen<{ task_id: string; project_id: string }>(
      'task:restored',
      (event) => {
        if (event.payload.project_id === projectId) {
          queryClient.invalidateQueries({
            queryKey: infiniteTaskKeys.all,
          });
          queryClient.invalidateQueries({
            queryKey: ['archived-count', projectId],
          });
        }
      }
    );
    unlisteners.push(restoredListener);

    // Listen for task:deleted events (permanent delete)
    const deletedListener = listen<{ task_id: string; project_id: string }>(
      'task:deleted',
      (event) => {
        if (event.payload.project_id === projectId) {
          queryClient.invalidateQueries({
            queryKey: infiniteTaskKeys.all,
          });
          queryClient.invalidateQueries({
            queryKey: ['archived-count', projectId],
          });
        }
      }
    );
    unlisteners.push(deletedListener);

    return () => {
      unlisteners.forEach((unlisten) => {
        unlisten.then((fn) => fn());
      });
    };
  }, [projectId, queryClient]);

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
      // Fetch the task from the API instead of trying to find it in columns
      // (columns are workflow definitions without task data)
      api.tasks.get(taskId).then((task) => {
        openModal("task-detail", { task });
      }).catch((err) => {
        console.error("Failed to fetch task:", err);
      });
    },
    [openModal]
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
        status: col.mapsTo,
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
        {/* Header with Show Archived toggle and Search Bar */}
        <div className="px-6 py-3 border-b border-border/40 space-y-3">
          {/* Search Bar (when search is open) */}
          {searchOpen && (
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
          )}

          {/* Show Archived toggle (only visible when there are archived tasks) */}
          {archivedCount > 0 && (
            <Toggle
              pressed={showArchived}
              onPressedChange={setShowArchived}
              aria-label="Toggle show archived tasks"
              className="gap-2 data-[state=on]:bg-accent/10 data-[state=on]:text-accent"
            >
              <Archive className="h-4 w-4" />
              <span className="text-sm font-medium">
                Show archived ({archivedCount})
              </span>
            </Toggle>
          )}
        </div>

        {/* TaskBoard container with radial gradient and scroll-snap */}
        <div
          data-testid="task-board"
          className="task-board relative flex items-stretch gap-3 py-6 overflow-x-auto flex-1"
          style={{
            background:
              "radial-gradient(ellipse at top, rgba(255,107,53,0.03) 0%, var(--bg-base) 50%)",
            scrollSnapType: "x proximity",
            scrollPaddingLeft: "16px",
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
                    onTaskSelect={handleTaskSelect}
                    hiddenTaskId={movingTaskId}
                    searchTasks={searchTasks}
                    matchCount={matchCount}
                  />
                );
              })}
            </>
          )}
        </div>
        <DragOverlay dropAnimation={null}>
          {activeTask && <TaskCard task={activeTask} isDragging />}
        </DragOverlay>
      </div>
    </DndContext>
  );
}
