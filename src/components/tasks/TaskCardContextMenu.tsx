/**
 * TaskCardContextMenu - Right-click context menu for task cards (Kanban)
 *
 * Keeps the ContextMenu/ContextMenuTrigger wrapper (Kanban-specific)
 * and delegates all menu item rendering to the shared TaskContextMenuItems.
 * Dialogs are rendered outside ContextMenuContent to survive menu close.
 */

import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import type { Task } from "@/types/task";
import {
  TaskContextMenuItems,
  TaskContextMenuDialogs,
  useTaskContextMenuActions,
  type TaskContextMenuItemsHandlers,
} from "./TaskContextMenuItems";

interface TaskCardContextMenuProps extends TaskContextMenuItemsHandlers {
  task: Task;
  children: React.ReactNode;
}

export function TaskCardContextMenu({
  task,
  children,
  ...handlers
}: TaskCardContextMenuProps) {
  const { menuHandlers, dialogProps } = useTaskContextMenuActions(handlers);

  return (
    <>
      <ContextMenu>
        <ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
        <ContextMenuContent>
          <TaskContextMenuItems task={task} handlers={handlers} menuHandlers={menuHandlers} />
        </ContextMenuContent>
      </ContextMenu>
      <TaskContextMenuDialogs dialogProps={dialogProps} onBlockWithReason={handlers.onBlockWithReason} />
    </>
  );
}
