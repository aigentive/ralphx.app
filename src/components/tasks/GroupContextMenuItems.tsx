/**
 * GroupContextMenuItems — Renders group-level bulk actions as ContextMenuItems.
 *
 * Reusable across Kanban columns, Graph plan groups, and Graph uncategorized
 * containers. Renders ContextMenuItem elements only (no wrapper).
 *
 * Usage:
 * ```tsx
 * const { confirm, confirmationDialogProps, ConfirmationDialog } = useConfirmation();
 *
 * <ContextMenu>
 *   <ContextMenuTrigger>{children}</ContextMenuTrigger>
 *   <ContextMenuContent>
 *     <GroupContextMenuItems
 *       groupLabel="Ready"
 *       groupKind="column"
 *       taskCount={5}
 *       projectId="proj-123"
 *       groupId="ready"
 *       onRemoveAll={handleRemoveAll}
 *       confirm={confirm}
 *     />
 *   </ContextMenuContent>
 *   <ConfirmationDialog {...confirmationDialogProps} />
 * </ContextMenu>
 * ```
 */

import { useCallback } from "react";
import { ContextMenuItem } from "@/components/ui/context-menu";
import {
  type GroupKind,
  GROUP_ACTIONS,
  getRemoveAllLabel,
  getCancelAllLabel,
} from "@/lib/task-actions";

export interface GroupContextMenuItemsProps {
  /** Display label for the group (e.g. "Ready", plan name, "Uncategorized") */
  groupLabel: string;
  /** What kind of group: column (status), plan, or uncategorized */
  groupKind: GroupKind;
  /** Number of tasks in this group */
  taskCount: number;
  /** Project ID for cleanup API */
  projectId: string;
  /** Group identifier: status name or session ID */
  groupId: string;
  /** Handler called after user confirms removal */
  onRemoveAll: () => void;
  /** Optional handler called after user confirms cancellation */
  onCancelAll?: () => void;
  /** Confirm function from useConfirmation hook */
  confirm: (opts: {
    title: string;
    description: string;
    confirmText?: string;
    variant?: "default" | "destructive";
  }) => Promise<boolean>;
}

export function GroupContextMenuItems({
  groupLabel,
  groupKind,
  taskCount,
  onRemoveAll,
  onCancelAll,
  confirm,
}: GroupContextMenuItemsProps) {
  const removeAction = GROUP_ACTIONS.removeAll;
  const cancelAction = GROUP_ACTIONS.cancelAll;
  const removeLabel = getRemoveAllLabel(groupKind, groupLabel);
  const cancelLabel = getCancelAllLabel(groupKind, groupLabel);

  const handleRemoveAll = useCallback(async () => {
    if (taskCount === 0) return;

    const config = removeAction.confirmConfig(groupLabel, taskCount);
    const confirmed = await confirm({
      title: config.title,
      description: config.description,
      confirmText: "Remove",
      variant: config.variant,
    });

    if (confirmed) {
      onRemoveAll();
    }
  }, [taskCount, removeAction, groupLabel, confirm, onRemoveAll]);

  const handleCancelAll = useCallback(async () => {
    if (taskCount === 0 || !onCancelAll) return;

    const config = cancelAction.confirmConfig(groupLabel, taskCount);
    const confirmed = await confirm({
      title: config.title,
      description: config.description,
      confirmText: "Cancel",
      variant: config.variant,
    });

    if (confirmed) {
      onCancelAll();
    }
  }, [taskCount, cancelAction, groupLabel, confirm, onCancelAll]);

  if (taskCount === 0) return null;

  const RemoveIcon = removeAction.icon;
  const CancelIcon = cancelAction.icon;

  return (
    <>
      <ContextMenuItem
        onClick={handleRemoveAll}
        className="text-destructive"
        data-testid="remove-all-action"
      >
        <RemoveIcon className="w-4 h-4 mr-2" />
        {removeLabel}
      </ContextMenuItem>
      {onCancelAll && (
        <ContextMenuItem
          onClick={handleCancelAll}
          className="text-destructive"
          data-testid="cancel-all-action"
        >
          <CancelIcon className="w-4 h-4 mr-2" />
          {cancelLabel}
        </ContextMenuItem>
      )}
    </>
  );
}
