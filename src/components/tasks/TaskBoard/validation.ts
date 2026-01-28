/**
 * Drag-drop validation logic for the kanban board
 *
 * Supports both column-level and group-level validation for multi-state columns.
 * See specs/plans/review_system.md for the group locking rules.
 */

import type { Task } from "@/types/task";
import type { StateGroup } from "@/types/workflow";
import type { InternalStatus } from "@/types/status";

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

/**
 * Find the group a task belongs to based on its internal status
 *
 * @param taskStatus - The task's internal status
 * @param groups - The groups defined for the column
 * @returns The matching group or undefined if no match
 */
export function findGroupForTask(
  taskStatus: InternalStatus,
  groups: StateGroup[] | undefined
): StateGroup | undefined {
  if (!groups || groups.length === 0) return undefined;
  return groups.find((g) => g.statuses.includes(taskStatus));
}

/**
 * Check if a task can be dragged from its group
 *
 * Group-level validation checks the canDragFrom property of the group.
 * This allows fine-grained control within multi-state columns.
 *
 * @param task - The task being dragged
 * @param groups - The groups defined for the source column
 * @returns true if the task can be dragged from its group
 */
export function canDragFromGroup(
  task: Task,
  groups: StateGroup[] | undefined
): boolean {
  if (!groups || groups.length === 0) {
    // No groups defined, use default column-level validation
    return true;
  }

  const group = findGroupForTask(task.internalStatus as InternalStatus, groups);
  if (!group) {
    // Task doesn't match any group, allow drag
    return true;
  }

  // Check group's canDragFrom (default to true if not specified)
  return group.canDragFrom !== false;
}

/**
 * Check if a task can be dropped to a target group
 *
 * Group-level validation checks the canDropTo property of the target group.
 *
 * @param targetStatus - The target status (if dropping to a specific group)
 * @param groups - The groups defined for the target column
 * @returns true if tasks can be dropped to the target group
 */
export function canDropToGroup(
  targetStatus: InternalStatus | undefined,
  groups: StateGroup[] | undefined
): boolean {
  if (!groups || groups.length === 0) {
    // No groups defined, use default column-level validation
    return true;
  }

  if (!targetStatus) {
    // No specific target status, check if any group allows drops
    return groups.some((g) => g.canDropTo !== false);
  }

  const group = findGroupForTask(targetStatus, groups);
  if (!group) {
    // No matching group, allow drop
    return true;
  }

  // Check group's canDropTo (default to true if not specified)
  return group.canDropTo !== false;
}

/**
 * Enhanced validation that includes group-level checks
 *
 * @param task - The task being dragged
 * @param sourceColumnId - The column the task is being dragged from
 * @param targetColumnId - The column the task is being dropped on
 * @param sourceGroups - Groups defined for the source column
 * @param targetGroups - Groups defined for the target column
 * @returns Validation result with error message if invalid
 */
export function validateDropWithGroups(
  task: Task,
  sourceColumnId: string,
  targetColumnId: string,
  sourceGroups: StateGroup[] | undefined,
  targetGroups: StateGroup[] | undefined
): ValidationResult {
  // First check column-level validation
  const columnResult = validateDrop(task, sourceColumnId, targetColumnId);
  if (!columnResult.valid) {
    return columnResult;
  }

  // Check source group restrictions
  if (!canDragFromGroup(task, sourceGroups)) {
    const group = findGroupForTask(task.internalStatus as InternalStatus, sourceGroups);
    const groupLabel = group?.label || "this group";
    return {
      valid: false,
      error: `Tasks in ${groupLabel} cannot be dragged. This state is system-managed.`,
    };
  }

  // Check target group restrictions (if moving to different column)
  if (sourceColumnId !== targetColumnId && !canDropToGroup(undefined, targetGroups)) {
    return {
      valid: false,
      error: `Cannot drop tasks to this column. All groups are system-managed.`,
    };
  }

  return { valid: true };
}
