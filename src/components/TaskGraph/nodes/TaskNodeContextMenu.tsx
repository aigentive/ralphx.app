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
  ContextMenuSeparator,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import type { Task } from "@/types/task";
import type { GroupInfo } from "@/lib/task-actions";
import type { TaskContextMenuHandlers } from "@/components/tasks/TaskContextMenuItems";
import {
  TaskContextMenuItems,
  TaskContextMenuDialogs,
  TaskContextMenuProvider,
  useTaskContextMenu,
} from "@/components/tasks/TaskContextMenuItems";
import { GroupContextMenuItems } from "@/components/tasks/GroupContextMenuItems";

// ============================================================================
// Types
// ============================================================================

export interface TaskNodeContextMenuProps {
  task: Task;
  children: React.ReactNode;
  handlers: TaskContextMenuHandlers;
  groupInfo?: GroupInfo;
}

// ============================================================================
// Component
// ============================================================================

export function TaskNodeContextMenu({
  task,
  children,
  handlers,
  groupInfo,
}: TaskNodeContextMenuProps) {
  const menuState = useTaskContextMenu();

  return (
    <TaskContextMenuProvider state={menuState}>
      <ContextMenu>
        <ContextMenuTrigger asChild>{children}</ContextMenuTrigger>
        <ContextMenuContent data-testid="task-node-context-menu">
          <TaskContextMenuItems task={task} handlers={handlers} context="graph" />
          {groupInfo && (
            <>
              <ContextMenuSeparator />
              <GroupContextMenuItems
                groupLabel={groupInfo.groupLabel}
                groupKind={groupInfo.groupKind}
                taskCount={groupInfo.taskCount}
                projectId={groupInfo.projectId}
                groupId={groupInfo.groupId}
                onRemoveAll={groupInfo.onRemoveAll}
                confirm={menuState.confirm}
              />
            </>
          )}
        </ContextMenuContent>
        <TaskContextMenuDialogs task={task} handlers={handlers} />
      </ContextMenu>
    </TaskContextMenuProvider>
  );
}
