/**
 * Group-level action definitions for bulk operations on task groups.
 * Used by GroupContextMenuItems across Kanban columns, Graph plan groups,
 * and Graph uncategorized containers.
 */

import { Trash2 } from "lucide-react";
import type { ConfirmConfig } from "./types";

/** Which kind of task group this is */
export type GroupKind = "column" | "plan" | "uncategorized";

/** Group context passed to task-level context menus for rendering group actions */
export interface GroupInfo {
  groupLabel: string;
  groupKind: GroupKind;
  taskCount: number;
  groupId: string;
  projectId: string;
  onRemoveAll: () => void;
}

/** A group-level action definition */
export interface GroupAction {
  id: string;
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  variant?: "default" | "destructive";
  confirmConfig: (groupLabel: string, taskCount: number) => ConfirmConfig;
}

/** Available group actions */
export const GROUP_ACTIONS: { removeAll: GroupAction } = {
  removeAll: {
    id: "removeAll",
    label: "Remove all",
    icon: Trash2,
    variant: "destructive",
    confirmConfig: (groupLabel: string, taskCount: number) => ({
      title: `Remove all ${groupLabel}?`,
      description: `This will permanently remove ${taskCount} task${taskCount === 1 ? "" : "s"}. This action cannot be undone.`,
      variant: "destructive",
    }),
  },
} as const;

/**
 * Get the display label for a "Remove all" action given the group context.
 * Produces explicit labels like "Remove all Ready", "Remove all from Plan Name",
 * "Remove all Uncategorized".
 */
export function getRemoveAllLabel(groupKind: GroupKind, groupLabel: string): string {
  switch (groupKind) {
    case "column":
      return `Remove all ${groupLabel}`;
    case "plan":
      return `Remove all from ${groupLabel}`;
    case "uncategorized":
      return "Remove all Uncategorized";
  }
}

/**
 * Resolve the cleanup API parameters for a given group.
 */
export function resolveGroupCleanupParams(
  groupKind: GroupKind,
  groupId: string,
): { groupKind: string; groupId: string } {
  switch (groupKind) {
    case "column":
      return { groupKind: "status", groupId };
    case "plan":
      return { groupKind: "session", groupId };
    case "uncategorized":
      return { groupKind: "uncategorized", groupId: "" };
  }
}
