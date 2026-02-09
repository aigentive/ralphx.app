/**
 * TaskContextMenuItems - Shared context menu items for task actions
 *
 * Renders ContextMenuItems (no wrapper) to be used inside both
 * Kanban (TaskCardContextMenu) and Graph (TaskNodeContextMenu) context menus.
 *
 * IMPORTANT: Dialogs must be rendered OUTSIDE ContextMenuContent because
 * Radix unmounts content on menu close. Use useTaskContextMenuActions() hook
 * to share state between items and dialogs, then render TaskContextMenuDialogs
 * as a sibling of <ContextMenu>.
 */

import { useState, useCallback } from "react";
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

export interface TaskContextMenuActionsReturn {
  menuHandlers: {
    handleArchive: () => Promise<void>;
    handleRestore: () => Promise<void>;
    handlePermanentDelete: () => Promise<void>;
    handleStatusChange: (newStatus: string, label: string) => Promise<void>;
    handleUnblock: () => Promise<void>;
    openBlockDialog: (taskTitle: string) => void;
  };
  dialogProps: {
    confirmationDialogProps: ReturnType<typeof useConfirmation>["confirmationDialogProps"];
    ConfirmationDialog: ReturnType<typeof useConfirmation>["ConfirmationDialog"];
    showBlockDialog: boolean;
    setShowBlockDialog: (show: boolean) => void;
    blockTaskTitle: string;
  };
}

/**
 * Hook that manages dialog state for task context menu actions.
 * Returns menu item handlers and dialog rendering props.
 *
 * Usage:
 * ```tsx
 * const { menuHandlers, dialogProps } = useTaskContextMenuActions(handlers);
 * // Inside ContextMenuContent:
 * <TaskContextMenuItems task={task} menuHandlers={menuHandlers} handlers={handlers} />
 * // Outside ContextMenuContent (sibling of ContextMenu):
 * <TaskContextMenuDialogs dialogProps={dialogProps} onBlockWithReason={handlers.onBlockWithReason} />
 * ```
 */
export function useTaskContextMenuActions(handlers: TaskContextMenuItemsHandlers): TaskContextMenuActionsReturn {
  const { confirm, confirmationDialogProps, ConfirmationDialog } = useConfirmation();
  const [showBlockDialog, setShowBlockDialog] = useState(false);
  const [blockTaskTitle, setBlockTaskTitle] = useState("");

  const handleArchive = useCallback(async () => {
    const confirmed = await confirm({
      title: "Archive this task?",
      description: "The task will be moved to the archive.",
      confirmText: "Archive",
      variant: "default",
    });
    if (confirmed) handlers.onArchive();
  }, [confirm, handlers]);

  const handleRestore = useCallback(async () => {
    const confirmed = await confirm({
      title: "Restore this task?",
      description: "The task will be restored to the backlog.",
      confirmText: "Restore",
      variant: "default",
    });
    if (confirmed) handlers.onRestore();
  }, [confirm, handlers]);

  const handlePermanentDelete = useCallback(async () => {
    const confirmed = await confirm({
      title: "Delete permanently?",
      description: "This will permanently delete the task. This action cannot be undone.",
      confirmText: "Delete",
      variant: "destructive",
    });
    if (confirmed) handlers.onPermanentDelete();
  }, [confirm, handlers]);

  const handleStatusChange = useCallback(async (newStatus: string, label: string) => {
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
  }, [confirm, handlers]);

  const handleUnblock = useCallback(async () => {
    const confirmed = await confirm({
      title: "Unblock this task?",
      description: "The task will be moved back to ready and the blocked reason will be cleared.",
      confirmText: "Unblock",
      variant: "default",
    });
    if (confirmed) handlers.onUnblock();
  }, [confirm, handlers]);

  const openBlockDialog = useCallback((taskTitle: string) => {
    setBlockTaskTitle(taskTitle);
    setShowBlockDialog(true);
  }, []);

  return {
    menuHandlers: {
      handleArchive,
      handleRestore,
      handlePermanentDelete,
      handleStatusChange,
      handleUnblock,
      openBlockDialog,
    },
    dialogProps: {
      confirmationDialogProps,
      ConfirmationDialog,
      showBlockDialog,
      setShowBlockDialog,
      blockTaskTitle,
    },
  };
}

/**
 * Renders the confirmation and block reason dialogs.
 * MUST be rendered OUTSIDE of ContextMenuContent (as sibling of ContextMenu)
 * because Radix unmounts ContextMenuContent on close.
 */
export function TaskContextMenuDialogs({
  dialogProps,
  onBlockWithReason,
}: {
  dialogProps: TaskContextMenuActionsReturn["dialogProps"];
  onBlockWithReason: (reason?: string) => void;
}) {
  const { confirmationDialogProps, ConfirmationDialog, showBlockDialog, setShowBlockDialog, blockTaskTitle } = dialogProps;

  return (
    <>
      <ConfirmationDialog {...confirmationDialogProps} />
      <BlockReasonDialog
        isOpen={showBlockDialog}
        onClose={() => setShowBlockDialog(false)}
        onConfirm={(reason) => {
          onBlockWithReason(reason);
          setShowBlockDialog(false);
        }}
        taskTitle={blockTaskTitle}
      />
    </>
  );
}

interface TaskContextMenuItemsProps {
  task: Task;
  handlers: TaskContextMenuItemsHandlers;
  menuHandlers: TaskContextMenuActionsReturn["menuHandlers"];
}

export function TaskContextMenuItems({ task, handlers, menuHandlers }: TaskContextMenuItemsProps) {
  const isArchived = task.archivedAt !== null;
  const canEditTask = canEdit(task);
  const statusActions = getStatusActions(task.internalStatus);
  const isBacklog = task.internalStatus === "backlog";

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
          <ContextMenuItem onClick={menuHandlers.handleArchive}>
            <Archive className="w-4 h-4 mr-2" />
            Archive
          </ContextMenuItem>

          {/* Status actions (Cancel, Block, Unblock, etc.) */}
          {statusActions.map((action) => (
            <ContextMenuItem
              key={action.status}
              onClick={() => {
                if (action.label === "Block") {
                  menuHandlers.openBlockDialog(task.title);
                } else if (action.label === "Unblock") {
                  menuHandlers.handleUnblock();
                } else {
                  menuHandlers.handleStatusChange(action.status, action.label);
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
          <ContextMenuItem onClick={menuHandlers.handleRestore}>
            <RotateCcw className="w-4 h-4 mr-2" />
            Restore
          </ContextMenuItem>

          {/* Permanent Delete */}
          <ContextMenuItem onClick={menuHandlers.handlePermanentDelete} className="text-destructive">
            <Trash className="w-4 h-4 mr-2" />
            Delete Permanently
          </ContextMenuItem>
        </>
      )}
    </>
  );
}
