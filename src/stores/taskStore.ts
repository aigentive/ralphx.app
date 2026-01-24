/**
 * Task store using Zustand with immer middleware
 *
 * Manages task state for the frontend. Tasks are stored in a Record
 * keyed by task ID for O(1) lookup. The store is synchronized with
 * backend state via Tauri events.
 */

import { create } from "zustand";
import { immer } from "zustand/middleware/immer";
import type { Task, InternalStatus } from "@/types/task";

// ============================================================================
// State Interface
// ============================================================================

interface TaskState {
  /** Tasks indexed by ID for O(1) lookup */
  tasks: Record<string, Task>;
  /** Currently selected task ID, or null if none */
  selectedTaskId: string | null;
}

// ============================================================================
// Actions Interface
// ============================================================================

interface TaskActions {
  /** Replace all tasks with new array (converts to Record) */
  setTasks: (tasks: Task[]) => void;
  /** Update a specific task with partial changes */
  updateTask: (taskId: string, changes: Partial<Task>) => void;
  /** Select a task by ID, or null to deselect */
  selectTask: (taskId: string | null) => void;
  /** Add a single task to the store */
  addTask: (task: Task) => void;
  /** Remove a task from the store */
  removeTask: (taskId: string) => void;
}

// ============================================================================
// Store Implementation
// ============================================================================

export const useTaskStore = create<TaskState & TaskActions>()(
  immer((set) => ({
    // Initial state
    tasks: {},
    selectedTaskId: null,

    // Actions
    setTasks: (tasks) =>
      set((state) => {
        state.tasks = Object.fromEntries(tasks.map((t) => [t.id, t]));
      }),

    updateTask: (taskId, changes) =>
      set((state) => {
        const task = state.tasks[taskId];
        if (task) {
          Object.assign(task, changes);
        }
      }),

    selectTask: (taskId) =>
      set((state) => {
        state.selectedTaskId = taskId;
      }),

    addTask: (task) =>
      set((state) => {
        state.tasks[task.id] = task;
      }),

    removeTask: (taskId) =>
      set((state) => {
        delete state.tasks[taskId];
        // Clear selection if removing selected task
        if (state.selectedTaskId === taskId) {
          state.selectedTaskId = null;
        }
      }),
  }))
);

// ============================================================================
// Selectors (defined outside store for memoization)
// ============================================================================

/**
 * Select all tasks with a specific status
 * @param status - The internal status to filter by
 * @returns Selector function returning matching tasks
 */
export const selectTasksByStatus =
  (status: InternalStatus) =>
  (state: TaskState): Task[] =>
    Object.values(state.tasks).filter((t) => t.internalStatus === status);

/**
 * Select the currently selected task
 * @returns The selected task, or null if none selected
 */
export const selectSelectedTask = (state: TaskState & TaskActions): Task | null =>
  state.selectedTaskId ? state.tasks[state.selectedTaskId] ?? null : null;
