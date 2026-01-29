/**
 * TaskCardContextMenu - Right-click context menu for task cards
 *
 * Provides contextual actions based on task status and state:
 * - View Details (always available)
 * - Edit (if not archived, not system-controlled)
 * - Archive/Restore
 * - Status transitions (Cancel, Block, Unblock, etc.)
 * - Permanent Delete (only for archived tasks)
 */

import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { Eye, Pencil, Archive, RotateCcw, Trash, X, Ban, Unlock, Lightbulb } from "lucide-react";
import type { Task } from "@/types/task";

interface TaskCardContextMenuProps {
  task: Task;
  children: React.ReactNode;
  onViewDetails: () => void;
  onEdit: () => void;
  onArchive: () => void;
  onRestore: () => void;
  onPermanentDelete: () => void;
  onStatusChange: (newStatus: string) => void;
  /** Handler for starting ideation seeded from this task (only for backlog tasks) */
  onStartIdeation?: () => void;
}

/**
 * System-controlled statuses that cannot be edited manually
 */
const SYSTEM_CONTROLLED_STATUSES = [
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
];

/**
 * Determine if a task can be edited
 */
function canEdit(task: Task): boolean {
  return (
    !task.archivedAt &&
    !SYSTEM_CONTROLLED_STATUSES.includes(task.internalStatus)
  );
}

/**
 * Get available status actions based on current status
 */
function getStatusActions(status: string): Array<{ label: string; status: string; icon: React.ComponentType<{ className?: string }> }> {
  const actions: Array<{ label: string; status: string; icon: React.ComponentType<{ className?: string }> }> = [];

  switch (status) {
    case "backlog":
      actions.push({ label: "Cancel", status: "cancelled", icon: X });
      break;
    case "ready":
      actions.push(
        { label: "Block", status: "blocked", icon: Ban },
        { label: "Cancel", status: "cancelled", icon: X }
      );
      break;
    case "blocked":
      actions.push(
        { label: "Unblock", status: "ready", icon: Unlock },
        { label: "Cancel", status: "cancelled", icon: X }
      );
      break;
    case "approved":
      actions.push({ label: "Re-open", status: "backlog", icon: RotateCcw });
      break;
    case "failed":
      actions.push({ label: "Retry", status: "backlog", icon: RotateCcw });
      break;
    case "cancelled":
      actions.push({ label: "Re-open", status: "backlog", icon: RotateCcw });
      break;
  }

  return actions;
}

export function TaskCardContextMenu({
  task,
  children,
  onViewDetails,
  onEdit,
  onArchive,
  onRestore,
  onPermanentDelete,
  onStatusChange,
  onStartIdeation,
}: TaskCardContextMenuProps) {
  const isArchived = task.archivedAt !== null;
  const canEditTask = canEdit(task);
  const statusActions = getStatusActions(task.internalStatus);
  // "Backlog" is the equivalent of "draft" - tasks that haven't started execution yet
  const isBacklog = task.internalStatus === "backlog";

  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
      <ContextMenuContent>
        {/* Always show View Details */}
        <ContextMenuItem onClick={onViewDetails}>
          <Eye className="w-4 h-4 mr-2" />
          View Details
        </ContextMenuItem>

        {/* Edit - only for non-archived, non-system-controlled tasks */}
        {canEditTask && (
          <ContextMenuItem onClick={onEdit}>
            <Pencil className="w-4 h-4 mr-2" />
            Edit
          </ContextMenuItem>
        )}

        {/* Start Ideation - only for backlog tasks */}
        {isBacklog && onStartIdeation && (
          <ContextMenuItem onClick={onStartIdeation}>
            <Lightbulb className="w-4 h-4 mr-2" />
            Start Ideation
          </ContextMenuItem>
        )}

        <ContextMenuSeparator />

        {/* Non-archived actions */}
        {!isArchived && (
          <>
            {/* Archive */}
            <ContextMenuItem onClick={onArchive}>
              <Archive className="w-4 h-4 mr-2" />
              Archive
            </ContextMenuItem>

            {/* Status actions (Cancel, Block, Unblock, etc.) */}
            {statusActions.map((action) => (
              <ContextMenuItem
                key={action.status}
                onClick={() => onStatusChange(action.status)}
              >
                <action.icon className="w-4 h-4 mr-2" />
                {action.label}
              </ContextMenuItem>
            ))}
          </>
        )}

        {/* Archived actions */}
        {isArchived && (
          <>
            {/* Restore */}
            <ContextMenuItem onClick={onRestore}>
              <RotateCcw className="w-4 h-4 mr-2" />
              Restore
            </ContextMenuItem>

            {/* Permanent Delete */}
            <ContextMenuItem onClick={onPermanentDelete} className="text-destructive">
              <Trash className="w-4 h-4 mr-2" />
              Delete Permanently
            </ContextMenuItem>
          </>
        )}
      </ContextMenuContent>
    </ContextMenu>
  );
}
