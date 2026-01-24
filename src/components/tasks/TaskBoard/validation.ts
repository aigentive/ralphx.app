/**
 * Drag-drop validation logic for the kanban board
 */

import type { Task } from "@/types/task";

export interface ValidationResult {
  valid: boolean;
  error?: string;
}

/** Columns that cannot have tasks dragged out */
const LOCKED_SOURCE_COLUMNS = ["in_progress", "in_review"];

/** Columns that cannot receive dropped tasks */
const LOCKED_TARGET_COLUMNS = ["done", "in_progress", "in_review"];

/**
 * Validate if a task can be dropped from source to target column
 *
 * @param task - The task being dragged
 * @param sourceColumnId - The column the task is being dragged from
 * @param targetColumnId - The column the task is being dropped on
 * @returns Validation result with error message if invalid
 */
export function validateDrop(
  task: Task,
  sourceColumnId: string,
  targetColumnId: string
): ValidationResult {
  // Block reordering within Done column
  if (sourceColumnId === "done" && targetColumnId === "done") {
    return { valid: false, error: "Cannot reorder tasks in Done column" };
  }

  // Block drag out of locked source columns
  if (LOCKED_SOURCE_COLUMNS.includes(sourceColumnId) && sourceColumnId !== targetColumnId) {
    return { valid: false, error: `Tasks in ${sourceColumnId.replace("_", " ")} cannot be moved. Use Stop action to cancel.` };
  }

  // Block drag to Done column
  if (targetColumnId === "done") {
    return { valid: false, error: "Cannot manually complete tasks. Tasks move to Done automatically." };
  }

  // Block drag to system-managed columns
  if (["in_progress", "in_review"].includes(targetColumnId) && sourceColumnId !== targetColumnId) {
    return { valid: false, error: `${targetColumnId.replace("_", " ")} is system-managed` };
  }

  // Validate requirements for Planned column
  if (targetColumnId === "planned") {
    if (!task.title || task.title.trim() === "") {
      return { valid: false, error: "Task must have a title to be planned" };
    }
    if (!task.description || task.description.trim() === "") {
      return { valid: false, error: "Task must have a description to be planned" };
    }
  }

  return { valid: true };
}

/**
 * Check if a column is locked (cannot receive drops)
 */
export function isLockedColumn(columnId: string): boolean {
  return LOCKED_TARGET_COLUMNS.includes(columnId);
}

/**
 * Check if a task can be dragged from its current column
 */
export function canDragFromColumn(columnId: string): boolean {
  return !LOCKED_SOURCE_COLUMNS.includes(columnId);
}
