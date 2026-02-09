/**
 * Shared constants for task actions — single source of truth for:
 * - System-controlled statuses
 * - canEdit() logic
 * - Confirmation message configs
 */

import type { InternalStatus } from "@/types/status";
import type { ConfirmConfig } from "./types";

/**
 * Statuses that are managed by the system (agents, state machine).
 * Tasks in these statuses cannot be manually edited.
 */
export const SYSTEM_CONTROLLED_STATUSES: readonly InternalStatus[] = [
  "executing",
  "qa_refining",
  "qa_testing",
  "qa_passed",
  "qa_failed",
  "pending_review",
  "revision_needed",
  "reviewing",
  "review_passed",
  "re_executing",
] as const;

/**
 * Determine if a task can be edited based on its status and archived state.
 */
export function canEdit(task: { archivedAt: string | null; internalStatus: string }): boolean {
  return (
    !task.archivedAt &&
    !(SYSTEM_CONTROLLED_STATUSES as readonly string[]).includes(task.internalStatus)
  );
}

/**
 * Confirmation configs keyed by action ID.
 * Merged superset of both Kanban and Graph confirmation messages.
 */
export const CONFIRMATION_CONFIGS = {
  // Kanban-origin status transitions
  cancelled: {
    title: "Cancel this task?",
    description: "The task will be marked as cancelled.",
    variant: "destructive",
  },
  blocked: {
    title: "Block this task?",
    description: "The task will be marked as blocked.",
    variant: "default",
  },
  ready: {
    title: "Unblock this task?",
    description: "The task will be moved back to ready.",
    variant: "default",
  },
  backlog: {
    title: "Re-open this task?",
    description: "The task will be moved to backlog.",
    variant: "default",
  },
  retry: {
    title: "Retry this task?",
    description: "The task will be queued for re-execution.",
    variant: "default",
  },

  // Graph-origin quick actions
  start: {
    title: "Start Execution?",
    description: "The task will be queued for execution by an AI worker.",
    variant: "default",
  },
  unblock: {
    title: "Unblock this task?",
    description: "The task will be moved back to ready and the blocked reason will be cleared.",
    variant: "default",
  },
  approve: {
    title: "Approve this task?",
    description: "The task will be marked as approved and completed.",
    variant: "default",
  },
  reject: {
    title: "Reject this task?",
    description: "The task will be marked as failed.",
    variant: "destructive",
  },
  "request-changes": {
    title: "Request changes?",
    description: "The task will be sent back for revision.",
    variant: "default",
  },
  "mark-resolved": {
    title: "Mark conflicts as resolved?",
    description: "The task will proceed with the merge.",
    variant: "default",
  },

  // Shared CRUD actions
  archive: {
    title: "Archive this task?",
    description: "The task will be moved to the archive.",
    variant: "default",
  },
  restore: {
    title: "Restore this task?",
    description: "The task will be restored to the backlog.",
    variant: "default",
  },
  "permanent-delete": {
    title: "Delete permanently?",
    description: "This will permanently delete the task. This action cannot be undone.",
    variant: "destructive",
  },
} as const satisfies Record<string, ConfirmConfig>;
