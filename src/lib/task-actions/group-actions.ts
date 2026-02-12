/**
 * Group-level action definitions for bulk operations on task groups.
 * Used by GroupContextMenuItems across Kanban columns, Graph plan groups,
 * and Graph uncategorized containers.
 */

import { Trash2, Ban } from "lucide-react";
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
  onCancelAll?: () => void;
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
export const GROUP_ACTIONS: { removeAll: GroupAction; cancelAll: GroupAction } = {
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
  cancelAll: {
    id: "cancelAll",
    label: "Cancel all",
    icon: Ban,
    variant: "destructive",
    confirmConfig: (groupLabel: string, taskCount: number) => ({
      title: `Cancel all ${groupLabel}?`,
      description: `This will cancel ${taskCount} task${taskCount === 1 ? "" : "s"}.`,
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
 * Get the display label for a "Cancel all" action given the group context.
 * Produces explicit labels like "Cancel all Ready", "Cancel all in Plan Name",
 * "Cancel all Uncategorized".
 */
export function getCancelAllLabel(groupKind: GroupKind, groupLabel: string): string {
  switch (groupKind) {
    case "column":
      return `Cancel all ${groupLabel}`;
    case "plan":
      return `Cancel all in ${groupLabel}`;
    case "uncategorized":
      return "Cancel all Uncategorized";
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
