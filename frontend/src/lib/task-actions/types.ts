/**
 * Shared types for task action definitions used by both
 * Kanban (TaskCardContextMenu) and Graph (TaskNodeContextMenu) context menus.
 */

import type { InternalStatus } from "@/types/status";

/** Confirmation dialog configuration for an action */
export interface ConfirmConfig {
  title: string;
  description: string;
  variant: "default" | "destructive";
}

/**
 * A task action definition — data-only, no rendering logic.
 * Both context menus consume these to render appropriate menu items.
 */
export interface TaskAction {
  /** Unique identifier for the action (e.g. "cancel", "block", "start") */
  id: string;
  /** Display label (e.g. "Cancel", "Block", "Start Execution") */
  label: string;
  /** Lucide icon component */
  icon: React.ComponentType<{ className?: string }>;
  /** Which handler to invoke */
  handlerKey: string;
  /** Visual variant */
  variant?: "default" | "destructive";
  /** Confirmation dialog config. If absent, action executes immediately. */
  confirmConfig?: ConfirmConfig;
  /**
   * If true, the action is a "view" action (no mutation, no confirmation needed).
   * View actions are dispatched immediately without confirmation.
   */
  isViewAction?: boolean;
  /**
   * If true, the action opens a special dialog (e.g. BlockReasonDialog)
   * instead of using standard confirmation.
   */
  opensDialog?: boolean;
}

/**
 * Surface context — which UI surface is requesting actions.
 * Determines which subset of actions are returned.
 */
export type ActionSurface = "kanban" | "graph";

/**
 * Map of handler keys to handler functions.
 * Context menu components provide these when rendering actions.
 */
export type ActionHandlers = Record<string, (() => void) | undefined>;

/** A status-to-actions mapping entry */
export type StatusActionsMap = Partial<Record<InternalStatus, TaskAction[]>>;
