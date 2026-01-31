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

import { useState } from "react";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import { Eye, Pencil, Archive, RotateCcw, Trash, X, Ban, Unlock, Lightbulb } from "lucide-react";
import type { Task } from "@/types/task";
import { useConfirmation } from "@/hooks/useConfirmation";
import { BlockReasonDialog } from "./BlockReasonDialog";

interface TaskCardContextMenuProps {
  task: Task;
  children: React.ReactNode;
  onViewDetails: () => void;
  onEdit: () => void;
  onArchive: () => void;
  onRestore: () => void;
  onPermanentDelete: () => void;
  onStatusChange: (newStatus: string) => void;
  /** Handler for blocking a task with an optional reason */
  onBlockWithReason: (reason?: string) => void;
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
  onBlockWithReason,
  onStartIdeation,
}: TaskCardContextMenuProps) {
  const { confirm, confirmationDialogProps, ConfirmationDialog } = useConfirmation();
  const [showBlockDialog, setShowBlockDialog] = useState(false);

  const isArchived = task.archivedAt !== null;
  const canEditTask = canEdit(task);
  const statusActions = getStatusActions(task.internalStatus);
  // "Backlog" is the equivalent of "draft" - tasks that haven't started execution yet
  const isBacklog = task.internalStatus === "backlog";

  // Confirmation message mappings for status actions
  const statusConfirmationMessages: Record<string, { title: string; description: string; variant: "default" | "destructive" }> = {
    cancelled: { title: "Cancel this task?", description: "The task will be marked as cancelled.", variant: "destructive" },
    blocked: { title: "Block this task?", description: "The task will be marked as blocked.", variant: "default" },
    ready: { title: "Unblock this task?", description: "The task will be moved back to ready.", variant: "default" },
    backlog: { title: "Re-open this task?", description: "The task will be moved to backlog.", variant: "default" },
  };

  const handleArchive = async () => {
    const confirmed = await confirm({
      title: "Archive this task?",
      description: "The task will be moved to the archive.",
      confirmText: "Archive",
      variant: "default",
    });
    if (confirmed) onArchive();
  };

  const handleRestore = async () => {
    const confirmed = await confirm({
      title: "Restore this task?",
      description: "The task will be restored to the backlog.",
      confirmText: "Restore",
      variant: "default",
    });
    if (confirmed) onRestore();
  };

  const handlePermanentDelete = async () => {
    const confirmed = await confirm({
      title: "Delete permanently?",
      description: "This will permanently delete the task. This action cannot be undone.",
      confirmText: "Delete",
      variant: "destructive",
    });
    if (confirmed) onPermanentDelete();
  };

  const handleStatusChange = async (newStatus: string, label: string) => {
    // Handle "Retry" action specifically (goes to backlog but with different messaging)
    const isRetry = label === "Retry";
    const messages = isRetry
      ? { title: "Retry this task?", description: "The task will be queued for re-execution.", variant: "default" as const }
      : statusConfirmationMessages[newStatus] ?? { title: `Change status to ${label}?`, description: `The task will be moved to ${label}.`, variant: "default" as const };

    const confirmed = await confirm({
      title: messages.title,
      description: messages.description,
      confirmText: label,
      variant: messages.variant,
    });
    if (confirmed) onStatusChange(newStatus);
  };

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
            <ContextMenuItem onClick={handleArchive}>
              <Archive className="w-4 h-4 mr-2" />
              Archive
            </ContextMenuItem>

            {/* Status actions (Cancel, Block, Unblock, etc.) */}
            {statusActions.map((action) => (
              <ContextMenuItem
                key={action.status}
                onClick={() => {
                  // Block action opens dialog instead of confirm
                  if (action.label === "Block") {
                    setShowBlockDialog(true);
                  } else {
                    handleStatusChange(action.status, action.label);
                  }
                }}
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
            <ContextMenuItem onClick={handleRestore}>
              <RotateCcw className="w-4 h-4 mr-2" />
              Restore
            </ContextMenuItem>

            {/* Permanent Delete */}
            <ContextMenuItem onClick={handlePermanentDelete} className="text-destructive">
              <Trash className="w-4 h-4 mr-2" />
              Delete Permanently
            </ContextMenuItem>
          </>
        )}
      </ContextMenuContent>
      <ConfirmationDialog {...confirmationDialogProps} />
      <BlockReasonDialog
        isOpen={showBlockDialog}
        onClose={() => setShowBlockDialog(false)}
        onConfirm={(reason) => {
          onBlockWithReason(reason);
          setShowBlockDialog(false);
        }}
        taskTitle={task.title}
      />
    </ContextMenu>
  );
}
