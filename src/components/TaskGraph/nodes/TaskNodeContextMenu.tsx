/**
 * TaskNodeContextMenu - Right-click context menu for task graph nodes
 *
 * Wraps children in a ContextMenu and delegates all menu item rendering
 * to the shared TaskContextMenuItems component with context="graph".
 *
 * Per spec: Phase E.1 of Task Graph View implementation
 */

import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import type { Task } from "@/types/task";
import type { TaskContextMenuHandlers } from "@/components/tasks/TaskContextMenuItems";
import {
  TaskContextMenuItems,
  TaskContextMenuDialogs,
  TaskContextMenuProvider,
  useTaskContextMenu,
} from "@/components/tasks/TaskContextMenuItems";

// ============================================================================
// Types
// ============================================================================

export interface TaskNodeContextMenuProps {
  task: Task;
  children: React.ReactNode;
  handlers: TaskContextMenuHandlers;
}

// ============================================================================
// Component
// ============================================================================

export function TaskNodeContextMenu({
  task,
  children,
  handlers,
}: TaskNodeContextMenuProps) {
  const menuState = useTaskContextMenu();

  return (
    <TaskContextMenuProvider state={menuState}>
      <ContextMenu>
        <ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
        <ContextMenuContent data-testid="task-node-context-menu">
          <TaskContextMenuItems task={task} handlers={handlers} context="graph" />
        </ContextMenuContent>
        <TaskContextMenuDialogs task={task} handlers={handlers} />
      </ContextMenu>
    </TaskContextMenuProvider>
  );
}
