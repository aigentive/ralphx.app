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
  TaskContextMenuProvider,
  useTaskContextMenu,
  type TaskContextMenuHandlers,
} from "./TaskContextMenuItems";

interface TaskCardContextMenuProps extends TaskContextMenuHandlers {
  task: Task;
  children: React.ReactNode;
}

export function TaskCardContextMenu({
  task,
  children,
  ...handlers
}: TaskCardContextMenuProps) {
  const menuState = useTaskContextMenu();

  return (
    <TaskContextMenuProvider state={menuState}>
      <ContextMenu>
        <ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
        <ContextMenuContent>
          <TaskContextMenuItems task={task} handlers={handlers} context="kanban" />
        </ContextMenuContent>
        <TaskContextMenuDialogs task={task} handlers={handlers} />
      </ContextMenu>
    </TaskContextMenuProvider>
  );
}
