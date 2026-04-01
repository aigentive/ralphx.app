/**
 * Quick Action types for the command palette
 *
 * Defines the extensible quick action system used in PlanQuickSwitcherPalette.
 * Actions follow a state machine flow: idle → confirming → creating → success.
 */

import type { LucideIcon } from "lucide-react";

/**
 * State of a quick action flow
 */
export type QuickActionFlowState = "idle" | "confirming" | "creating" | "success";

/**
 * Quick action interface
 *
 * Extensible action type for the command palette. Implementations can be
 * ideation sessions, task creation, search-by-id, etc.
 */
export interface QuickAction {
  /** Unique identifier (e.g., "ideation", "create-task") */
  id: string;
  /** Display label (e.g., "Start new ideation session") */
  label: string;
  /** Icon from lucide-react */
  icon: LucideIcon;
  /** Description generator based on query (e.g., `"${query}"`) */
  description: (query: string) => string;
  /** Whether this action should appear for the current query */
  isVisible: (query: string) => boolean;
  /** Execute the action. Returns entity ID on success. */
  execute: (query: string) => Promise<string>;
  /** Label shown during creation (e.g., "Creating your ideation session...") */
  creatingLabel: string;
  /** Label shown on success (e.g., "Session created!") */
  successLabel: string;
  /** Button text on success (e.g., "View Session") */
  viewLabel: string;
  /** Navigate to the created entity */
  navigateTo: (entityId: string) => void;
}
