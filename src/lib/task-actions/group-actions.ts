/**
 * Group-level action definitions for bulk operations on task groups.
 * Used by GroupContextMenuItems across Kanban columns, Graph plan groups,
 * and Graph uncategorized containers.
 */

import { Ban, PauseCircle, Play, Archive } from "lucide-react";
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
  onCancelAll?: () => void;
  onPauseAll?: () => void;
  onResumeAll?: () => void;
  onArchiveAll?: () => void;
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
export const GROUP_ACTIONS: {
  cancelAll: GroupAction;
  pauseAll: GroupAction;
  resumeAll: GroupAction;
  archiveAll: GroupAction;
} = {
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
  pauseAll: {
    id: "pauseAll",
    label: "Pause all",
    icon: PauseCircle,
    confirmConfig: (groupLabel: string, taskCount: number) => ({
      title: `Pause all ${groupLabel}?`,
      description: `This will pause ${taskCount} task${taskCount === 1 ? "" : "s"}.`,
      variant: "default",
    }),
  },
  resumeAll: {
    id: "resumeAll",
    label: "Resume all",
    icon: Play,
    confirmConfig: (groupLabel: string, taskCount: number) => ({
      title: `Resume all ${groupLabel}?`,
      description: `This will resume ${taskCount} paused task${taskCount === 1 ? "" : "s"}.`,
      variant: "default",
    }),
  },
  archiveAll: {
    id: "archiveAll",
    label: "Archive all",
    icon: Archive,
    confirmConfig: (groupLabel: string, taskCount: number) => ({
      title: `Archive all ${groupLabel}?`,
      description: `This will archive ${taskCount} task${taskCount === 1 ? "" : "s"}.`,
      variant: "default",
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

export function getPauseAllLabel(groupKind: GroupKind, groupLabel: string): string {
  switch (groupKind) {
    case "column":
      return `Pause all ${groupLabel}`;
    case "plan":
      return `Pause all in ${groupLabel}`;
    case "uncategorized":
      return "Pause all Uncategorized";
  }
}

export function getResumeAllLabel(groupKind: GroupKind, groupLabel: string): string {
  switch (groupKind) {
    case "column":
      return `Resume all ${groupLabel}`;
    case "plan":
      return `Resume all in ${groupLabel}`;
    case "uncategorized":
      return "Resume all Uncategorized";
  }
}

export function getArchiveAllLabel(groupKind: GroupKind, groupLabel: string): string {
  switch (groupKind) {
    case "column":
      return `Archive all ${groupLabel}`;
    case "plan":
      return `Archive all in ${groupLabel}`;
    case "uncategorized":
      return "Archive all Uncategorized";
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
