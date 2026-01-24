/**
 * Priority reordering logic for the kanban board
 */

import type { Task } from "@/types/task";

/**
 * Calculate the new priority for a task at a given position
 *
 * @param tasks - Current tasks in the column (sorted by priority)
 * @param newIndex - The target index for the task
 * @returns The new priority value
 */
export function calculateNewPriority(tasks: Task[], newIndex: number): number {
  if (tasks.length === 0) return 0;
  if (newIndex === 0) return 0;
  if (newIndex >= tasks.length) return tasks.length;

  // Priority between previous and current item at that position
  const prevPriority = tasks[newIndex - 1]?.priority ?? 0;
  const nextPriority = tasks[newIndex]?.priority ?? prevPriority + 2;

  return Math.floor((prevPriority + nextPriority) / 2);
}

/**
 * Reorder tasks array and update priorities
 *
 * @param tasks - Current tasks array
 * @param fromIndex - Index of the task being moved
 * @param toIndex - Target index
 * @returns New array with updated priorities
 */
export function reorderTasks(tasks: Task[], fromIndex: number, toIndex: number): Task[] {
  if (fromIndex === toIndex) return tasks;

  const result = [...tasks];
  const [moved] = result.splice(fromIndex, 1);
  if (!moved) return tasks;

  result.splice(toIndex, 0, moved);

  // Update priorities to match new positions
  return result.map((task, index) => ({
    ...task,
    priority: index,
  }));
}
