/**
 * TaskCardContextMenu - Right-click context menu for task cards (Kanban)
 *
 * Keeps the ContextMenu/ContextMenuTrigger wrapper (Kanban-specific)
 * and delegates all menu item rendering to the shared TaskContextMenuItems.
 */

import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import type { Task } from "@/types/task";
import { TaskContextMenuItems, type TaskContextMenuItemsHandlers } from "./TaskContextMenuItems";

interface TaskCardContextMenuProps extends TaskContextMenuItemsHandlers {
  task: Task;
  children: React.ReactNode;
}

export function TaskCardContextMenu({
  task,
  children,
  ...handlers
}: TaskCardContextMenuProps) {
  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
      <ContextMenuContent>
        <TaskContextMenuItems task={task} handlers={handlers} context="kanban" />
      </ContextMenuContent>
    </ContextMenu>
  );
}
