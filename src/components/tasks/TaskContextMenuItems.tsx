/**
 * TaskContextMenuItems - Shared context menu items for task actions
 *
 * Renders ContextMenuItems (no wrapper) to be used inside both
 * Kanban (TaskCardContextMenu) and Graph (TaskNodeContextMenu) context menus.
 */

import { useState } from "react";
import {
  ContextMenuItem,
  ContextMenuSeparator,
} from "@/components/ui/context-menu";
import { Eye, Pencil, Archive, RotateCcw, Trash, X, Ban, Unlock, Lightbulb } from "lucide-react";
import type { Task } from "@/types/task";
import { useConfirmation } from "@/hooks/useConfirmation";
import { BlockReasonDialog } from "./BlockReasonDialog";

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

function canEdit(task: Task): boolean {
  return (
    !task.archivedAt &&
    !SYSTEM_CONTROLLED_STATUSES.includes(task.internalStatus)
  );
}

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

const statusConfirmationMessages: Record<string, { title: string; description: string; variant: "default" | "destructive" }> = {
  cancelled: { title: "Cancel this task?", description: "The task will be marked as cancelled.", variant: "destructive" },
  blocked: { title: "Block this task?", description: "The task will be marked as blocked.", variant: "default" },
  ready: { title: "Unblock this task?", description: "The task will be moved back to ready.", variant: "default" },
  backlog: { title: "Re-open this task?", description: "The task will be moved to backlog.", variant: "default" },
};

export interface TaskContextMenuItemsHandlers {
  onViewDetails: () => void;
  onEdit: () => void;
  onArchive: () => void;
  onRestore: () => void;
  onPermanentDelete: () => void;
  onStatusChange: (newStatus: string) => void;
  onBlockWithReason: (reason?: string) => void;
  onUnblock: () => void;
  onStartIdeation?: () => void;
}

interface TaskContextMenuItemsProps {
  task: Task;
  handlers: TaskContextMenuItemsHandlers;
  context: "kanban" | "graph";
}

export function TaskContextMenuItems({ task, handlers, context: _context }: TaskContextMenuItemsProps) {
  const { confirm, confirmationDialogProps, ConfirmationDialog } = useConfirmation();
  const [showBlockDialog, setShowBlockDialog] = useState(false);

  const isArchived = task.archivedAt !== null;
  const canEditTask = canEdit(task);
  const statusActions = getStatusActions(task.internalStatus);
  const isBacklog = task.internalStatus === "backlog";

  const handleArchive = async () => {
    const confirmed = await confirm({
      title: "Archive this task?",
      description: "The task will be moved to the archive.",
      confirmText: "Archive",
      variant: "default",
    });
    if (confirmed) handlers.onArchive();
  };

  const handleRestore = async () => {
    const confirmed = await confirm({
      title: "Restore this task?",
      description: "The task will be restored to the backlog.",
      confirmText: "Restore",
      variant: "default",
    });
    if (confirmed) handlers.onRestore();
  };

  const handlePermanentDelete = async () => {
    const confirmed = await confirm({
      title: "Delete permanently?",
      description: "This will permanently delete the task. This action cannot be undone.",
      confirmText: "Delete",
      variant: "destructive",
    });
    if (confirmed) handlers.onPermanentDelete();
  };

  const handleStatusChange = async (newStatus: string, label: string) => {
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
    if (confirmed) handlers.onStatusChange(newStatus);
  };

  const handleUnblock = async () => {
    const confirmed = await confirm({
      title: "Unblock this task?",
      description: "The task will be moved back to ready and the blocked reason will be cleared.",
      confirmText: "Unblock",
      variant: "default",
    });
    if (confirmed) handlers.onUnblock();
  };

  return (
    <>
      {/* Always show View Details */}
      <ContextMenuItem onClick={handlers.onViewDetails}>
        <Eye className="w-4 h-4 mr-2" />
        View Details
      </ContextMenuItem>

      {/* Edit - only for non-archived, non-system-controlled tasks */}
      {canEditTask && (
        <ContextMenuItem onClick={handlers.onEdit}>
          <Pencil className="w-4 h-4 mr-2" />
          Edit
        </ContextMenuItem>
      )}

      {/* Start Ideation - only for backlog tasks */}
      {isBacklog && handlers.onStartIdeation && (
        <ContextMenuItem onClick={handlers.onStartIdeation}>
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
                if (action.label === "Block") {
                  setShowBlockDialog(true);
                } else if (action.label === "Unblock") {
                  handleUnblock();
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

      {/* Dialogs - rendered as siblings */}
      <ConfirmationDialog {...confirmationDialogProps} />
      <BlockReasonDialog
        isOpen={showBlockDialog}
        onClose={() => setShowBlockDialog(false)}
        onConfirm={(reason) => {
          handlers.onBlockWithReason(reason);
          setShowBlockDialog(false);
        }}
        taskTitle={task.title}
      />
    </>
  );
}
